/* SPDX-FileCopyrightText: © 2024 Decompollaborate */
/* SPDX-License-Identifier: MIT */

use rabbitizer::Vram;

use crate::{address_range::AddressRange, rom_address::RomAddress, size::Size};

#[derive(Debug, Copy, Clone, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct RomVramRange {
    rom: AddressRange<RomAddress>,
    vram: AddressRange<Vram>,
}

impl RomVramRange {
    #[must_use]
    pub fn new(rom: AddressRange<RomAddress>, vram: AddressRange<Vram>) -> Self {
        assert!(
            vram.size() >= rom.size(),
            "vram ({:?}) can't be smaller than rom ({:?})",
            vram,
            rom
        );
        assert!(
            rom.size() > Size::new(0),
            "rom ({:?}) must have non-zero size",
            rom
        );
        assert!(
            vram.size() > Size::new(0),
            "vram ({:?}) must have non-zero size",
            vram
        );
        assert!(
            vram.start().inner() % 4 == rom.start().inner() % 4,
            "vram ({:?}) and rom ({:?}) must have the same alignment",
            vram,
            rom
        );

        Self { rom, vram }
    }

    #[must_use]
    pub const fn rom(&self) -> AddressRange<RomAddress> {
        self.rom
    }
    #[must_use]
    pub const fn vram(&self) -> AddressRange<Vram> {
        self.vram
    }

    #[must_use]
    pub fn in_rom_range(&self, rom: RomAddress) -> bool {
        self.rom.in_range(rom)
    }
    #[must_use]
    pub fn in_vram_range(&self, vram: Vram) -> bool {
        self.vram.in_range(vram)
    }

    #[must_use]
    pub fn vram_fom_rom(&self, rom: RomAddress) -> Option<Vram> {
        self.rom.in_range(rom).then(|| {
            let diff = rom - self.rom.start();
            self.vram.start() + diff
        })
    }

    #[must_use]
    pub fn rom_from_vram(&self, vram: Vram) -> Option<RomAddress> {
        self.vram.in_range(vram).then(|| {
            let diff = (vram - self.vram.start())
                .try_into()
                .expect("This should not panic");
            self.rom.start() + diff
        })
    }
}
