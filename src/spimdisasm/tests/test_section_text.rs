/* SPDX-FileCopyrightText: © 2024 Decompollaborate */
/* SPDX-License-Identifier: MIT */

use rabbitizer::{InstructionFlags, Vram};
use spimdisasm::{
    context::{Context, GlobalConfig},
    rom_address::RomAddress,
    sections::{SectionText, SectionTextSettings},
};

#[cfg(test)]
#[test]
fn test_section_text_1() {
    use rabbitizer::DisplayFlags;
    use spimdisasm::{
        address_range::AddressRange, context::InputEndian, parent_segment_info::ParentSegmentInfo,
        size::Size, symbols::Symbol,
    };

    let bytes = &[
        // 0x80000400
        0x27, 0xBD, 0xFF, 0xE8, // addiu
        0xAF, 0xB0, 0x00, 0x10, // sw
        0xAF, 0xBF, 0x00, 0x14, // sw
        0x0C, 0x00, 0x01, 0x3F, // jal
        0x00, 0x80, 0x80, 0x21, //  addu
        0x0C, 0x00, 0x01, 0x3F, // jal
        0x02, 0x00, 0x20, 0x21, //  addu
        0x3C, 0x03, 0x80, 0x1A, // lui
        0x8C, 0x63, 0x6D, 0x7C, // lw
        0x24, 0x02, 0x00, 0x02, // addiu
        0x14, 0x62, 0x00, 0x08, // bne
        0x00, 0x00, 0x00, 0x00, //  nop
        0x0C, 0x00, 0x01, 0x3F, // jal
        0x02, 0x00, 0x20, 0x21, //  addu
        0x8F, 0xBF, 0x00, 0x14, // lw
        0x8F, 0xB0, 0x00, 0x10, // lw
        0x27, 0xBD, 0x00, 0x18, // addiu
        0x03, 0xE0, 0x00, 0x08, // jr
        0x00, 0x00, 0x00, 0x00, //  nop
        0x8F, 0xBF, 0x00, 0x14, // lw
        0x8F, 0xB0, 0x00, 0x10, // lw
        0x27, 0xBD, 0x00, 0x18, // addiu
        0x03, 0xE0, 0x00, 0x08, // jr
        0x00, 0x00, 0x00, 0x00, //  nop
        // 0x80000460
        0x27, 0xBD, 0xFF, 0xD0, // addiu
        0x3C, 0x04, 0x01, 0x07, // lui
        0x24, 0x84, 0x7C, 0x60, // addiu
        0xAF, 0xB1, 0x00, 0x24, // sw
        0x3C, 0x11, 0x80, 0x1C, // lui
        0x8E, 0x31, 0x70, 0xB0, // lw
        0x3C, 0x05, 0x01, 0x08, // lui
        0x24, 0xA5, 0x35, 0xA0, // addiu
        0x00, 0xA4, 0x28, 0x23, // subu
        0xAF, 0xBF, 0x00, 0x28, // sw
        0x0C, 0x02, 0x66, 0x00, // jal
        0xAF, 0xB0, 0x00, 0x20, //  sw
        0x3C, 0x04, 0x00, 0x58, // lui
        0x24, 0x84, 0x1D, 0xF0, // addiu
        0x3C, 0x10, 0x00, 0x59, // lui
        0x26, 0x10, 0xD7, 0x30, // addiu
        0x02, 0x04, 0x80, 0x23, // subu
        0x02, 0x00, 0x30, 0x21, // addu
        0x0C, 0x00, 0x04, 0xC4, // jal
        0x02, 0x20, 0x28, 0x21, //  addu
        0x02, 0x20, 0x20, 0x21, // addu
        0x3C, 0x02, 0x80, 0x1C, // lui
        0x8C, 0x42, 0x70, 0xB0, // lw
        0x24, 0x05, 0xFF, 0xFF, // addiu
        0x00, 0x00, 0x30, 0x21, // addu
        0xAF, 0xA0, 0x00, 0x10, // sw
        0xAF, 0xA0, 0x00, 0x14, // sw
        0xAF, 0xA0, 0x00, 0x18, // sw
        0x00, 0x50, 0x10, 0x21, // addu
        0x3C, 0x01, 0x80, 0x1C, // lui
        0xAC, 0x22, 0x70, 0xB0, // sw
        0x0C, 0x01, 0x39, 0x39, // jal
        0x24, 0x07, 0x00, 0x02, //  addiu
        0x8F, 0xBF, 0x00, 0x28, // lw
        0x8F, 0xB1, 0x00, 0x24, // lw
        0x8F, 0xB0, 0x00, 0x20, // lw
        0x27, 0xBD, 0x00, 0x30, // addiu
        0x03, 0xE0, 0x00, 0x08, // jr
        0x00, 0x00, 0x00, 0x00, //  nop
        // 0x800004FC
        0x27, 0xBD, 0xFF, 0xE8, // addiu
        0xAF, 0xBF, 0x00, 0x10, // sw
        0x3C, 0x01, 0x80, 0x22, // lui
        0xAC, 0x20, 0xAD, 0x28, // sw
        0x0C, 0x01, 0xD2, 0xBB, // jal
        0x00, 0x00, 0x00, 0x00, //  nop
        0x8F, 0xBF, 0x00, 0x10, // lw
        0x27, 0xBD, 0x00, 0x18, // addiu
        0x03, 0xE0, 0x00, 0x08, // jr
        0x00, 0x00, 0x00, 0x00, //  nop
    ];
    let rom = RomAddress::new(0x001050);
    let vram = Vram::new(0x80000400);
    let size = Size::new(0x1000);

    let global_config = GlobalConfig::new(InputEndian::Big);
    let mut context = Context::new(
        global_config,
        AddressRange::new(rom, rom + size),
        AddressRange::new(vram, vram + size),
    );
    let text_settings = SectionTextSettings::new(InstructionFlags::new());
    let display_flags = DisplayFlags::new();

    let section_text = SectionText::new(
        &mut context,
        text_settings,
        "test".into(),
        bytes,
        rom,
        vram,
        ParentSegmentInfo::new(rom, None),
    )
    .unwrap();

    for func in section_text.functions() {
        println!("func_{}:", func.vram());

        for instr in func.instructions() {
            println!("{}", instr.display(None, &display_flags));
        }

        println!();
    }

    assert_eq!(section_text.functions().len(), 3);

    let symbols = context.global_segment().symbols();
    for s in symbols {
        println!("{:?}", s.1);
    }
    assert_eq!(symbols.len(), 3);

    // None::<u32>.unwrap();
}
