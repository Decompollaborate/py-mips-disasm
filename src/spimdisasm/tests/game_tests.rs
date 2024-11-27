/* SPDX-FileCopyrightText: © 2024 Decompollaborate */
/* SPDX-License-Identifier: MIT */

use rabbitizer::{InstructionFlags, Vram};
use spimdisasm::{
    address_range::AddressRange,
    context::{Context, ContextBuilder, GlobalConfig, InputEndian},
    metadata::SymbolType,
    parent_segment_info::ParentSegmentInfo,
    rom_address::RomAddress,
    sections::{SectionData, SectionDataSettings, SectionText, SectionTextSettings},
};

mod game_tests_info;
use game_tests_info::{
    create_drmario64_us_segments, create_drmario64_us_symbols, SegmentData, TestSection,
    TestSegment, TestSegmentInfo, UserSymbol,
};

pub fn get_ranges_from_segments(
    segments: &[TestSegment],
) -> (AddressRange<RomAddress>, AddressRange<Vram>) {
    let mut rom_start = None;
    let mut rom_end = None;
    let mut vram_start = None;
    let mut vram_end = None;

    for w in segments.windows(2) {
        let a = &w[0];
        let b = &w[1];

        match (a, b) {
            (TestSegment::EndMarker(..), _) => {
                panic!("Doesn't make sense")
            }
            (TestSegment::Info(x), TestSegment::EndMarker(y)) => {
                assert!(x.rom <= *y);

                rom_start.get_or_insert(x.rom);
                vram_start.get_or_insert(x.vram);

                rom_end = Some(*y);
                vram_end = Some(x.vram_end(*y - x.rom));
            }
            (TestSegment::Info(x), TestSegment::Info(y)) => {
                assert!(x.rom <= y.rom);

                rom_start.get_or_insert(x.rom);
                vram_start.get_or_insert(x.vram);

                rom_end.get_or_insert(y.rom);
                vram_end.get_or_insert(x.vram_end(y.rom - x.rom));
            }
        }
    }

    let global_rom_range = AddressRange::new(rom_start.unwrap(), rom_end.unwrap());
    let global_vram_range = AddressRange::new(vram_start.unwrap(), vram_end.unwrap());

    (global_rom_range, global_vram_range)
}

fn init_context(
    global_rom_range: AddressRange<RomAddress>,
    global_vram_range: AddressRange<Vram>,
    symbols: Vec<UserSymbol>,
    rom_bytes: &[u8],
    user_segments: &[TestSegment],
) -> Context {
    assert!(user_segments.len() >= 2);

    let mut builder = ContextBuilder::new(
        GlobalConfig::new(InputEndian::Big),
        global_rom_range,
        global_vram_range,
    );

    let mut global_segment = builder.global_segment();
    for sym in symbols {
        match sym {
            game_tests_info::UserSymbol::Info(user_symbol_info) => {
                let sym = match user_symbol_info.typ {
                    Some(SymbolType::Function) => global_segment.add_function(
                        user_symbol_info.name,
                        user_symbol_info.vram,
                        user_symbol_info.rom,
                    ),
                    Some(SymbolType::BranchLabel) => global_segment.add_branch_label(
                        user_symbol_info.name,
                        user_symbol_info.vram,
                        user_symbol_info.rom,
                    ),
                    Some(SymbolType::Jumptable) => global_segment.add_jumptable(
                        user_symbol_info.name,
                        user_symbol_info.vram,
                        user_symbol_info.rom,
                    ),
                    Some(SymbolType::JumptableLabel) => global_segment.add_jumptable_label(
                        user_symbol_info.name,
                        user_symbol_info.vram,
                        user_symbol_info.rom,
                    ),
                    Some(SymbolType::GccExceptTable) => global_segment.add_gcc_except_table(
                        user_symbol_info.name,
                        user_symbol_info.vram,
                        user_symbol_info.rom,
                    ),
                    Some(SymbolType::GccExceptTableLabel) => global_segment
                        .add_gcc_except_table_label(
                            user_symbol_info.name,
                            user_symbol_info.vram,
                            user_symbol_info.rom,
                        ),
                    Some(SymbolType::UserDeclared(name)) => {
                        let sym = global_segment.add_symbol(
                            user_symbol_info.name,
                            user_symbol_info.vram,
                            user_symbol_info.rom,
                        );
                        *sym.user_declared_type_mut() = Some(SymbolType::UserDeclared(name));
                        sym
                    }
                    Some(_) => todo!(),
                    None => global_segment.add_symbol(
                        user_symbol_info.name,
                        user_symbol_info.vram,
                        user_symbol_info.rom,
                    ),
                };

                if let Some(name_end) = user_symbol_info.name_end {
                    *sym.user_declared_name_end_mut() = Some(name_end);
                }
                if let Some(size) = user_symbol_info.size {
                    *sym.user_declared_size_mut() = Some(size);
                }
                *sym.rodata_migration_behavior_mut() = user_symbol_info.migration_behavior;
                if user_symbol_info.dont_allow_addend {
                    sym.set_dont_allow_addend();
                }
            }
            game_tests_info::UserSymbol::Ignored(_vram, _size) => {}
        }
    }

    let mut finder_heater = builder.process().process();

    for w in user_segments.windows(2) {
        let a = &w[0];
        let b = &w[1];
        match (a, b) {
            (TestSegment::EndMarker(..), _) => panic!(),
            (
                TestSegment::Info(info),
                TestSegment::Info(TestSegmentInfo {
                    rom: segment_rom_end,
                    ..
                })
                | TestSegment::EndMarker(segment_rom_end),
            ) => {
                for (i, sect) in info.sections.iter().enumerate() {
                    let rom_end = if i + 1 < info.sections.len() {
                        match info.sections[i + 1] {
                            TestSection::Text(rom, _)
                            | TestSection::Data(rom, _)
                            | TestSection::Rodata(rom, _)
                            | TestSection::Bin(rom, _) => rom,
                            TestSection::Bss(..) => *segment_rom_end,
                        }
                    } else {
                        *segment_rom_end
                    };

                    match sect {
                        TestSection::Text(rom, _) => finder_heater.preanalyze_text(
                            &SectionTextSettings::new(InstructionFlags::default()),
                            &rom_bytes[AddressRange::new(*rom, rom_end)],
                            *rom,
                            info.vram_from_rom(*rom),
                        ),
                        TestSection::Data(rom, _) => finder_heater.preanalyze_data(
                            &SectionDataSettings::new(),
                            &rom_bytes[AddressRange::new(*rom, rom_end)],
                            *rom,
                            info.vram_from_rom(*rom),
                        ),
                        TestSection::Rodata(rom, _) => finder_heater.preanalyze_rodata(
                            &SectionDataSettings::new(),
                            &rom_bytes[AddressRange::new(*rom, rom_end)],
                            *rom,
                            info.vram_from_rom(*rom),
                        ),
                        TestSection::Bss(..) | TestSection::Bin(..) => {}
                    }
                }
            }
        }
    }

    finder_heater.process().build()
}

fn init_segments(
    mut context: &mut Context,
    rom_bytes: &[u8],
    user_segments: Vec<TestSegment>,
) -> Vec<SegmentData> {
    assert!(user_segments.len() >= 2);

    let mut segments = Vec::new();

    for w in user_segments.windows(2) {
        let a = &w[0];
        let b = &w[1];
        match (a, b) {
            (TestSegment::EndMarker(..), _) => panic!(),
            (
                TestSegment::Info(info),
                TestSegment::Info(TestSegmentInfo {
                    rom: segment_rom_end,
                    ..
                })
                | TestSegment::EndMarker(segment_rom_end),
            ) => {
                let mut text_sections = Vec::new();
                let mut data_sections = Vec::new();

                let parent_segment_info = ParentSegmentInfo::new(info.rom, None);

                for (i, sect) in info.sections.iter().enumerate() {
                    let rom_end = if i + 1 < info.sections.len() {
                        match info.sections[i + 1] {
                            TestSection::Text(rom, _)
                            | TestSection::Data(rom, _)
                            | TestSection::Rodata(rom, _)
                            | TestSection::Bin(rom, _) => rom,
                            TestSection::Bss(..) => *segment_rom_end,
                        }
                    } else {
                        *segment_rom_end
                    };

                    match sect {
                        TestSection::Text(rom, name) => {
                            let text_settings =
                                SectionTextSettings::new(InstructionFlags::default());
                            text_sections.push(
                                SectionText::new(
                                    &mut context,
                                    &text_settings,
                                    (*name).into(),
                                    &rom_bytes[AddressRange::new(*rom, rom_end)],
                                    *rom,
                                    info.vram_from_rom(*rom),
                                    parent_segment_info.clone(),
                                )
                                .unwrap(),
                            );
                        }
                        TestSection::Data(rom, name) => {
                            let data_settings = SectionDataSettings::new();
                            data_sections.push(
                                SectionData::new(
                                    &mut context,
                                    &data_settings,
                                    (*name).into(),
                                    &rom_bytes[AddressRange::new(*rom, rom_end)],
                                    *rom,
                                    info.vram_from_rom(*rom),
                                    parent_segment_info.clone(),
                                )
                                .unwrap(),
                            );
                        }
                        TestSection::Rodata(..) => {}
                        TestSection::Bss(..) => {}
                        TestSection::Bin(..) => {}
                    }
                }
                segments.push(SegmentData {
                    name: info.name.into(),
                    text_sections,
                    data_sections,
                });
            }
        }
    }

    segments
}

#[cfg_attr(feature = "game_tests", test)]
#[allow(dead_code)]
fn drmario64_us_without_symbols() {
    let drmario64_us_segments = create_drmario64_us_segments();

    let rom_bytes = std::fs::read("../../baserom_uncompressed.us.z64").unwrap();

    let (global_rom_range, global_vram_range) = get_ranges_from_segments(&drmario64_us_segments);

    let mut context = init_context(
        global_rom_range,
        global_vram_range,
        Vec::new(),
        &rom_bytes,
        &drmario64_us_segments,
    );

    let segments = init_segments(&mut context, &rom_bytes, drmario64_us_segments);

    assert_eq!(context.global_segment().symbols().len(), 1459);

    /*
    for seg in &segments {
        for sect in &seg.text_sections {
            println!("{} {} {}", seg.name, sect.name(), sect.functions().len());
        }
    }
    */
    let function_count: usize = segments
        .iter()
        .map(|x| {
            x.text_sections
                .iter()
                .map(|y| y.functions().len())
                .sum::<usize>()
        })
        .sum();
    assert_eq!(function_count, 1402);

    let data_syms_count: usize = segments
        .iter()
        .map(|x| {
            x.data_sections
                .iter()
                .map(|y| y.data_symbols().len())
                .sum::<usize>()
        })
        .sum();
    assert_eq!(data_syms_count, 57);
}

#[cfg_attr(feature = "game_tests", test)]
#[allow(dead_code)]
fn drmario64_us_with_symbols() {
    let drmario64_us_segments = create_drmario64_us_segments();

    let rom_bytes = std::fs::read("../../baserom_uncompressed.us.z64").unwrap();

    let (global_rom_range, global_vram_range) = get_ranges_from_segments(&drmario64_us_segments);

    let mut context = init_context(
        global_rom_range,
        global_vram_range,
        create_drmario64_us_symbols(),
        &rom_bytes,
        &drmario64_us_segments,
    );

    let segments = init_segments(&mut context, &rom_bytes, drmario64_us_segments);

    assert_eq!(context.global_segment().symbols().len(), 2915);

    /*
    for seg in &segments {
        for sect in &seg.text_sections {
            println!("{} {} {}", seg.name, sect.name(), sect.functions().len());
        }
    }
    */
    let function_count: usize = segments
        .iter()
        .map(|x| {
            x.text_sections
                .iter()
                .map(|y| y.functions().len())
                .sum::<usize>()
        })
        .sum();
    assert_eq!(function_count, 1413);

    let data_syms_count: usize = segments
        .iter()
        .map(|x| {
            x.data_sections
                .iter()
                .map(|y| y.data_symbols().len())
                .sum::<usize>()
        })
        .sum();
    assert_eq!(data_syms_count, 439);
}
