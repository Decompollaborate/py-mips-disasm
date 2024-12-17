/* SPDX-FileCopyrightText: © 2024 Decompollaborate */
/* SPDX-License-Identifier: MIT */

use alloc::{collections::btree_set::BTreeSet, vec::Vec};
use rabbitizer::{Instruction, Vram};

use crate::{
    address_range::AddressRange,
    analysis::{InstructionAnalysisResult, InstructionAnalyzer},
    config::Compiler,
    context::{Context, OwnedSegmentNotFoundError},
    metadata::{GeneratedBy, ParentSectionMetadata, SegmentMetadata, SymbolMetadata},
    parent_segment_info::ParentSegmentInfo,
    relocation::{RelocReferencedSym, RelocationInfo, RelocationType},
    rom_address::RomAddress,
    rom_vram_range::RomVramRange,
    section_type::SectionType,
    size::Size,
};

use super::{
    display::{FunctionDisplay, FunctionDisplaySettings, SymDisplayError},
    trait_symbol::RomSymbol,
    Symbol,
};

#[derive(Debug, Clone, Hash, PartialEq)]
pub struct SymbolFunction {
    ranges: RomVramRange,
    instructions: Vec<Instruction>,
    parent_segment_info: ParentSegmentInfo,

    instr_analysis: InstructionAnalysisResult,
    relocs: Vec<Option<RelocationInfo>>,
}

impl SymbolFunction {
    pub(crate) fn new(
        context: &mut Context,
        instructions: Vec<Instruction>,
        rom: RomAddress,
        vram: Vram,
        _in_section_offset: usize,
        parent_segment_info: ParentSegmentInfo,
        properties: SymbolFunctionProperties,
    ) -> Result<Self, OwnedSegmentNotFoundError> {
        let size = Size::new(instructions.len() as u32 * 4);
        let rom_range = AddressRange::new(rom, rom + size);
        let vram_range = AddressRange::new(vram, vram + size);
        let ranges = RomVramRange::new(rom_range, vram_range);

        let mut relocs = vec![None; instructions.len()];

        let instr_analysis = InstructionAnalyzer::analyze(context, ranges, &instructions);

        let owned_segment = context.find_owned_segment_mut(&parent_segment_info)?;
        let metadata = owned_segment.add_function(vram, Some(rom), GeneratedBy::Autogenerated);
        *metadata.autodetected_size_mut() = Some(size);
        metadata.set_defined();

        properties.apply_to_metadata(metadata);

        Self::process_instr_analysis_result_owned(
            &mut relocs,
            &instr_analysis,
            &ranges,
            &parent_segment_info,
            owned_segment,
        );
        Self::process_instr_analysis_result_referenced(
            &mut relocs,
            &instr_analysis,
            &ranges,
            context,
            &parent_segment_info,
        );
        Self::generate_relocs_from_analyzer(&mut relocs, &instr_analysis, &ranges, &instructions);

        Ok(Self {
            ranges,
            instructions,
            parent_segment_info,
            instr_analysis,
            relocs,
        })
    }

    fn process_instr_analysis_result_owned(
        relocs: &mut [Option<RelocationInfo>],
        instr_analysis: &InstructionAnalysisResult,
        ranges: &RomVramRange,
        parent_segment_info: &ParentSegmentInfo,
        owned_segment: &mut SegmentMetadata,
    ) {
        // TODO: Consider moving reloc generation to a later step

        for (instr_rom, target_vram) in instr_analysis.branch_targets() {
            /*
            if common.GlobalConfig.INPUT_FILE_TYPE == common.InputFileType.ELF:
                if self.getVromOffset(instrOffset) in self.context.globalRelocationOverrides:
                    # Avoid creating wrong symbols on elf files
                    continue
            */

            let branch_sym = owned_segment.add_branch_label(
                *target_vram,
                ranges.rom_from_vram(*target_vram),
                GeneratedBy::Autogenerated,
            );
            branch_sym.add_reference_function(
                ranges.vram().start(),
                parent_segment_info.clone(),
                *instr_rom,
            );
            if let Some(typ) = branch_sym.sym_type() {
                if typ.valid_branch_target() {
                    let instr_index = (*instr_rom - ranges.rom().start()).inner() / 4;
                    relocs[instr_index as usize] = Some(
                        RelocationType::R_MIPS_PC16
                            .new_reloc_info(RelocReferencedSym::Address(*target_vram)),
                    );

                    /*
                    labelSym.referenceCounter += 1
                    labelSym.referenceFunctions.add(self.contextSym)
                    labelSym.parentFunction = self.contextSym
                    labelSym.parentFileName = self.contextSym.parentFileName
                    self.contextSym.branchLabels.add(labelSym.vram, labelSym)
                    */
                }
            }
        }
        for (instr_rom, target_vram) in instr_analysis.branch_targets_outside() {
            /*
            if common.GlobalConfig.INPUT_FILE_TYPE == common.InputFileType.ELF:
                if self.getVromOffset(instrOffset) in self.context.globalRelocationOverrides:
                    # Avoid creating wrong symbols on elf files
                    continue
            */

            let branch_sym = owned_segment.add_branch_label(
                *target_vram,
                ranges.rom_from_vram(*target_vram),
                GeneratedBy::Autogenerated,
            );
            branch_sym.add_reference_function(
                ranges.vram().start(),
                parent_segment_info.clone(),
                *instr_rom,
            );
            if let Some(typ) = branch_sym.sym_type() {
                if typ.valid_branch_target() {
                    let instr_index = (*instr_rom - ranges.rom().start()).inner() / 4;
                    relocs[instr_index as usize] = Some(
                        RelocationType::R_MIPS_PC16
                            .new_reloc_info(RelocReferencedSym::Address(*target_vram)),
                    );

                    /*
                    labelSym.referenceCounter += 1
                    labelSym.referenceFunctions.add(self.contextSym)
                    labelSym.parentFunction = self.contextSym
                    labelSym.parentFileName = self.contextSym.parentFileName
                    self.contextSym.branchLabels.add(labelSym.vram, labelSym)
                    */
                }
            }
            // TODO: add some kind of comment mentioning this instr is branching outside the current function.
        }

        // Jump tables
        for (instr_rom, target_vram) in instr_analysis.referenced_jumptables() {
            let jumptable =
                owned_segment.add_jumptable(*target_vram, None, GeneratedBy::Autogenerated);
            jumptable.add_reference_function(
                ranges.vram().start(),
                parent_segment_info.clone(),
                *instr_rom,
            );
            /*
            jumpTable.parentFunction = self.contextSym
            self.contextSym.jumpTables.add(jumpTable.vram, jumpTable)
            */
        }
    }

    fn process_instr_analysis_result_referenced(
        relocs: &mut [Option<RelocationInfo>],
        instr_analysis: &InstructionAnalysisResult,
        ranges: &RomVramRange,
        context: &mut Context,
        parent_segment_info: &ParentSegmentInfo,
    ) {
        for (instr_rom, target_vram) in instr_analysis.func_calls() {
            /*
            if self.context.isAddressBanned(targetVram):
                continue
            */

            /*
            if common.GlobalConfig.INPUT_FILE_TYPE == common.InputFileType.ELF:
                if self.getVromOffset(instrOffset) in self.context.globalRelocationOverrides:
                    # Avoid creating wrong symbols on elf files
                    continue
            */

            if let Some(referenced_segment) =
                context.find_referenced_segment_mut(*target_vram, parent_segment_info)
            {
                let func_sym =
                    referenced_segment.add_function(*target_vram, None, GeneratedBy::Autogenerated);
                func_sym.add_reference_function(
                    ranges.vram().start(),
                    parent_segment_info.clone(),
                    *instr_rom,
                );
                /*
                funcSym.referenceCounter += 1
                funcSym.referenceFunctions.add(self.contextSym)
                */
            }
            let instr_index = (*instr_rom - ranges.rom().start()).inner() / 4;
            relocs[instr_index as usize] = Some(
                RelocationType::R_MIPS_26.new_reloc_info(RelocReferencedSym::Address(*target_vram)),
            );
        }

        for (instr_rom, symbol_vram) in instr_analysis.address_per_lo_instr() {
            /*
            if self.context.isAddressBanned(symVram):
                continue
            */
            /*
            if common.GlobalConfig.INPUT_FILE_TYPE == common.InputFileType.ELF:
                if self.getVromOffset(loOffset) in self.context.globalRelocationOverrides:
                    # Avoid creating wrong symbols on elf files
                    continue
            */
            /*
            symAccessDict = self.instrAnalyzer.possibleSymbolTypes.get(symVram, dict())
            symAccess = None
            if len(symAccessDict) == 1:
                # Infer type info if there's only one access type
                symAccess = list(symAccessDict)[0]
            */
            let sym_access = if let Some(sym_access_info) =
                instr_analysis.type_info_per_address().get(symbol_vram)
            {
                if sym_access_info.len() == 1 {
                    sym_access_info.iter().next().map(|(k, _v)| k)
                } else {
                    None
                }
            } else {
                None
            };

            if let Some(referenced_segment) =
                context.find_referenced_segment_mut(*symbol_vram, parent_segment_info)
            {
                let sym_metadata = referenced_segment.add_symbol(
                    *symbol_vram,
                    None,
                    GeneratedBy::Autogenerated,
                    None,
                    true,
                );
                sym_metadata.add_reference_function(
                    ranges.vram().start(),
                    parent_segment_info.clone(),
                    *instr_rom,
                );
                /*
                contextSym = self.addSymbol(symVram, isAutogenerated=True, allowAddendInstead=True)
                if contextSym is not None:
                    # TODO: do this in a less ugly way
                    if contextSym.address != symVram:
                        if contextSym.address % 4 != 0 or symVram % 4 != 0:
                            if contextSym.getType() in {"u16", "s16", "u8", "u8"} or (symAccess is not None and symAccess.accessType in {rabbitizer.AccessType.BYTE, rabbitizer.AccessType.SHORT}):
                                if not (contextSym.getSize() > 4):
                                    if contextSym.userDeclaredSize is None or symVram >= contextSym.address + contextSym.userDeclaredSize:
                                        if symAccess is not None:
                                            contextSym.setAccessTypeIfUnset(symAccess.accessType, symAccess.unsignedMemoryAccess)
                                        contextSym.setFirstLoAccessIfUnset(loOffset)
                                        contextSym = self.addSymbol(symVram, isAutogenerated=True)
                */

                /*
                contextSym.referenceCounter += 1
                contextSym.referenceFunctions.add(self.contextSym)
                contextSym.setFirstLoAccessIfUnset(loOffset)
                */
                if let Some(sym_access) = sym_access {
                    sym_metadata.set_access_type_if_unset(*sym_access);
                    /*
                    if contextSym.isAutogenerated:
                        # Handle mips1 doublefloats
                        if contextSym.accessType == rabbitizer.AccessType.FLOAT and common.GlobalConfig.ABI == common.Abi.O32:
                            instr = self.instructions[loOffset//4]
                            if instr.doesDereference() and instr.isFloat() and not instr.isDouble():
                                if instr.ft.value % 2 != 0:
                                    # lwc1/swc1 with an odd fpr means it is an mips1 doublefloats reference
                                    if symVram % 8 != 0:
                                        # We need to remove the the symbol pointing to the middle of this doublefloats
                                        got = contextSym.isGot
                                        gotLocal = contextSym.isGotLocal
                                        gotGlobal = contextSym.isGotGlobal
                                        self.removeSymbol(symVram)

                                        # Align down to 8
                                        symVram = (symVram >> 3) << 3
                                        contextSym = self.addSymbol(symVram, isAutogenerated=True)
                                        contextSym.referenceCounter += 1
                                        contextSym.referenceFunctions.add(self.contextSym)
                                        contextSym.setFirstLoAccessIfUnset(loOffset)
                                        contextSym.isGot = got
                                        contextSym.isGotLocal = gotLocal
                                        contextSym.isGotGlobal = gotGlobal
                                    contextSym.accessType = rabbitizer.AccessType.DOUBLEFLOAT
                                    contextSym.unsignedAccessType = False
                                    contextSym.isMips1Double = True
                    */
                }
            }

            let instr_index = (*instr_rom - ranges.rom().start()).inner() / 4;
            relocs[instr_index as usize] = Some(
                RelocationType::R_MIPS_LO16
                    .new_reloc_info(RelocReferencedSym::Address(*symbol_vram)),
            );
        }
        for (instr_rom, symbol_vram) in instr_analysis.address_per_hi_instr() {
            let instr_index = (*instr_rom - ranges.rom().start()).inner() / 4;
            relocs[instr_index as usize] = Some(
                RelocationType::R_MIPS_HI16
                    .new_reloc_info(RelocReferencedSym::Address(*symbol_vram)),
            );
        }

        /*
        # To debug jumptable rejection change this check to `True`
        if False:
            for jrInstrOffset, (referenceOffset, jtblAddress, branchOffset) in self.instrAnalyzer.rejectedjumpRegisterIntrOffset.items():
                self.endOfLineComment[jrInstrOffset//4] = f" /* Jumping to something at address 0x{jtblAddress:08X} (inferred from 0x{self.getVromOffset(referenceOffset):X}). Jumptable rejected by instruction at vrom 0x{self.getVromOffset(branchOffset):X} */
        "
        */

        /*
        if self.isLikelyHandwritten:
            for instr in self.instructions:
                instr.inHandwrittenFunction = self.isLikelyHandwritten
        */
    }

    fn generate_relocs_from_analyzer(
        relocs: &mut [Option<RelocationInfo>],
        instr_analysis: &InstructionAnalysisResult,
        ranges: &RomVramRange,
        instrs: &[Instruction],
    ) {
        /*
        for instrOffset, address in self.instrAnalyzer.symbolInstrOffset.items():
            if self.context.isAddressBanned(address):
                continue

            contextSym = self.getSymbol(address)

            gotHiLo = False
            gotSmall = False
            if contextSym is None and address < 0 and common.GlobalConfig.PIC and common.GlobalConfig.GP_VALUE is not None:
                # Negative pointer may mean it is a weird GOT access
                gotAccess = common.GlobalConfig.GP_VALUE + address
                gpAccess = self.context.gpAccesses.requestAddress(gotAccess)
                if gpAccess is not None:
                    address = gpAccess.address
                    contextSym = self.getSymbol(address)
                    gotHiLo = True
                    gotSmall = gpAccess.isSmallSection
                else:
                    common.Utils.eprint(4, f"0x{self.instructions[instrOffset//4].vram:08X}", f"0x{gotAccess:08X}", self.instructions[instrOffset//4].disassemble())
                    pass

            if contextSym is None:
                continue

            if contextSym.isGotGlobal:
                if instrOffset not in self.instrAnalyzer.gotAccessAddresses and not gotHiLo:
                    continue

            instr = self.instructions[instrOffset//4]

            relocType = self._getRelocTypeForInstruction(instr, instrOffset, contextSym, gotHiLo, gotSmall)
            if relocType == common.RelocType.MIPS_GPREL16:
                contextSym.accessedAsGpRel = True
            self.relocs[instrOffset] = common.RelocationInfo(relocType, contextSym, address - contextSym.vram)
        */

        /*
        for instrOffset in self.instrAnalyzer.cploadOffsets:
            # .cpload directive is meant to use the `_gp_disp` pseudo-symbol
            instr = self.instructions[instrOffset//4]

            relocType = self._getRelocTypeForInstruction(instr, instrOffset)
            self.relocs[instrOffset] = common.RelocationInfo(relocType, "_gp_disp")
        */

        /*
        for instrOffset, gpInfo in self.instrAnalyzer.gpSets.items():
            hiInstrOffset = gpInfo.hiOffset
            hiInstr = self.instructions[hiInstrOffset//4]
            instr = self.instructions[instrOffset//4]

            hiRelocType = self._getRelocTypeForInstruction(hiInstr, hiInstrOffset)
            relocType = self._getRelocTypeForInstruction(instr, instrOffset)
            if not common.GlobalConfig.PIC and gpInfo.value == common.GlobalConfig.GP_VALUE:
                self.relocs[hiInstrOffset] = common.RelocationInfo(hiRelocType, "_gp")
                self.relocs[instrOffset] = common.RelocationInfo(relocType, "_gp")
            else:
                # TODO: consider reusing the logic of the self.instrAnalyzer.symbolInstrOffset loop
                address = gpInfo.value
                if self.context.isAddressBanned(address):
                    continue

                contextSym = self.getSymbol(address)
                if contextSym is None:
                    continue

                self.relocs[hiInstrOffset] = common.RelocationInfo(hiRelocType, contextSym)
                self.relocs[instrOffset] = common.RelocationInfo(relocType, contextSym)
        */

        for (instr_rom, constant) in instr_analysis.constant_per_instr() {
            let instr_index = (*instr_rom - ranges.rom().start()).inner() / 4;
            let instr = &instrs[instr_index as usize];
            // TODO: proper reloc inference
            let reloc_type = if instr.opcode().can_be_hi() {
                RelocationType::R_CUSTOM_CONSTANT_HI
            } else {
                RelocationType::R_CUSTOM_CONSTANT_LO
            };

            // TODO: use `:08X`.
            relocs[instr_index as usize] = Some(
                reloc_type
                    .new_reloc_info(RelocReferencedSym::SymName(format!("0x{:X}", constant), 0)),
            );
        }
        /*
        for instrOffset, constant in self.instrAnalyzer.constantInstrOffset.items():
            instr = self.instructions[instrOffset//4]
            relocType = self._getRelocTypeForInstruction(instr, instrOffset)

            if relocType in {common.RelocType.MIPS_HI16, common.RelocType.MIPS_LO16}:
                # We can only do this kind of shenanigans for normal %hi/%lo relocs

                symbol = self.getConstant(constant)
                if symbol is not None:
                    self.relocs[instrOffset] = common.RelocationInfo(relocType, symbol.getName())
                elif common.GlobalConfig.SYMBOL_FINDER_FILTERED_ADDRESSES_AS_HILO:
                    self.relocs[instrOffset] = common.RelocationInfo(relocType, f"0x{constant:X}")
                else:
                    # Pretend this pair is a constant
                    loInstr = instr
                    if instr.canBeHi():
                        loInstr = self.instructions[self.instrAnalyzer.hiToLowDict[instrOffset] // 4]

                    generatedReloc = self._generateHiLoConstantReloc(constant, instr, loInstr)
                    if generatedReloc is not None:
                        self.relocs[instrOffset] = generatedReloc
            else:
                comment = f"Failed to symbolize address 0x{constant:08X} for {relocType.getPercentRel()}. Make sure this address is within the recognized valid address space."
                if relocType in {common.RelocType.MIPS_GPREL16, common.RelocType.MIPS_GOT16}:
                    if common.GlobalConfig.GP_VALUE is None:
                        comment += f" Please specify a gp_value."
                    elif not self.context.isInTotalVramRange(common.GlobalConfig.GP_VALUE):
                        comment += f" The provided gp_value (0x{common.GlobalConfig.GP_VALUE:08X}) seems wrong."
                self.endOfLineComment[instrOffset//4] = f" /* {comment} */
"
        */

        /*
        for instrOffset, targetVram in self.instrAnalyzer.funcCallInstrOffsets.items():
            funcSym = self.getSymbol(targetVram, tryPlusOffset=False)
            if funcSym is None:
                continue
            self.relocs[instrOffset] = common.RelocationInfo(common.RelocType.MIPS_26, funcSym)

        */

        // Handle unpaired `lui`s
        for (instr_rom, (_hi_reg, hi_imm)) in instr_analysis.hi_instrs() {
            if !instr_analysis
                .address_per_hi_instr()
                .contains_key(instr_rom)
                && !instr_analysis.constant_per_instr().contains_key(instr_rom)
            {
                let instr_index = (*instr_rom - ranges.rom().start()).inner() / 4;
                let constant = (*hi_imm as u32) << 16;

                // TODO: use `:08X`.
                relocs[instr_index as usize] =
                    Some(RelocationType::R_CUSTOM_CONSTANT_HI.new_reloc_info(
                        RelocReferencedSym::SymName(format!("0x{:X}", constant), 0),
                    ));
            }
        }
    }
}

impl SymbolFunction {
    // TODO: maybe remove?
    pub fn instructions(&self) -> &[Instruction] {
        &self.instructions
    }

    #[must_use]
    pub(crate) fn handwritten_instrs(&self) -> &BTreeSet<RomAddress> {
        self.instr_analysis.handwritten_instrs()
    }
}

impl<'ctx, 'sym, 'flg> SymbolFunction {
    pub fn display(
        &'sym self,
        context: &'ctx Context,
        settings: &'flg FunctionDisplaySettings,
    ) -> Result<FunctionDisplay<'ctx, 'sym, 'flg>, SymDisplayError> {
        FunctionDisplay::new(context, self, settings)
    }
}

impl Symbol for SymbolFunction {
    fn vram_range(&self) -> &AddressRange<Vram> {
        self.ranges.vram()
    }

    fn parent_segment_info(&self) -> &ParentSegmentInfo {
        &self.parent_segment_info
    }

    #[must_use]
    fn section_type(&self) -> SectionType {
        SectionType::Text
    }
}

impl RomSymbol for SymbolFunction {
    #[must_use]
    fn rom_vram_range(&self) -> &RomVramRange {
        &self.ranges
    }

    #[must_use]
    fn relocs(&self) -> &[Option<RelocationInfo>] {
        &self.relocs
    }
}

#[derive(Debug, Clone, Hash, PartialEq)]
pub(crate) struct SymbolFunctionProperties {
    pub parent_metadata: ParentSectionMetadata,
    pub compiler: Option<Compiler>,
    pub auto_pad_by: Option<Vram>,
}

impl SymbolFunctionProperties {
    fn apply_to_metadata(self, metadata: &mut SymbolMetadata) {
        metadata.set_parent_metadata(self.parent_metadata);

        if let Some(compiler) = self.compiler {
            metadata.set_compiler(compiler);
        }

        if let Some(auto_pad_by) = self.auto_pad_by {
            metadata.set_auto_created_pad_by(auto_pad_by);
        }
    }
}
