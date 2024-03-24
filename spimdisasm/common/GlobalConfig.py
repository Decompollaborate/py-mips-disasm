#!/usr/bin/env python3

# SPDX-FileCopyrightText: © 2022-2024 Decompollaborate
# SPDX-License-Identifier: MIT

from __future__ import annotations

import argparse
import dataclasses
import enum
import os

from . import Utils
from .OrderedEnum import OrderedEnum


class InputEndian(enum.Enum):
    BIG = "big"
    LITTLE = "little"
    MIDDLE = "middle"

    @staticmethod
    def fromStr(value: str) -> InputEndian:
        if value == "little":
            return InputEndian.LITTLE
        if value == "middle":
            return InputEndian.MIDDLE
        return InputEndian.BIG

    def toFormatString(self) -> str:
        if self == InputEndian.BIG:
            return ">"
        if self == InputEndian.LITTLE:
            return "<"
        raise ValueError(f"No struct format string available for : {self}")


compilerOptions = {"IDO", "GCC", "SN64", "PSYQ", "EGCS", "MWCC", "EEGCC"}

@enum.unique
class Compiler(enum.Enum):
    UNKNOWN = None
    IDO = "IDO"
    GCC = "GCC"
    SN64 = "SN64"
    PSYQ = "PSYQ"
    EGCS = "EGCS"
    MWCC = "MWCC"
    EEGCC = "EEGCC"

    @staticmethod
    def fromStr(value: str) -> Compiler:
        if value not in compilerOptions:
            return Compiler.UNKNOWN
        return Compiler(value)


class Abi(enum.Enum):
    O32    = "O32"
    N32    = "N32"
    O64    = "O64"
    N64    = "N64"
    EABI32 = "EABI32"
    EABI64 = "EABI64"

    @staticmethod
    def fromStr(value: str) -> Abi:
        try:
            return Abi(value)
        except ValueError:
            return Abi.O32


archLevelOptions = {
    "MIPS1",
    "MIPS2",
    "MIPS3",
    "MIPS4",
    "MIPS5",
    "MIPS32",
    "MIPS64",
    "MIPS32R2",
    "MIPS64R2",
}

class ArchLevel(OrderedEnum):
    MIPS1       = 1
    MIPS2       = 2
    MIPS3       = 3
    MIPS4       = 4
    MIPS5       = 5
    MIPS32      = 6
    MIPS64      = 7
    MIPS32R2    = 8
    MIPS64R2    = 9

    @staticmethod
    def fromValue(value: int) -> ArchLevel|None:
        try:
            return ArchLevel(value)
        except ValueError:
            return None


class InputFileType(enum.Enum):
    BINARY = "binary"
    ELF = "elf"


@dataclasses.dataclass
class GlobalConfigType:
    DISASSEMBLE_UNKNOWN_INSTRUCTIONS: bool = False
    """Try to disassemble non implemented instructions and functions"""

    PRODUCE_SYMBOLS_PLUS_OFFSET: bool = True
    TRUST_USER_FUNCTIONS: bool = True
    TRUST_JAL_FUNCTIONS: bool = True

    RODATA_STRING_ENCODING: str = "EUC-JP"
    DATA_STRING_ENCODING: str = "ASCII"

    RODATA_STRING_GUESSER_LEVEL: int = 1
    """Rodata string guesser"""

    DATA_STRING_GUESSER_LEVEL: int = 2
    """Data string guesser"""

    PASCAL_RODATA_STRING_GUESSER_LEVEL: int = 0

    PASCAL_DATA_STRING_GUESSER_LEVEL: int = 0

    #! @deprecated
    @property
    def STRING_GUESSER(self) -> bool:
        return self.RODATA_STRING_GUESSER_LEVEL > 0
    #! @deprecated
    @STRING_GUESSER.setter
    def STRING_GUESSER(self, value: bool) -> None:
        if value:
            if self.RODATA_STRING_GUESSER_LEVEL <= 0:
                self.RODATA_STRING_GUESSER_LEVEL = 1
        else:
            self.RODATA_STRING_GUESSER_LEVEL = 0

    #! @deprecated
    @property
    def AGGRESSIVE_STRING_GUESSER(self) -> bool:
        return self.RODATA_STRING_GUESSER_LEVEL > 1
    #! @deprecated
    @AGGRESSIVE_STRING_GUESSER.setter
    def AGGRESSIVE_STRING_GUESSER(self, value: bool) -> None:
        if value:
            self.RODATA_STRING_GUESSER_LEVEL = 9
        else:
            if self.RODATA_STRING_GUESSER_LEVEL >= 0:
                self.RODATA_STRING_GUESSER_LEVEL = 1

    AUTOGENERATED_NAMES_BASED_ON_SECTION_TYPE: bool = True
    """Name autogenerated symbols after the section those are come from

    Use RO_ for symbols in .rodata and B_ for symbols in .bss"""

    AUTOGENERATED_NAMES_BASED_ON_DATA_TYPE: bool = True
    """Name autogenerated symbols after their type

    Use STR_ for strings, FLT_ for floats and DBL_ for doubles"""

    AUTOGENERATED_NAMES_BASED_ON_FILE_NAME: bool = False
    """Name autogenerated symbols after their containing file"""

    SEQUENTIAL_LABEL_NAMES: bool = False
    """Name branch and jump table labels after their containing function and a sequential number"""

    #! @deprecated
    LEGACY_SYM_ADDR_ZERO_PADDING: bool = False
    """
    Restore the legacy behavior of padding up to 6 digits with zeroes the autogenerated symbol names.
    The current behavior is to pad up to 8 digits with zeroes.
    This option is deprecated and may be removed in the future.
    """

    CUSTOM_SUFFIX: str = ""

    COMPILER: Compiler = Compiler.IDO

    DETECT_REDUNDANT_FUNCTION_END: bool = False
    """Tries to detect redundant and unreferenced functions ends and merge them together.
    This option is ignored if the compiler is not set to IDO"""

    ENDIAN: InputEndian = InputEndian.BIG
    """Endian for input binary files"""
    ENDIAN_DATA: InputEndian|None = None
    """If not None then specifies the endian for the .data section"""
    ENDIAN_RODATA: InputEndian|None = None
    """If not None then specifies the endian for the .rodata section"""

    ABI: Abi = Abi.O32
    """Controls tweaks related to the used ABI

    O32 is known as 'abi1'
    N32 is known as 'abi2'

    Please note this option does not control the register names used by rabbitizer
    """

    ARCHLEVEL: ArchLevel = ArchLevel.MIPS3

    INPUT_FILE_TYPE: InputFileType = InputFileType.BINARY

    GP_VALUE: int|None = None
    """Value used for $gp relocation loads and stores"""
    PIC: bool = False
    """Position independent code"""
    EMIT_CPLOAD: bool = True
    """Emits a .cpload directive instead of the corresponding instructions if it were detected"""

    EMIT_INLINE_RELOC: bool = False

    SYMBOL_FINDER_FILTER_LOW_ADDRESSES: bool = True
    """Toggle pointer detection for lower addresses (lower than SYMBOL_FINDER_FILTER_ADDRESSES_ADDR_LOW)"""
    SYMBOL_FINDER_FILTER_ADDRESSES_ADDR_LOW: int = 0x80000000
    SYMBOL_FINDER_FILTER_HIGH_ADDRESSES: bool = True
    """Toggle pointer detection for higher addresses (higher than SYMBOL_FINDER_FILTER_ADDRESSES_ADDR_HIGH)"""
    SYMBOL_FINDER_FILTER_ADDRESSES_ADDR_HIGH: int = 0xC0000000
    SYMBOL_FINDER_FILTERED_ADDRESSES_AS_CONSTANTS: bool = True
    """Treat filtered out addresses as constants pairs"""
    SYMBOL_FINDER_FILTERED_ADDRESSES_AS_HILO: bool = False
    """Allow using %hi/%lo syntax for filtered out addresses"""

    ALLOW_UNKSEGMENT: bool = True
    """Allow using symbols from the unknown segment"""

    ALLOW_ALL_ADDENDS_ON_DATA: bool = True
    """Enable using addends on symbols referenced by data"""
    ALLOW_ALL_CONSTANTS_ON_DATA: bool = True
    """Enable referencing constants by data"""


    ASM_COMMENT: bool = True
    """Toggle the comments in generated assembly code"""
    ASM_COMMENT_OFFSET_WIDTH: int = 6
    GLABEL_ASM_COUNT: bool = True
    """Toggle the glabel count comment on functions"""
    ASM_REFERENCEE_SYMBOLS: bool = False

    ASM_TEXT_LABEL: str = "glabel"
    ASM_TEXT_ALT_LABEL: str = "glabel"
    ASM_JTBL_LABEL: str = "jlabel"
    ASM_DATA_LABEL: str = "dlabel"
    ASM_USE_SYMBOL_LABEL: bool = True
    ASM_TEXT_ENT_LABEL: str = ""
    ASM_TEXT_END_LABEL: str = ""
    ASM_TEXT_FUNC_AS_LABEL: bool = False
    ASM_DATA_SYM_AS_LABEL: bool = False
    ASM_EMIT_SIZE_DIRECTIVE: bool = True
    ASM_USE_PRELUDE: bool = True
    ASM_PRELUDE_USE_INCLUDES: bool = True
    ASM_PRELUDE_USE_INSTRUCTION_DIRECTIVES: bool = True
    ASM_PRELUDE_USE_SECTION_START: bool = True
    ASM_GENERATED_BY: bool = True

    ASM_GLOBALIZE_TEXT_LABELS_REFERENCED_BY_NON_JUMPTABLE: bool = False
    """
    Use `ASM_JTBL_LABEL` on text labels that are referenced by non jumptable
    symbols.

    This is turned off by default since text labels referenced by non
    jumptables is usually a symptom of something going wrong, like fake symbols
    fake references, or jumptables being disassembled as data instead of rodata
    """

    PRINT_NEW_FILE_BOUNDARIES: bool = False
    """Print to stdout every file boundary found in .text and .rodata"""

    USE_DOT_BYTE: bool = True
    """Disassemble symbols marked as bytes with .byte instead of .word"""
    USE_DOT_SHORT: bool = True
    """Disassemble symbols marked as shorts with .short instead of .word"""

    LINE_ENDS: str = "\n"

    PANIC_RANGE_CHECK: bool = False
    """Produce a fatal error if a range check fails instead of just printing a warning"""

    CREATE_DATA_PADS: bool = True
    """Create dummy and unreferenced data symbols after another symbol which has non-zero user-declared size.

    The generated pad symbols may have non-zero data"""
    CREATE_RODATA_PADS: bool = False
    """Create dummy and unreferenced rodata symbols after another symbol which has non-zero user-declared size.

    The generated pad symbols may have non-zero data"""

    QUIET: bool = False
    VERBOSE: bool = False


    PRINT_FUNCTION_ANALYSIS_DEBUG_INFO: bool = False
    PRINT_SYMBOL_FINDER_DEBUG_INFO: bool = False
    PRINT_UNPAIRED_LUIS_DEBUG_INFO: bool = False


    REMOVE_POINTERS: bool = False
    IGNORE_BRANCHES: bool = False
    """Ignores the address of every branch, jump and jal"""
    IGNORE_WORD_LIST: set[int] = dataclasses.field(default_factory=set)
    """Ignores words that starts in 0xXX"""
    WRITE_BINARY: bool = False
    """write to files splitted binaries"""


    def addParametersToArgParse(self, parser: argparse.ArgumentParser) -> None:
        backendConfig = parser.add_argument_group("Disassembler backend configuration")

        backendConfig.add_argument("--disasm-unknown", help=f"Force disassembling functions with unknown instructions. Defaults to {self.DISASSEMBLE_UNKNOWN_INSTRUCTIONS}", action=Utils.BooleanOptionalAction)

        backendConfig.add_argument("--rodata-string-encoding", help=f"Specify the encoding used for decoding all rodata strings. Defaults to {self.RODATA_STRING_ENCODING}")
        backendConfig.add_argument("--data-string-encoding", help=f"Specify the encoding used for decoding all rodata strings. Defaults to {self.DATA_STRING_ENCODING}")

        rodataStringGuesserHelp = f"""\
Sets the level for the rodata C string guesser. Smaller values mean more conservative methods to guess a string, while higher values are more agressive. Level 0 (and negative) completely disables the guessing feature. Defaults to {self.RODATA_STRING_GUESSER_LEVEL}.

A C string must start at a 0x4-aligned region, which is '\\0' terminated and padded with '\\0's until a 0x4 boundary.

- level 0: Completely disable the guessing feature.
- level 1: The most conservative guessing level. Imposes the following restrictions:
    - Do not try to guess if the user provided a type for the symbol.
    - Do no try to guess if type information for the symbol can be inferred by other means.
    - A string symbol must be referenced only once.
    - Strings must not be empty.
- level 2: A string no longer needs to be referenced only once to be considered a possible string. This can happen because of a deduplication optimization.
- level 3: Empty strings are allowed.
- level 4: Symbols with autodetected type information but no user type information can still be guessed as strings.
"""
        backendConfig.add_argument("--rodata-string-guesser", help=rodataStringGuesserHelp, type=int, metavar="level")
        backendConfig.add_argument("--data-string-guesser", help=f"Sets the level for the data C string guesser. See the explanation of `--rodata-string-guesser`. Defaults to {self.DATA_STRING_GUESSER_LEVEL}.", type=int, metavar="level")

        backendConfig.add_argument("--pascal-rodata-string-guesser", help=f"EXPERIMENTAL, this feature may change or be removed in the future. Sets the level for the data Pascal string guesser. See the explanation of `--rodata-string-guesser`. Defaults to {self.PASCAL_RODATA_STRING_GUESSER_LEVEL}.", type=int, metavar="level")
        backendConfig.add_argument("--pascal-data-string-guesser", help=f"EXPERIMENTAL, this feature may change or be removed in the future. Sets the level for the data Pascal string guesser. See the explanation of `--rodata-string-guesser`. Defaults to {self.PASCAL_DATA_STRING_GUESSER_LEVEL}.", type=int, metavar="level")

        backendConfig.add_argument("--string-guesser", help=f"DEPRECATED, prefer `--rodata-string-guesser`. Toggles the string guesser feature. Defaults to {self.STRING_GUESSER}", action=Utils.BooleanOptionalAction)
        backendConfig.add_argument("--aggressive-string-guesser", help=f"DEPRECATED, prefer `--rodata-string-guesser`. Makes the string guesser feature to be more aggressive when trying to detect strings. Requires `--string-guesser` to be enabled. Defaults to {self.AGGRESSIVE_STRING_GUESSER}", action=Utils.BooleanOptionalAction)


        backendConfig.add_argument("--name-vars-by-section", help=f"Toggles the naming-after-section feature for autogenerated names. This means autogenerated symbols get a RO_ or B_ prefix if the symbol is from a rodata or bss section. Defaults to {self.AUTOGENERATED_NAMES_BASED_ON_SECTION_TYPE}", action=Utils.BooleanOptionalAction)
        backendConfig.add_argument("--name-vars-by-type", help=f"Toggles the naming-after-type feature for autogenerated names. This means autogenerated symbols can get a STR_, FLT_ or DBL_ prefix if the symbol is a string, float or double. Defaults to {self.AUTOGENERATED_NAMES_BASED_ON_DATA_TYPE}", action=Utils.BooleanOptionalAction)
        backendConfig.add_argument("--name-vars-by-file", help=f"Toggles the naming-after-file feature for autogenerated names. This means autogenerated symbols are named via a file name and an offset if the symbol is from a file. Defaults to {self.AUTOGENERATED_NAMES_BASED_ON_FILE_NAME}", action=Utils.BooleanOptionalAction)
        backendConfig.add_argument("--sequential-label-names", help=f"Toggles naming branch and jump table labels after their containing function and a sequential number. Defaults to {self.SEQUENTIAL_LABEL_NAMES}", action=Utils.BooleanOptionalAction)

        backendConfig.add_argument("--legacy-sym-addr-zero-padding", help=f"Restore the legacy behavior of padding up to 6 digits with zeroes the autogenerated symbol names. The current behavior is to pad up to 8 digits with zeroes. This option is deprecated and may be removed in the future. Defaults to {self.LEGACY_SYM_ADDR_ZERO_PADDING}", action=Utils.BooleanOptionalAction)

        backendConfig.add_argument("--custom-suffix", help="Set a custom suffix for automatically generated symbols")

        backendConfig.add_argument("--compiler", help=f"Enables some tweaks for the selected compiler. Defaults to {self.COMPILER.name}", choices=compilerOptions)
        backendConfig.add_argument("--detect-redundant-function-end", help=f"Tries to detect redundant and unreferenced function ends (jr $ra; nop), and merge it into the previous function. Currently it only is applied when the compiler is set to IDO. Defaults to {self.DETECT_REDUNDANT_FUNCTION_END}", action=Utils.BooleanOptionalAction)

        backendConfig.add_argument("--endian", help=f"Set the endianness of input files. Defaults to {self.ENDIAN.name.lower()}", choices=["big", "little", "middle"], default=self.ENDIAN.name.lower())

        backendConfig.add_argument("--abi", help=f"Changes the ABI of the disassembly, applying corresponding tweaks. Defaults to {self.ABI.name}", choices=["O32", "N32", "O64", "N64", "EABI32", "EABI64"], default=self.ABI.name)
        backendConfig.add_argument("--arch-level", help=f"Changes the arch level of the disassembly, applying corresponding tweaks. Defaults to {self.ARCHLEVEL.name}", choices=archLevelOptions, default=self.ARCHLEVEL.name)


        backendConfig.add_argument("--gp", help="Set the value used for loads and stores related to the $gp register. A hex value is expected")
        backendConfig.add_argument("--pic", help=f"Enables PIC analysis and the usage of some rel types, like %%got. Defaults to {self.PIC}", action=Utils.BooleanOptionalAction)
        backendConfig.add_argument("--emit-cpload", help=f"Emits a .cpload directive instead of the corresponding instructions if it were detected on PIC binaries. Defaults to {self.EMIT_CPLOAD}", action=Utils.BooleanOptionalAction)

        backendConfig.add_argument("--emit-inline-reloc", help=f"Emit a comment indicating the relocation in each instruction/word. Defaults to {self.EMIT_INLINE_RELOC}", action=Utils.BooleanOptionalAction)

        backendConfig.add_argument("--filter-low-addresses", help=f"Filter out low addresses (lower than 0x40000000) when searching for pointers. Defaults to {self.SYMBOL_FINDER_FILTER_LOW_ADDRESSES}", action=Utils.BooleanOptionalAction)
        backendConfig.add_argument("--filter-high-addresses", help=f"Filter out high addresses (higher than 0xC0000000) when searching for pointers. Defaults to {self.SYMBOL_FINDER_FILTER_HIGH_ADDRESSES}", action=Utils.BooleanOptionalAction)
        backendConfig.add_argument("--filtered-addresses-as-constants", help=f"Treat filtered out addressed as constants. Defaults to {self.SYMBOL_FINDER_FILTERED_ADDRESSES_AS_CONSTANTS}", action=Utils.BooleanOptionalAction)
        backendConfig.add_argument("--filtered-addresses-as-hilo", help=f"Use %%hi/%%lo syntax for filtered out addresses. Defaults to {self.SYMBOL_FINDER_FILTERED_ADDRESSES_AS_HILO}", action=Utils.BooleanOptionalAction)

        backendConfig.add_argument("--allow-unksegment", help=f"Allow using symbols from the unknown segment. Defaults to {self.ALLOW_UNKSEGMENT}", action=Utils.BooleanOptionalAction)

        backendConfig.add_argument("--allow-all-addends-on-data", help=f"Enable using addends on symbols referenced by data. Defaults to {self.ALLOW_ALL_ADDENDS_ON_DATA}", action=Utils.BooleanOptionalAction)
        backendConfig.add_argument("--allow-all-constants-on-data", help=f"Enable referencing constants by data. Defaults to {self.ALLOW_ALL_CONSTANTS_ON_DATA}", action=Utils.BooleanOptionalAction)


        miscConfig = parser.add_argument_group("Disassembler misc options")

        miscConfig.add_argument("--asm-comments", help=f"Toggle the comments in generated assembly code. Defaults to {self.ASM_COMMENT}", action=Utils.BooleanOptionalAction)
        miscConfig.add_argument("--comment-offset-width", help=f"Sets the zeroes width padding for the file offset comment. Defaults to {self.ASM_COMMENT_OFFSET_WIDTH}", action=Utils.BooleanOptionalAction)
        miscConfig.add_argument("--glabel-count", help=f"Toggle glabel count comment. Defaults to {self.GLABEL_ASM_COUNT}", action=Utils.BooleanOptionalAction)
        miscConfig.add_argument("--asm-referencee-symbols", help=f"Toggle glabel count comment. Defaults to {self.ASM_REFERENCEE_SYMBOLS}", action=Utils.BooleanOptionalAction)

        miscConfig.add_argument("--asm-text-label", help=f"Changes the label used to declare functions. Defaults to {self.ASM_TEXT_LABEL}")
        miscConfig.add_argument("--asm-text-alt-label", help=f"Changes the label used to declare symbols in the middle of functions. Defaults to {self.ASM_TEXT_ALT_LABEL}")
        miscConfig.add_argument("--asm-jtbl-label", help=f"Changes the label used to declare jumptable labels. Defaults to {self.ASM_JTBL_LABEL}")
        miscConfig.add_argument("--asm-data-label", help=f"Changes the label used to declare data symbols. Defaults to {self.ASM_DATA_LABEL}")
        miscConfig.add_argument("--asm-use-symbol-label", help=f"Toggles the use of labels for symbols. Defaults to {self.ASM_USE_SYMBOL_LABEL}", action=Utils.BooleanOptionalAction)
        miscConfig.add_argument("--asm-ent-label", help=f"Tells the disassembler to start using an ent label for functions")
        miscConfig.add_argument("--asm-end-label", help=f"Tells the disassembler to start using an end label for functions")
        miscConfig.add_argument("--asm-func-as-label", help=f"Toggle adding the function name as an additional label. Defaults to {self.ASM_TEXT_FUNC_AS_LABEL}", action=Utils.BooleanOptionalAction)
        miscConfig.add_argument("--asm-data-as-label", help=f"Toggle adding the data symbol name as an additional label. Defaults to {self.ASM_DATA_SYM_AS_LABEL}", action=Utils.BooleanOptionalAction)
        miscConfig.add_argument("--asm-emit-size-directive", help=f"Toggles emitting a size directive to generated symbols. Defaults to {self.ASM_EMIT_SIZE_DIRECTIVE}", action=Utils.BooleanOptionalAction)
        miscConfig.add_argument("--asm-use-prelude", help=f"Toggle use of the default prelude for asm files. Defaults to {self.ASM_USE_PRELUDE}", action=Utils.BooleanOptionalAction)
        miscConfig.add_argument("--asm-prelude-use-includes", help=f"Toggle use of the asm includes on the default prelude. Has no effect if `--asm-use-prelude` is turned off. Defaults to {self.ASM_PRELUDE_USE_INCLUDES}", action=Utils.BooleanOptionalAction)
        miscConfig.add_argument("--asm-prelude-use-instruction-directives", help=f"Toggle use of the instruction directives on the default prelude. Has no effect if `--asm-use-prelude` is turned off. Defaults to {self.ASM_PRELUDE_USE_INSTRUCTION_DIRECTIVES}", action=Utils.BooleanOptionalAction)
        miscConfig.add_argument("--asm-prelude-use-section-start", help=f"Toggle use of the section start directive on the default prelude. Has no effect if `--asm-use-prelude` is turned off. Defaults to {self.ASM_PRELUDE_USE_SECTION_START}", action=Utils.BooleanOptionalAction)
        miscConfig.add_argument("--asm-generated-by", help=f"Toggle comment indicating the tool and version used to generate the disassembly. Defaults to {self.ASM_GENERATED_BY}", action=Utils.BooleanOptionalAction)

        miscConfig.add_argument("--asm-globalize-text-labels-referenced-by-non-jumptable", help=f"""\
Use `ASM_JTBL_LABEL` on text labels that are referenced by non jumptable
symbols.

This is turned off by default since text labels referenced by non
jumptables is usually a symptom of something going wrong, like fake symbols
fake references, or jumptables being disassembled as data instead of rodata.
Defaults to {self.ASM_GLOBALIZE_TEXT_LABELS_REFERENCED_BY_NON_JUMPTABLE}""", action=Utils.BooleanOptionalAction)

        miscConfig.add_argument("--print-new-file-boundaries", help=f"Print to stdout any new file boundary found. Defaults to {self.PRINT_NEW_FILE_BOUNDARIES}", action=Utils.BooleanOptionalAction)

        miscConfig.add_argument("--use-dot-byte", help=f"Disassemble symbols marked as bytes with .byte instead of .word. Defaults to {self.USE_DOT_BYTE}", action=Utils.BooleanOptionalAction)
        miscConfig.add_argument("--use-dot-short", help=f"Disassemble symbols marked as shorts with .short instead of .word. Defaults to {self.USE_DOT_SHORT}", action=Utils.BooleanOptionalAction)

        miscConfig.add_argument("--panic-range-check", help=f"Produce a fatal error if a range check fails instead of just printing a warning. Defaults to {self.PANIC_RANGE_CHECK}", action=Utils.BooleanOptionalAction)

        miscConfig.add_argument("--create-data-pads", help=f"Create dummy and unreferenced data symbols after another symbol which has non-zero user-declared size.\nThe generated pad symbols may have non-zero data. Defaults to {self.CREATE_DATA_PADS}", action=Utils.BooleanOptionalAction)
        miscConfig.add_argument("--create-rodata-pads", help=f"Create dummy and unreferenced rodata symbols after another symbol which has non-zero user-declared size.\nThe generated pad symbols may have non-zero data. Defaults to {self.CREATE_RODATA_PADS}", action=Utils.BooleanOptionalAction)


        verbosityConfig = parser.add_argument_group("Verbosity options")

        verbosityConfig.add_argument("-v", "--verbose", help="Enable verbose mode", action=Utils.BooleanOptionalAction)
        verbosityConfig.add_argument("-q", "--quiet", help="Silence most of the output", action=Utils.BooleanOptionalAction)


        debugging = parser.add_argument_group("Disassembler debugging options")

        debugging.add_argument("--debug-func-analysis", help="Enables some debug info printing related to the function analysis)", action=Utils.BooleanOptionalAction)
        debugging.add_argument("--debug-symbol-finder", help="Enables some debug info printing related to the symbol finder system)", action=Utils.BooleanOptionalAction)
        debugging.add_argument("--debug-unpaired-luis", help="Enables some debug info printing related to the unpaired LUI instructions)", action=Utils.BooleanOptionalAction)


    def processEnvironmentVariables(self) -> None:
        from typing import Any

        # Allows changing the global configuration by setting a SPIMDISASM_SETTINGNAME environment variable
        # For example: SPIMDISASM_EMIT_CPLOAD=False

        for attr in dir(self):
            if attr.startswith("__"):
                continue

            currentValue = getattr(self, attr)

            environmentValue: Any = os.getenv(f"SPIMDISASM_{attr}", currentValue)
            if environmentValue == currentValue:
                continue

            if isinstance(currentValue, bool):
                if environmentValue.upper() == "TRUE":
                    environmentValue = True
                elif environmentValue.upper() == "FALSE":
                    environmentValue = False
                elif environmentValue == "0":
                    environmentValue = False
                else:
                    environmentValue = bool(environmentValue)
            elif isinstance(currentValue, Compiler):
                environmentValue = Compiler.fromStr(environmentValue)
            elif isinstance(currentValue, InputEndian):
                environmentValue = InputEndian.fromStr(environmentValue)
            elif isinstance(currentValue, Abi):
                environmentValue = Abi.fromStr(environmentValue)
            elif isinstance(currentValue, ArchLevel):
                value = ArchLevel.fromValue(int(environmentValue))
                if value is not None:
                    environmentValue = value
                else:
                    environmentValue = currentValue
            elif isinstance(currentValue, int):
                environmentValue = int(environmentValue, 16)

            setattr(self, attr, environmentValue)

    def parseArgs(self, args: argparse.Namespace) -> None:
        if args.disasm_unknown is not None:
            self.DISASSEMBLE_UNKNOWN_INSTRUCTIONS = args.disasm_unknown

        if args.rodata_string_encoding is not None:
            self.RODATA_STRING_ENCODING = args.rodata_string_encoding
        if args.data_string_encoding is not None:
            self.DATA_STRING_ENCODING = args.data_string_encoding

        if args.rodata_string_guesser is not None:
            self.RODATA_STRING_GUESSER_LEVEL = args.rodata_string_guesser
        if args.data_string_guesser is not None:
            self.DATA_STRING_GUESSER_LEVEL = args.data_string_guesser

        if args.pascal_rodata_string_guesser is not None:
            self.PASCAL_RODATA_STRING_GUESSER_LEVEL = args.pascal_rodata_string_guesser
        if args.pascal_data_string_guesser is not None:
            self.PASCAL_DATA_STRING_GUESSER_LEVEL = args.pascal_data_string_guesser

        if args.string_guesser is not None:
            self.STRING_GUESSER = args.string_guesser
        if args.aggressive_string_guesser is not None:
            self.AGGRESSIVE_STRING_GUESSER = args.aggressive_string_guesser

        if args.name_vars_by_section is not None:
            self.AUTOGENERATED_NAMES_BASED_ON_SECTION_TYPE = args.name_vars_by_section
        if args.name_vars_by_type is not None:
            self.AUTOGENERATED_NAMES_BASED_ON_DATA_TYPE = args.name_vars_by_type
        if args.name_vars_by_file is not None:
            self.AUTOGENERATED_NAMES_BASED_ON_FILE_NAME = args.name_vars_by_file
        if args.sequential_label_names is not None:
            self.SEQUENTIAL_LABEL_NAMES = args.sequential_label_names

        if args.legacy_sym_addr_zero_padding is not None:
            self.LEGACY_SYM_ADDR_ZERO_PADDING = args.legacy_sym_addr_zero_padding

        if args.custom_suffix:
            self.CUSTOM_SUFFIX = args.custom_suffix

        if args.compiler is not None:
            self.COMPILER = Compiler.fromStr(args.compiler)

        if args.detect_redundant_function_end is not None:
            self.DETECT_REDUNDANT_FUNCTION_END = args.detect_redundant_function_end

        if args.endian is not None:
            self.ENDIAN = InputEndian.fromStr(args.endian)

        if args.abi is not None:
            self.ABI = Abi.fromStr(args.abi)

        arch_level = ArchLevel.fromValue(args.arch_level)
        if arch_level is not None:
            self.ARCHLEVEL = arch_level

        if args.gp is not None:
            self.GP_VALUE = int(args.gp, 16)
        if args.pic is not None:
            self.PIC = args.pic
        if args.emit_cpload is not None:
            self.EMIT_CPLOAD = args.emit_cpload

        if args.emit_inline_reloc is not None:
            self.EMIT_INLINE_RELOC = args.emit_inline_reloc

        if args.filter_low_addresses is not None:
            self.SYMBOL_FINDER_FILTER_LOW_ADDRESSES = args.filter_low_addresses
        if args.filter_high_addresses is not None:
            self.SYMBOL_FINDER_FILTER_HIGH_ADDRESSES = args.filter_high_addresses
        if args.filtered_addresses_as_constants is not None:
            self.SYMBOL_FINDER_FILTERED_ADDRESSES_AS_CONSTANTS = args.filtered_addresses_as_constants
        if args.filtered_addresses_as_hilo is not None:
            self.SYMBOL_FINDER_FILTERED_ADDRESSES_AS_HILO = args.filtered_addresses_as_hilo

        if args.allow_unksegment is not None:
            self.ALLOW_UNKSEGMENT = args.allow_unksegment

        if args.allow_all_addends_on_data is not None:
            self.ALLOW_ALL_ADDENDS_ON_DATA = args.allow_all_addends_on_data
        if args.allow_all_constants_on_data is not None:
            self.ALLOW_ALL_CONSTANTS_ON_DATA = args.allow_all_constants_on_data


        if args.asm_comments is not None:
            self.ASM_COMMENT = args.asm_comments
        if args.comment_offset_width is not None:
            self.ASM_COMMENT_OFFSET_WIDTH = args.comment_offset_width
        if args.glabel_count is not None:
            self.GLABEL_ASM_COUNT = args.glabel_count
        if args.asm_referencee_symbols is not None:
            self.ASM_REFERENCEE_SYMBOLS = args.asm_referencee_symbols

        if args.asm_text_label:
            self.ASM_TEXT_LABEL = args.asm_text_label
        if args.asm_text_alt_label:
            self.ASM_TEXT_ALT_LABEL = args.asm_text_alt_label
        if args.asm_jtbl_label:
            self.ASM_JTBL_LABEL = args.asm_jtbl_label
        if args.asm_data_label:
            self.ASM_DATA_LABEL = args.asm_data_label
        if args.asm_use_symbol_label is not None:
            self.ASM_USE_SYMBOL_LABEL = args.asm_use_symbol_label
        if args.asm_ent_label:
            self.ASM_TEXT_ENT_LABEL = args.asm_ent_label
        if args.asm_end_label:
            self.ASM_TEXT_END_LABEL = args.asm_end_label
        if args.asm_func_as_label is not None:
            self.ASM_TEXT_FUNC_AS_LABEL = args.asm_func_as_label
        if args.asm_data_as_label is not None:
            self.ASM_DATA_SYM_AS_LABEL = args.asm_data_as_label
        if args.asm_emit_size_directive is not None:
            self.ASM_EMIT_SIZE_DIRECTIVE = args.asm_emit_size_directive
        if args.asm_use_prelude is not None:
            self.ASM_USE_PRELUDE = args.asm_use_prelude
        if args.asm_prelude_use_includes is not None:
            self.ASM_PRELUDE_USE_INCLUDES = args.asm_prelude_use_includes
        if args.asm_prelude_use_instruction_directives is not None:
            self.ASM_PRELUDE_USE_INSTRUCTION_DIRECTIVES = args.asm_prelude_use_instruction_directives
        if args.asm_prelude_use_section_start is not None:
            self.ASM_PRELUDE_USE_SECTION_START = args.asm_prelude_use_section_start
        if args.asm_generated_by is not None:
            self.ASM_GENERATED_BY = args.asm_generated_by

        if args.print_new_file_boundaries is not None:
            self.PRINT_NEW_FILE_BOUNDARIES = args.print_new_file_boundaries

        if args.use_dot_byte is not None:
            self.USE_DOT_BYTE = args.use_dot_byte
        if args.use_dot_short is not None:
            self.USE_DOT_SHORT = args.use_dot_short

        if args.panic_range_check is not None:
            self.PANIC_RANGE_CHECK = args.panic_range_check

        if args.create_data_pads is not None:
            self.CREATE_DATA_PADS = args.create_data_pads
        if args.create_rodata_pads is not None:
            self.CREATE_RODATA_PADS = args.create_rodata_pads


        if args.verbose is not None:
            self.VERBOSE = args.verbose
        if args.quiet is not None:
            self.QUIET = args.quiet


        if args.debug_func_analysis is not None:
            self.PRINT_FUNCTION_ANALYSIS_DEBUG_INFO = args.debug_func_analysis
        if args.debug_symbol_finder is not None:
            self.PRINT_SYMBOL_FINDER_DEBUG_INFO = args.debug_symbol_finder
        if args.debug_unpaired_luis is not None:
            self.PRINT_UNPAIRED_LUIS_DEBUG_INFO = args.debug_unpaired_luis

GlobalConfig = GlobalConfigType()

GlobalConfig.processEnvironmentVariables()
