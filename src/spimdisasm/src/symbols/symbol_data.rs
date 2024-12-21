/* SPDX-FileCopyrightText: © 2024 Decompollaborate */
/* SPDX-License-Identifier: MIT */

use alloc::vec::Vec;
use rabbitizer::Vram;

use crate::{
    address_range::AddressRange,
    config::Compiler,
    context::{Context, OwnedSegmentNotFoundError},
    metadata::{GeneratedBy, ParentSectionMetadata, SymbolMetadata, SymbolType},
    parent_segment_info::ParentSegmentInfo,
    relocation::{RelocReferencedSym, RelocationInfo, RelocationType},
    rom_address::RomAddress,
    rom_vram_range::RomVramRange,
    section_type::SectionType,
    size::Size,
    str_decoding::Encoding,
};

use super::{
    display::{SymDataDisplay, SymDataDisplaySettings, SymDisplayError},
    trait_symbol::RomSymbol,
    Symbol,
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
        rom: RomAddress,
        vram: Vram,
        _in_section_offset: usize,
        parent_segment_info: ParentSegmentInfo,
        section_type: SectionType,
        properties: SymbolDataProperties,
    ) -> Result<Self, OwnedSegmentNotFoundError> {
        let size = Size::new(raw_bytes.len() as u32);
        let rom_range = AddressRange::new(rom, rom + size);
        let vram_range = AddressRange::new(vram, vram + size);
        let ranges = RomVramRange::new(rom_range, vram_range);

        let mut relocs = vec![None; raw_bytes.len() / 4];

        let endian = context.global_config().endian();

        let owned_segment = context.find_owned_segment_mut(&parent_segment_info)?;
        let metadata = owned_segment.add_symbol(
            vram,
            Some(rom),
            GeneratedBy::Autogenerated,
            Some(section_type),
            false,
        );
        *metadata.autodetected_size_mut() = Some(size);
        metadata.set_defined();

        let encoding = properties.encoding;
        properties.apply_to_metadata(metadata);

        let sym_type = metadata.sym_type();

        let should_search_for_address = sym_type.is_none_or(|x| x.can_reference_symbols());
        let is_jtbl = sym_type == Some(&SymbolType::Jumptable);

        // TODO: improve heuristic to determine if should search for symbols
        if rom.inner() % 4 == 0 && should_search_for_address {
            for (i, word_bytes) in raw_bytes.chunks_exact(4).enumerate() {
                let word = endian.word_from_bytes(word_bytes);
                let word_vram = Vram::new(word);
                let offset = Size::new(i as u32);

                if owned_segment.in_vram_range(word_vram) {
                    let sym_metadata = if is_jtbl {
                        owned_segment.add_jumptable_label(
                            word_vram,
                            None,
                            GeneratedBy::Autogenerated,
                        )
                    } else {
                        owned_segment.add_symbol(
                            word_vram,
                            None,
                            GeneratedBy::Autogenerated,
                            None,
                            true,
                        )
                    };
                    sym_metadata.add_reference_symbol(
                        ranges.vram().start(),
                        parent_segment_info.clone(),
                        rom + offset,
                    );

                    relocs[i] = Some(
                        RelocationType::R_MIPS_32
                            .new_reloc_info(RelocReferencedSym::Address(word_vram)),
                    );
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
        SymDataDisplay::new(context, self, settings)
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
