/* SPDX-FileCopyrightText: © 2024-2025 Decompollaborate */
/* SPDX-License-Identifier: MIT */

use alloc::vec::Vec;

use crate::{
    addresses::{AddressRange, Rom, RomVramRange, Size, Vram},
    collections::addended_ordered_map::FindSettings,
    config::{Compiler, Endian},
    context::Context,
    metadata::{GeneratedBy, ParentSectionMetadata, SymbolMetadata, SymbolType},
    parent_segment_info::ParentSegmentInfo,
    relocation::{RelocReferencedSym, RelocationInfo, RelocationType},
    section_type::SectionType,
    str_decoding::Encoding,
};

use super::{
    display::{InternalSymDisplSettings, SymDataDisplay, SymDataDisplaySettings, SymDisplayError},
    trait_symbol::RomSymbol,
    Symbol, SymbolCreationError,
};

#[derive(Debug, Clone, Hash, PartialEq)]
pub struct SymbolData {
    ranges: RomVramRange,
    raw_bytes: Vec<u8>,
    parent_segment_info: ParentSegmentInfo,
    section_type: SectionType,
    relocs: Vec<Option<RelocationInfo>>,

    encoding: Encoding,
}

impl SymbolData {
    // TODO
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn new(
        context: &mut Context,
        raw_bytes: Vec<u8>,
        rom: Rom,
        vram: Vram,
        _in_section_offset: usize,
        parent_segment_info: ParentSegmentInfo,
        section_type: SectionType,
        properties: SymbolDataProperties,
    ) -> Result<Self, SymbolCreationError> {
        let size = Size::new(raw_bytes.len() as u32);
        let rom_range = AddressRange::new(rom, rom + size);
        let vram_range = AddressRange::new(vram, vram + size);
        let ranges = RomVramRange::new(rom_range, vram_range);

        let mut relocs = vec![None; raw_bytes.len() / 4];

        let endian = context.global_config().endian();

        let owned_segment = context.find_owned_segment_mut(&parent_segment_info)?;
        let metadata = owned_segment.add_symbol(vram, GeneratedBy::Autogenerated, false)?;
        *metadata.rom_mut() = Some(rom);
        *metadata.section_type_mut() = Some(section_type);
        *metadata.autodetected_size_mut() = Some(size);
        metadata.set_defined();
        metadata.set_trailing_padding_size(count_padding(
            &raw_bytes,
            metadata.user_declared_size(),
            metadata.sym_type(),
            endian,
            rom,
        ));

        let encoding = properties.encoding;
        properties.apply_to_metadata(metadata);

        let sym_type = metadata.sym_type();

        let should_search_for_address = sym_type.is_none_or(|x| x.can_reference_symbols());
        let is_jtbl = sym_type == Some(SymbolType::Jumptable);

        // TODO: improve heuristic to determine if should search for symbols
        if rom.inner() % 4 == 0 && should_search_for_address {
            for (i, word_bytes) in raw_bytes.chunks_exact(4).enumerate() {
                let word = endian.word_from_bytes(word_bytes);
                let word_vram = Vram::new(word);
                let offset = Size::new(i as u32);

                if owned_segment.in_vram_range(word_vram) {
                    let valid_reference = if is_jtbl {
                        let sym_metadata = owned_segment.add_symbol(
                            word_vram,
                            GeneratedBy::Autogenerated,
                            false,
                        )?;
                        sym_metadata.set_type_with_priorities(
                            SymbolType::JumptableLabel,
                            GeneratedBy::Autogenerated,
                        );
                        sym_metadata.add_reference_symbol(
                            ranges.vram().start(),
                            parent_segment_info.clone(),
                            rom + offset,
                        );
                        true
                    } else if let Some(sym_metadata) =
                        owned_segment.find_symbol(word_vram, FindSettings::default())
                    {
                        if sym_metadata.vram() == word_vram {
                            true
                        } else if let Some(sym_typ) = sym_metadata.sym_type() {
                            sym_typ.may_have_addend()
                        } else {
                            true
                        }
                    } else {
                        false
                    };

                    if valid_reference {
                        // TODO: move reloc generation to a later step
                        relocs[i] = Some(
                            RelocationType::R_MIPS_32
                                .new_reloc_info(RelocReferencedSym::Address(word_vram)),
                        );
                    }
                } else {
                    // TODO
                }
            }
        }

        Ok(Self {
            ranges,
            raw_bytes,
            parent_segment_info,
            section_type,
            relocs,

            encoding,
        })
    }
}

impl SymbolData {
    pub(crate) fn raw_bytes(&self) -> &[u8] {
        &self.raw_bytes
    }

    pub(crate) fn encoding(&self) -> Encoding {
        self.encoding
    }
}

impl<'ctx, 'sym, 'flg> SymbolData {
    pub fn display(
        &'sym self,
        context: &'ctx Context,
        settings: &'flg SymDataDisplaySettings,
    ) -> Result<SymDataDisplay<'ctx, 'sym, 'flg>, SymDisplayError> {
        self.display_internal(context, settings, InternalSymDisplSettings::new(false))
    }

    pub(crate) fn display_internal(
        &'sym self,
        context: &'ctx Context,
        settings: &'flg SymDataDisplaySettings,
        internal_settings: InternalSymDisplSettings,
    ) -> Result<SymDataDisplay<'ctx, 'sym, 'flg>, SymDisplayError> {
        SymDataDisplay::new(context, self, settings, internal_settings)
    }
}

impl Symbol for SymbolData {
    fn vram_range(&self) -> &AddressRange<Vram> {
        self.ranges.vram()
    }

    fn parent_segment_info(&self) -> &ParentSegmentInfo {
        &self.parent_segment_info
    }

    #[must_use]
    fn section_type(&self) -> SectionType {
        self.section_type
    }
}

impl RomSymbol for SymbolData {
    #[must_use]
    fn rom_vram_range(&self) -> &RomVramRange {
        &self.ranges
    }

    fn relocs(&self) -> &[Option<RelocationInfo>] {
        &self.relocs
    }
}

#[derive(Debug, Clone, Hash, PartialEq)]
pub(crate) struct SymbolDataProperties {
    pub parent_metadata: ParentSectionMetadata,
    pub compiler: Option<Compiler>,
    pub auto_pad_by: Option<Vram>,
    pub detected_type: Option<SymbolType>,
    pub encoding: Encoding,
}

impl SymbolDataProperties {
    fn apply_to_metadata(self, metadata: &mut SymbolMetadata) {
        metadata.set_parent_metadata(self.parent_metadata);

        if let Some(compiler) = self.compiler {
            metadata.set_compiler(compiler);
        }

        if let Some(auto_pad_by) = self.auto_pad_by {
            metadata.set_auto_created_pad_by(auto_pad_by);
        }

        if let Some(detected_type) = self.detected_type {
            metadata.set_type(detected_type, GeneratedBy::Autogenerated);
        }
    }
}

fn count_padding(
    raw_bytes: &[u8],
    user_declared_size: Option<Size>,
    typ: Option<SymbolType>,
    endian: Endian,
    rom: Rom,
) -> Size {
    if user_declared_size.is_some() {
        return Size::new(0);
    }

    let mut count: u32 = 0;

    match typ {
        Some(SymbolType::UserCustom) => {}
        Some(SymbolType::CString) => {
            for byte in raw_bytes.iter().rev() {
                if *byte != 0 {
                    break;
                }
                count += 1;
            }
            count = count.saturating_sub(1);
        }
        Some(SymbolType::Float64 | SymbolType::DWord) => {
            if raw_bytes.len() > 8 {
                for byte_group in raw_bytes[8..].chunks_exact(8).rev() {
                    let dword = endian.dword_from_bytes(byte_group);
                    if dword != 0 {
                        break;
                    }
                    count += 8;
                }
            }
        }
        Some(
            SymbolType::Float32
            | SymbolType::Word
            | SymbolType::Jumptable
            | SymbolType::GccExceptTable,
        ) => {
            if raw_bytes.len() > 4 {
                for byte_group in raw_bytes[4..].chunks_exact(4).rev() {
                    let word = endian.word_from_bytes(byte_group);
                    if word != 0 {
                        break;
                    }
                    count += 4;
                }
            }
        }
        // TODO: Should count padding for those bytes and shorts? And how?
        Some(SymbolType::Byte) => {}
        Some(SymbolType::Short) => {}
        Some(
            SymbolType::BranchLabel | SymbolType::JumptableLabel | SymbolType::GccExceptTableLabel,
        ) => {}
        Some(SymbolType::Function) => {}
        None => {
            // Treat it as word-sized if the alignement and size allow it.
            if raw_bytes.len() > 4 && raw_bytes.len() % 4 == 0 && rom.inner() % 4 == 0 {
                for byte_group in raw_bytes[4..].chunks_exact(4).rev() {
                    let word = endian.word_from_bytes(byte_group);
                    if word != 0 {
                        break;
                    }
                    count += 4;
                }
            }
        }
    }

    Size::new(count)
}
