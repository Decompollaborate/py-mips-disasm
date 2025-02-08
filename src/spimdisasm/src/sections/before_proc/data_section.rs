/* SPDX-FileCopyrightText: © 2024-2025 Decompollaborate */
/* SPDX-License-Identifier: MIT */

use alloc::{collections::btree_map::BTreeMap, string::String, vec::Vec};
use core::hash;

#[cfg(feature = "pyo3")]
use pyo3::prelude::*;

use crate::{
    addresses::{AddressRange, Rom, RomVramRange, Size, Vram},
    analysis::StringGuesserLevel,
    collections::{
        addended_ordered_map::FindSettings, unordered_map::UnorderedMap,
        unordered_set::UnorderedSet,
    },
    config::{Compiler, Endian},
    context::Context,
    metadata::{ParentSectionMetadata, SegmentMetadata, SymbolType},
    parent_segment_info::ParentSegmentInfo,
    relocation::RelocationInfo,
    section_type::SectionType,
    sections::{
        processed::DataSectionProcessed, RomSection, RomSectionPreprocessed, Section,
        SectionCreationError, SectionPostProcessError, SectionPreprocessed,
    },
    str_decoding::Encoding,
    symbols::{
        before_proc::{data_sym::DataSymProperties, DataSym},
        Symbol, SymbolPreprocessed,
    },
};

#[derive(Debug, Clone)]
#[must_use]
pub struct DataSection {
    name: String,

    ranges: RomVramRange,

    parent_segment_info: ParentSegmentInfo,

    // in_section_offset: u32,
    section_type: SectionType,

    //
    data_symbols: Vec<DataSym>,

    symbol_vrams: UnorderedSet<Vram>,
}

impl DataSection {
    // TODO: fix
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn new(
        context: &mut Context,
        settings: &DataSectionSettings,
        name: String,
        raw_bytes: &[u8],
        rom: Rom,
        vram: Vram,
        parent_segment_info: ParentSegmentInfo,
        section_type: SectionType,
    ) -> Result<Self, SectionCreationError> {
        if raw_bytes.is_empty() {
            return Err(SectionCreationError::EmptySection { name, vram });
        }
        if (rom.inner() % 4) != (vram.inner() % 4) {
            // TODO: Does this check make sense? It would be weird if this kind of section existed, wouldn't it?
            return Err(SectionCreationError::RomVramAlignmentMismatch {
                name,
                rom,
                vram,
                multiple_of: 4,
            });
        }

        let size = Size::new(raw_bytes.len() as u32);
        let rom_range = AddressRange::new(rom, rom + size);
        let vram_range = AddressRange::new(vram, vram + size);
        let ranges = RomVramRange::new(rom_range, vram_range);

        // Ensure there's a symbol at the beginning of the section.
        context
            .find_owned_segment_mut(&parent_segment_info)?
            .add_symbol(vram, false)?;

        let owned_segment = context.find_owned_segment(&parent_segment_info)?;

        let (symbols_info_vec, auto_pads) = Self::find_symbols(
            owned_segment,
            settings,
            raw_bytes,
            vram_range,
            section_type,
            context.global_config().endian(),
        );

        let mut data_symbols = Vec::new();
        let mut symbol_vrams = UnorderedSet::new();

        for (i, (new_sym_vram, sym_type)) in symbols_info_vec.iter().enumerate() {
            let start = new_sym_vram.sub_vram(&vram).inner() as usize;
            let end = if i + 1 < symbols_info_vec.len() {
                symbols_info_vec[i + 1].0.sub_vram(&vram).inner() as usize
            } else {
                raw_bytes.len()
            };
            debug_assert!(
                start < end,
                "{:?} {} {} {} {}",
                rom,
                vram,
                start,
                end,
                raw_bytes.len()
            );

            let sym_rom = rom + Size::new(start as u32);

            symbol_vrams.insert(*new_sym_vram);

            let properties = DataSymProperties {
                parent_metadata: ParentSectionMetadata::new(
                    name.clone(),
                    vram,
                    parent_segment_info.clone(),
                ),
                compiler: settings.compiler,
                auto_pad_by: auto_pads.get(new_sym_vram).copied(),
                detected_type: *sym_type,
                encoding: settings.encoding,
            };
            let /*mut*/ sym = DataSym::new(context, raw_bytes[start..end].into(), sym_rom, *new_sym_vram, start, parent_segment_info.clone(), section_type, properties)?;

            data_symbols.push(sym);
        }

        Ok(Self {
            name,
            ranges,
            parent_segment_info,
            section_type,
            data_symbols,
            symbol_vrams,
        })
    }

    #[allow(clippy::type_complexity)]
    fn find_symbols(
        owned_segment: &SegmentMetadata,
        settings: &DataSectionSettings,
        raw_bytes: &[u8],
        vram_range: AddressRange<Vram>,
        section_type: SectionType,
        endian: Endian,
    ) -> (Vec<(Vram, Option<SymbolType>)>, UnorderedMap<Vram, Vram>) {
        let mut symbols_info = BTreeMap::new();
        // Ensure there's a symbol at the beginning of the section.
        symbols_info.insert(vram_range.start(), None);
        let mut auto_pads = UnorderedMap::new();

        if vram_range.start().inner() % 4 != 0 || section_type == SectionType::GccExceptTable {
            // Not word-aligned, so I don't think it would make sense to look for pointers.
            // Fallback to a simpler algorithm.
            // Alternatively, if this is a except table section then avoid looking doing analisys,
            // since we know it can only contain table(s) and DataSym will make sure to produce the
            // labels.

            for reference in owned_segment.find_references_range(vram_range) {
                let reference_vram = reference.vram();
                symbols_info.insert(reference_vram, reference.sym_type());
                if let Some(size) = reference.size() {
                    let next_vram = reference_vram + size;
                    if vram_range.in_range(next_vram) {
                        symbols_info.insert(next_vram, None);
                        auto_pads.insert(next_vram, reference_vram);
                    }
                }
            }

            return (symbols_info.into_iter().collect(), auto_pads);
        }

        let mut remaining_string_size = 0;

        let mut prev_sym_info: Option<(Vram, Option<SymbolType>)> = None;
        // If true: the previous symbol made us thought we may be in late_rodata
        let mut maybe_reached_late_rodata = false;
        // If true, we are sure we are in late_rodata
        let mut reached_late_rodata = false;

        let mut float_counter = 0;
        let mut float_padding_counter = 0;

        // Look for stuff that looks like addresses which point to symbols on this section
        for (i, word_bytes) in raw_bytes.chunks_exact(4).enumerate() {
            let local_offset = i * 4;

            let current_vram = vram_range.start() + Size::new(local_offset as u32);
            let b_vram = current_vram + Size::new(1);
            let c_vram = current_vram + Size::new(2);
            let d_vram = current_vram + Size::new(3);

            if remaining_string_size <= 0 {
                let a = owned_segment.find_reference(current_vram, FindSettings::new(false));
                let b = owned_segment.find_reference(b_vram, FindSettings::new(false));
                let c = owned_segment.find_reference(c_vram, FindSettings::new(false));
                let d = owned_segment.find_reference(d_vram, FindSettings::new(false));

                if b.is_none() && c.is_none() && d.is_none() {
                    // There's no symbol in between
                    let word = endian.word_from_bytes(word_bytes);

                    let current_type = match a {
                        None => prev_sym_info.and_then(|x| x.1),
                        Some(wrapper) => wrapper.sym_type(),
                    };

                    if maybe_reached_late_rodata
                        && matches!(
                            current_type,
                            Some(SymbolType::Float32 | SymbolType::Float64)
                        )
                        && a.is_some()
                    {
                        reached_late_rodata = true;
                    }

                    if let Some(a) = a {
                        if matches!(
                            a.sym_type(),
                            Some(SymbolType::Float32 | SymbolType::Float64)
                        ) {
                            float_counter = 1;
                            float_padding_counter = 0;
                        } else {
                            float_counter = 0;
                            float_padding_counter = 0;
                        }
                    } else if current_type == Some(SymbolType::Float32) {
                        float_counter += 1;
                        if word == 0 {
                            float_padding_counter += 1;
                        }
                    } else if current_type == Some(SymbolType::Float64) {
                        if current_vram.inner() % 8 == 0 {
                            if local_offset + 8 <= raw_bytes.len() {
                                float_counter += 1;
                                if endian
                                    .dword_from_bytes(&raw_bytes[local_offset..local_offset + 8])
                                    == 0
                                {
                                    float_padding_counter += 1;
                                }
                            } else {
                                float_counter = 0;
                                float_padding_counter = 0;
                            }
                        }
                    } else {
                        float_counter = 0;
                        float_padding_counter = 0;
                    }

                    let should_search_for_address =
                        current_type.is_none_or(|x| x.can_reference_symbols());

                    let word_vram = Vram::new(word);
                    if should_search_for_address {
                        // TODO: improve heuristic to determine if should search for symbols
                        if !owned_segment.is_vram_ignored(word_vram)
                            && vram_range.in_range(word_vram)
                        {
                            // Vram is contained in this section
                            let word_ref =
                                owned_segment.find_reference(word_vram, FindSettings::new(true));
                            if word_ref.is_none_or(|x| {
                                x.vram() == word_vram || current_type.is_some_and(|t| t.is_table())
                            }) {
                                // Only count this symbol if it doesn't have an addend.
                                // If it does have an addend then it may be part of a larger symbol.
                                symbols_info.entry(word_vram).or_default();
                            }
                        }
                    }

                    let word_ref = owned_segment.find_reference(word_vram, FindSettings::new(true));

                    if !owned_segment.is_vram_ignored(current_vram)
                        && word_ref.is_none_or(|x| match x.sym_type() {
                            Some(SymbolType::Function) => x.vram() != word_vram,
                            Some(t) => {
                                if t.is_label() {
                                    x.vram() != word_vram
                                } else {
                                    false
                                }
                            }
                            _ => false,
                        })
                    {
                        let current_ref =
                            owned_segment.find_reference(current_vram, FindSettings::new(true));

                        if current_ref.is_none_or(|x| x.vram() == current_vram) {
                            let guessed_size = settings.string_guesser_level.guess(
                                current_ref,
                                current_vram,
                                &raw_bytes[local_offset..],
                                settings.encoding,
                                maybe_reached_late_rodata || reached_late_rodata,
                            );

                            match guessed_size {
                                Ok(str_size) => {
                                    let str_sym_size = str_size.next_multiple_of(4);
                                    let mut in_between_range =
                                        owned_segment.find_references_range(AddressRange::new(
                                            current_vram + Size::new(1),
                                            current_vram + Size::new(str_sym_size as u32),
                                        ));

                                    if in_between_range.next().is_none() {
                                        // Check if there is already another symbol after the current one and before the end of the string,
                                        // in which case we say this symbol should not be a string

                                        remaining_string_size = str_size as i32;

                                        *symbols_info.entry(current_vram).or_default() =
                                            Some(SymbolType::CString);
                                        if !auto_pads.contains_key(&current_vram) {
                                            auto_pads.insert(current_vram, current_vram);
                                        }

                                        let mut next_vram =
                                            current_vram + Size::new(str_sym_size as u32);
                                        if next_vram.inner() % 8 == 4 {
                                            // Some compilers align strings to 8, leaving some annoying padding.
                                            // We try to check if the next symbol is aligned, and if that's the case then grab the
                                            // padding into this symbol.
                                            if local_offset + str_sym_size + 4 <= raw_bytes.len() {
                                                let next_word = endian.word_from_bytes(
                                                    &raw_bytes[local_offset + str_sym_size..],
                                                );
                                                if next_word == 0 {
                                                    // Next word is zero, which means it could be padding bytes, so we have to check
                                                    // if it may be an actual symbol by checking if anything references it
                                                    if owned_segment
                                                        .find_reference(
                                                            next_vram,
                                                            FindSettings::new(false),
                                                        )
                                                        .is_none_or(|x| x.reference_counter() == 0)
                                                    {
                                                        let next_next_vram = Vram::new(
                                                            next_vram.inner().next_multiple_of(8),
                                                        );
                                                        if vram_range.in_range(next_next_vram) {
                                                            let next_next_ref = owned_segment
                                                                .find_reference(
                                                                    next_next_vram,
                                                                    FindSettings::new(false),
                                                                );

                                                            if let Some(compiler) =
                                                                settings.compiler
                                                            {
                                                                if next_next_ref.is_some_and(|x| {
                                                                    x.sym_type().is_some_and(|sym_type| {
                                                                        compiler
                                                                            .prev_align_for_type(sym_type)
                                                                            >= Some(3)
                                                                    })
                                                                }) {
                                                                    next_vram += Size::new(4);
                                                                }
                                                            }
                                                        } else if vram_range.end() == next_next_vram
                                                        {
                                                            // trailing padding, lets just add it here
                                                            next_vram += Size::new(4);
                                                        }
                                                    }
                                                }
                                            }
                                        }

                                        if vram_range.in_range(next_vram)
                                            && !owned_segment.is_vram_ignored(next_vram)
                                        {
                                            // Avoid generating a symbol at the end of the section
                                            symbols_info.entry(next_vram).or_default();
                                            auto_pads.insert(next_vram, current_vram);
                                        }

                                        // Next symbol should not be affected by this string.
                                        prev_sym_info = None;
                                    }
                                }

                                Err(_e) => {}
                            }
                        }
                    }
                }

                for (x_vram, x) in [(current_vram, a), (b_vram, b), (c_vram, c), (d_vram, d)] {
                    if owned_segment.is_vram_ignored(x_vram) {
                        continue;
                    }

                    if let Some(reference) = x {
                        symbols_info.entry(reference.vram()).or_default();

                        if let Some(size) = reference.user_declared_size() {
                            let next_vram = reference.vram() + size;

                            // Avoid generating a symbol at the end of the section
                            if vram_range.in_range(next_vram) {
                                let allow_next = match reference.sym_type() {
                                    Some(SymbolType::CString) => next_vram.inner() % 4 == 0,
                                    _ => true,
                                };
                                if allow_next {
                                    symbols_info.entry(next_vram).or_default();
                                    auto_pads.insert(next_vram, reference.vram());
                                }
                            }
                        }
                        prev_sym_info = Some((x_vram, reference.sym_type()));
                    }
                }
            }

            maybe_reached_late_rodata = false;
            if !reached_late_rodata
                && section_type == SectionType::Rodata
                && prev_sym_info
                    .is_some_and(|x| x.1.is_some_and(|x| x.is_late_rodata(settings.compiler())))
            {
                if prev_sym_info.is_some_and(|x| x.1 == Some(SymbolType::Jumptable)) {
                    reached_late_rodata = true;
                } else if float_padding_counter + 1 == float_counter {
                    // Finding a float or a double is not proof enough to say we are in late_rodata, because we
                    // can be after a const array of floats/doubles.
                    // An example of this is the libultra file `xldtob`.
                    // It is okay for late rodata floats to have padding, but if a float has non-zero padding
                    // it means it isn't a late_rodata float.
                    maybe_reached_late_rodata = true;
                }
            }
            remaining_string_size -= 4;
        }

        (symbols_info.into_iter().collect(), auto_pads)
    }
}

impl DataSection {
    pub fn data_symbols(&self) -> &[DataSym] {
        &self.data_symbols
    }
}

impl DataSection {
    pub fn post_process(
        self,
        context: &mut Context,
        user_relocs: &BTreeMap<Rom, RelocationInfo>,
    ) -> Result<DataSectionProcessed, SectionPostProcessError> {
        DataSectionProcessed::new(
            context,
            self.name,
            self.ranges,
            self.parent_segment_info,
            self.section_type,
            self.data_symbols,
            self.symbol_vrams,
            user_relocs,
        )
    }
}

impl Section for DataSection {
    fn name(&self) -> &str {
        &self.name
    }

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

    fn symbol_list(&self) -> &[impl Symbol] {
        &self.data_symbols
    }

    fn symbols_vrams(&self) -> &UnorderedSet<Vram> {
        &self.symbol_vrams
    }
}
impl RomSection for DataSection {
    fn rom_vram_range(&self) -> &RomVramRange {
        &self.ranges
    }
}
impl SectionPreprocessed for DataSection {
    fn symbol_list(&self) -> &[impl SymbolPreprocessed] {
        &self.data_symbols
    }
}
impl RomSectionPreprocessed for DataSection {}

impl hash::Hash for DataSection {
    fn hash<H: core::hash::Hasher>(&self, state: &mut H) {
        self.parent_segment_info.hash(state);
        self.ranges.hash(state);
    }
}
impl PartialEq for DataSection {
    fn eq(&self, other: &Self) -> bool {
        self.parent_segment_info == other.parent_segment_info && self.ranges == other.ranges
    }
}
impl PartialOrd for DataSection {
    fn partial_cmp(&self, other: &Self) -> Option<core::cmp::Ordering> {
        // Compare segment info first, so symbols get sorted by segment
        match self
            .parent_segment_info
            .partial_cmp(&other.parent_segment_info)
        {
            Some(core::cmp::Ordering::Equal) => {}
            ord => return ord,
        }
        self.ranges.partial_cmp(&other.ranges)
    }
}

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, PartialOrd, Ord)]
#[cfg_attr(feature = "pyo3", pyclass(module = "spimdisasm"))]
pub struct DataSectionSettings {
    compiler: Option<Compiler>,
    string_guesser_level: StringGuesserLevel,
    encoding: Encoding,
}

impl DataSectionSettings {
    pub fn new(compiler: Option<Compiler>) -> Self {
        Self {
            compiler,
            string_guesser_level: StringGuesserLevel::default(),
            encoding: Encoding::default(),
        }
    }

    pub fn compiler(&self) -> Option<Compiler> {
        self.compiler
    }

    pub fn string_guesser_level(&self) -> StringGuesserLevel {
        self.string_guesser_level
    }
    pub fn set_string_guesser_level(&mut self, string_guesser_level: StringGuesserLevel) {
        self.string_guesser_level = string_guesser_level;
    }
    pub fn with_string_guesser_level(self, string_guesser_level: StringGuesserLevel) -> Self {
        Self {
            string_guesser_level,
            ..self
        }
    }

    pub fn encoding(&self) -> Encoding {
        self.encoding
    }
    pub fn set_encoding(&mut self, encoding: Encoding) {
        self.encoding = encoding;
    }
    pub fn with_encoding(self, encoding: Encoding) -> Self {
        Self { encoding, ..self }
    }
}

#[cfg(feature = "pyo3")]
pub(crate) mod python_bindings {
    use super::*;

    #[pymethods]
    impl DataSectionSettings {
        #[new]
        #[pyo3(signature = (compiler))]
        pub fn py_new(compiler: Option<Compiler>) -> Self {
            Self::new(compiler)
        }

        #[pyo3(name = "set_string_guesser_level")]
        pub fn py_set_string_guesser_level(&mut self, string_guesser_level: StringGuesserLevel) {
            self.set_string_guesser_level(string_guesser_level)
        }

        #[pyo3(name = "set_encoding")]
        pub fn py_set_encoding(&mut self, encoding: Encoding) {
            self.set_encoding(encoding);
        }
    }
}
