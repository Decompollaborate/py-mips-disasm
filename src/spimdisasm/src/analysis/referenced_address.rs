/* SPDX-FileCopyrightText: © 2024-2025 Decompollaborate */
/* SPDX-License-Identifier: MIT */

use core::hash::Hash;

use crate::{
    addresses::Size, addresses::Vram, collections::unordered_map::UnorderedMap,
    metadata::SymbolType,
};

#[derive(Debug, Clone, Eq)]
pub struct ReferencedAddress {
    vram: Vram,

    sym_type: UnorderedMap<SymbolType, u32>,

    sizes: UnorderedMap<Option<Size>, u32>,
    alignments: UnorderedMap<Option<u8>, u32>,

    reference_count: usize,
}

impl ReferencedAddress {
    pub fn new(vram: Vram) -> Self {
        Self {
            vram,

            sym_type: UnorderedMap::new(),

            alignments: UnorderedMap::new(),
            sizes: UnorderedMap::new(),

            reference_count: 0,
        }
    }

    pub const fn vram(&self) -> Vram {
        self.vram
    }

    pub fn sym_type(&self) -> Option<SymbolType> {
        if self.sym_type.len() == 1 {
            self.sym_type.iter().next().map(|(typ, _count)| *typ)
        } else {
            None
        }
    }

    pub fn size(&self) -> Option<Size> {
        if self.sizes.len() == 1 {
            self.sizes.iter().next().and_then(|(siz, _count)| *siz)
        } else {
            None
        }
    }

    pub fn alignment(&self) -> Option<u8> {
        if self.alignments.len() == 1 {
            self.alignments.iter().next().and_then(|(x, _count)| *x)
        } else {
            None
        }
    }

    pub fn reference_counter(&self) -> usize {
        self.reference_count
    }

    pub fn set_sym_type(&mut self, sym_type: SymbolType) {
        *self.sym_type.entry(sym_type).or_default() += 1;
    }

    pub fn set_size(&mut self, val: Option<u8>) {
        *self
            .sizes
            .entry(val.map(|x| Size::new(x.into())))
            .or_default() += 1;
    }
    pub fn set_alignment(&mut self, val: Option<u8>) {
        *self.alignments.entry(val).or_default() += 1;
    }

    pub fn increment_references(&mut self) {
        self.reference_count += 1;
    }
}

impl PartialEq for ReferencedAddress {
    fn eq(&self, other: &Self) -> bool {
        self.vram == other.vram
    }
}
impl PartialOrd for ReferencedAddress {
    fn partial_cmp(&self, other: &Self) -> Option<core::cmp::Ordering> {
        self.vram.partial_cmp(&other.vram)
    }
}
impl Hash for ReferencedAddress {
    fn hash<H: core::hash::Hasher>(&self, state: &mut H) {
        self.vram.hash(state);
    }
}
