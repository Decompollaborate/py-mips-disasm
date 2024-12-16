/* SPDX-FileCopyrightText: © 2024 Decompollaborate */
/* SPDX-License-Identifier: MIT */

use alloc::{
    collections::{btree_map::BTreeMap, btree_set::BTreeSet},
    string::String,
    vec::Vec,
};
use rabbitizer::Vram;

#[cfg(feature = "pyo3")]
use pyo3::prelude::*;

use crate::{
    address_range::AddressRange,
    context::{Context, OwnedSegmentNotFoundError},
    parent_segment_info::ParentSegmentInfo,
    section_type::SectionType,
    symbols::{symbol_noload::SymbolNoloadProperties, Symbol, SymbolNoload},
};

use super::Section;

#[derive(Debug, Copy, Clone, Hash, PartialEq, Eq, PartialOrd, Ord)]
#[cfg_attr(feature = "pyo3", pyclass(module = "spimdisasm"))]
pub struct SectionNoloadSettings {}

impl SectionNoloadSettings {
    pub fn new() -> Self {
        Self {}
    }
}
impl Default for SectionNoloadSettings {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Hash, PartialEq)]
#[must_use]
#[cfg_attr(feature = "pyo3", pyclass(module = "spimdisasm"))]
pub struct SectionNoload {
    name: String,

    vram_range: AddressRange<Vram>,

    parent_segment_info: ParentSegmentInfo,

    // in_section_offset: u32,

    //
    noload_symbols: Vec<SymbolNoload>,

    symbol_vrams: BTreeSet<Vram>,
}

impl SectionNoload {
    pub(crate) fn new(
        context: &mut Context,
        _settings: &SectionNoloadSettings,
        name: String,
        vram_range: AddressRange<Vram>,
        parent_segment_info: ParentSegmentInfo,
    ) -> Result<Self, OwnedSegmentNotFoundError> {
        assert!(
            vram_range.size().inner() != 0,
            "Can't initialize zero-sized noload section. {:?}",
            vram_range
        );

        let mut noload_symbols = Vec::new();
        let mut symbol_vrams = BTreeSet::new();

        let owned_segment = context.find_owned_segment(&parent_segment_info)?;

        let mut symbols_info = BTreeSet::new();
        // Ensure there's a symbol at the beginning of the section.
        symbols_info.insert(vram_range.start());

        let mut auto_pads: BTreeMap<Vram, Vram> = BTreeMap::new();

        /*
        # If something that could be a pointer found in data happens to be in
        # the middle of this bss file's addresses space then consider it as a
        # new bss variable
        for ptr in self.getAndPopPointerInDataReferencesRange(self.bssVramStart, self.bssVramEnd):
            # Check if the symbol already exists, in case the user has provided size
            contextSym = self.getSymbol(ptr, tryPlusOffset=True)
            if contextSym is None:
                self.addSymbol(ptr, sectionType=self.sectionType, isAutogenerated=True)
        */

        for (sym_vram, sym) in
            owned_segment.find_symbols_range(vram_range.start(), vram_range.end())
        {
            symbols_info.insert(*sym_vram);

            if let Some(size) = sym.user_declared_size() {
                // TODO: signal this symbol is an autogenerated pad
                let next_vram = sym.vram() + size;
                if next_vram != vram_range.end() {
                    // Avoid generating a symbol at the end of the section
                    symbols_info.insert(next_vram);
                    auto_pads.insert(next_vram, sym.vram());
                }
            }
        }

        let symbols_info_vec: Vec<Vram> = symbols_info.into_iter().collect();

        for (i, new_sym_vram) in symbols_info_vec.iter().enumerate() {
            let start = new_sym_vram.sub_vram(&vram_range.start()).inner() as usize;
            let new_sym_vram_end = if i + 1 < symbols_info_vec.len() {
                symbols_info_vec[i + 1]
            } else {
                vram_range.end()
            };
            debug_assert!(
                *new_sym_vram < new_sym_vram_end,
                "{:?} {} {}",
                vram_range,
                new_sym_vram,
                new_sym_vram_end
            );

            symbol_vrams.insert(*new_sym_vram);

            let properties = SymbolNoloadProperties {
                auto_pad_by: auto_pads.get(new_sym_vram).copied(),
            };
            let /*mut*/ sym = SymbolNoload::new(context, AddressRange::new(*new_sym_vram, new_sym_vram_end), start, parent_segment_info.clone(), properties)?;

            noload_symbols.push(sym);
        }

        Ok(Self {
            name,
            vram_range,
            parent_segment_info,
            noload_symbols,
            symbol_vrams,
        })
    }

    // TODO: remove
    pub fn noload_symbols(&self) -> &[SymbolNoload] {
        &self.noload_symbols
    }
}

impl Section for SectionNoload {
    fn name(&self) -> &str {
        &self.name
    }

    fn vram_range(&self) -> &AddressRange<Vram> {
        &self.vram_range
    }

    fn parent_segment_info(&self) -> &ParentSegmentInfo {
        &self.parent_segment_info
    }

    #[must_use]
    fn section_type(&self) -> SectionType {
        SectionType::Bss
    }

    fn symbol_list(&self) -> &[impl Symbol] {
        &self.noload_symbols
    }

    fn symbols_vrams(&self) -> &BTreeSet<Vram> {
        &self.symbol_vrams
    }
}

#[cfg(feature = "pyo3")]
pub(crate) mod python_bindings {
    use crate::symbols::display::{SymDisplayError, SymNoloadDisplaySettings};

    use super::*;

    #[pymethods]
    impl SectionNoloadSettings {
        #[new]
        pub fn py_new() -> Self {
            Self::new()
        }
    }

    #[pymethods]
    impl SectionNoload {
        #[pyo3(name = "sym_count")]
        pub fn py_sym_count(&self) -> usize {
            self.noload_symbols.len()
        }

        #[pyo3(name = "display_sym")]
        pub fn py_display_sym(
            &self,
            context: &Context,
            index: usize,
            settings: &SymNoloadDisplaySettings,
        ) -> Result<Option<String>, SymDisplayError> {
            let sym = self.noload_symbols.get(index);

            Ok(if let Some(sym) = sym {
                Some(sym.display(context, settings)?.to_string())
            } else {
                None
            })
        }
    }
}
