#!/usr/bin/env python3

# SPDX-FileCopyrightText: © 2022 Decompollaborate
# SPDX-License-Identifier: MIT

from __future__ import annotations

import dataclasses
import enum
from typing import Callable
import rabbitizer

from .GlobalConfig import GlobalConfig
from .FileSectionType import FileSectionType


class SymbolSpecialType(enum.Enum):
    function        = enum.auto()
    branchlabel     = enum.auto()
    jumptable       = enum.auto()
    jumptablelabel  = enum.auto()
    hardwarereg     = enum.auto()
    constant        = enum.auto()


    def toStr(self) -> str:
        return "@" + self.name

    @staticmethod
    def fromStr(symTypeStr: str|None) -> SymbolSpecialType|None:
        if symTypeStr == "@function":
            return SymbolSpecialType.function
        if symTypeStr == "@branchlabel":
            return SymbolSpecialType.branchlabel
        if symTypeStr == "@jumptable":
            return SymbolSpecialType.jumptable
        if symTypeStr == "@jumptablelabel":
            return SymbolSpecialType.jumptablelabel
        if symTypeStr == "@hardwarereg":
            return SymbolSpecialType.hardwarereg
        if symTypeStr == "@constant":
            return SymbolSpecialType.constant
        return None


@dataclasses.dataclass
class AccessTypeInfo:
    size: int
    typeSigned: str|None
    typeUnsigned: str|None
    typeNameAliases: set[str] = dataclasses.field(default_factory=set)

    def typeMatchesAccess(self, typename) -> bool:
        if typename is None:
            return False
        if typename == self.typeSigned:
            return True
        if typename == self.typeUnsigned:
            return True
        return typename in self.typeNameAliases

gAccessKinds: dict[rabbitizer.Enum, AccessTypeInfo] = {
    rabbitizer.AccessType.BYTE: AccessTypeInfo(1, "s8", "u8"),
    rabbitizer.AccessType.SHORT: AccessTypeInfo(2, "s16", "u16"),
    # Ignore signed WORD since it tends to not give a proper type
    rabbitizer.AccessType.WORD: AccessTypeInfo(1, None, "u32"),
    rabbitizer.AccessType.DOUBLEWORD: AccessTypeInfo(1, "s64", "u64"),
    rabbitizer.AccessType.FLOAT: AccessTypeInfo(1, "f32", None, {"Vec3f"}),
    rabbitizer.AccessType.DOUBLEFLOAT: AccessTypeInfo(1, "f64", None),
}


@dataclasses.dataclass
class ContextSymbol:
    address: int
    name: str|None = None
    size: int|None = None
    type: SymbolSpecialType|str|None = None

    accessType: rabbitizer.Enum|None = None
    unsignedAccessType: bool|None = None

    vromAddress: int|None = None

    sectionType: FileSectionType = FileSectionType.Unknown

    isDefined: bool = False
    "This symbol exists in any of the analyzed sections"
    isUserDeclared: bool = False
    "Declared externally by the user, but it may have not been found yet"
    isAutogenerated: bool = False
    "This symbol was automatically generated by the disassembler"

    isMaybeString: bool = False

    referenceCounter: int = 0
    "How much this symbol is referenced by something else"

    referenceFunctions: set[ContextSymbol] = dataclasses.field(default_factory=set)
    "Which functions reference this symbol"

    overlayCategory: str|None = None

    nameGetCallback: Callable[[ContextSymbol], str]|None = None
    """Used to register a name of a symbol which may change in the future outside of here

    The only parameter is the ContextSymbol itself, and it should return a string containing the name of the symbol.

    Used by .getName() instead of using the setted name or the default generated name.
    """

    unknownSegment: bool = False

    isGot: bool = False
    isGotGlobal: bool = False


    @property
    def vram(self) -> int:
        return self.address

    def hasNoType(self) -> bool:
        return (self.type is None or self.type == "") and self.accessType is None


    def isTrustableFunction(self, rsp: bool=False) -> bool:
        """Checks if the function symbol should be trusted based on the current disassembler settings"""
        if self.unknownSegment:
            return False

        if GlobalConfig.TRUST_USER_FUNCTIONS and self.isUserDeclared:
            if self.type == SymbolSpecialType.branchlabel:
                return False
            return True

        if GlobalConfig.TRUST_JAL_FUNCTIONS and self.isAutogenerated and self.type == SymbolSpecialType.function:
            return True

        if rsp:
            return True

        return False


    def isByte(self) -> bool:
        if not GlobalConfig.USE_DOT_BYTE:
            return False
        # Type is checked first to favour user-declared type over the autodetected one
        if gAccessKinds[rabbitizer.AccessType.BYTE].typeMatchesAccess(self.type):
            return True
        if self.accessType == rabbitizer.AccessType.BYTE:
            return True
        return False

    def isShort(self) -> bool:
        if not GlobalConfig.USE_DOT_SHORT:
            return False
        if gAccessKinds[rabbitizer.AccessType.SHORT].typeMatchesAccess(self.type):
            return True
        if self.accessType == rabbitizer.AccessType.SHORT:
            return True
        return False


    def isString(self) -> bool:
        if self.type == "char" or self.type == "char*":
            return True
        elif self.hasNoType(): # no type information, let's try to guess
            if GlobalConfig.STRING_GUESSER and self.isMaybeString:
                return True
        return False

    def isFloat(self) -> bool:
        if gAccessKinds[rabbitizer.AccessType.FLOAT].typeMatchesAccess(self.type):
            return True
        if self.accessType == rabbitizer.AccessType.FLOAT:
            return True
        return False

    def isDouble(self) -> bool:
        if gAccessKinds[rabbitizer.AccessType.DOUBLEFLOAT].typeMatchesAccess(self.type):
            return True
        if self.accessType == rabbitizer.AccessType.DOUBLEFLOAT:
            return True
        return False

    def isJumpTable(self) -> bool:
        return self.type == SymbolSpecialType.jumptable

    def isMaybeConstVariable(self) -> bool:
        if self.isFloat():
            return False
        if self.isDouble():
            return False
        elif self.isJumpTable():
            return False
        elif self.isString():
            return False
        return True


    def isStatic(self) -> bool:
        if self.type == SymbolSpecialType.jumptablelabel:
            return False
        if self.name is None:
            return False
        return self.name.startswith(".")

    def isLateRodata(self) -> bool:
        # if self.referenceCounter > 1: return False # ?
        return self.isJumpTable() or self.isFloat() or self.isDouble()


    def getDefaultName(self) -> str:
        suffix = ""
        if self.overlayCategory is not None:
            suffix = "_"
            if self.vromAddress is not None:
                suffix += f"{self.vromAddress:06X}"

        if self.type is not None:
            if self.type == SymbolSpecialType.function:
                return f"func_{self.address:08X}{suffix}"
            if self.type == SymbolSpecialType.branchlabel:
                return f".L{self.address:08X}{suffix}"
            if self.type == SymbolSpecialType.jumptable:
                return f"jtbl_{self.address:08X}{suffix}"
            if self.type == SymbolSpecialType.jumptablelabel:
                return f"L{self.address:08X}{suffix}"

        if GlobalConfig.AUTOGENERATED_NAMES_BASED_ON_SECTION_TYPE:
            if self.sectionType == FileSectionType.Rodata:
                return f"RO_{self.address:06X}{suffix}"
            if self.sectionType == FileSectionType.Bss:
                return f"B_{self.address:06X}{suffix}"

        return f"D_{self.address:06X}{suffix}"

    def getName(self) -> str:
        if self.nameGetCallback is not None:
            return self.nameGetCallback(self)
        if self.name is None:
            return self.getDefaultName()
        return self.name

    def setNameIfUnset(self, name: str) -> bool:
        if self.name is None:
            self.type = name
            return True
        return False

    def setNameGetCallback(self, callback: Callable[[ContextSymbol], str]) -> None:
        self.nameGetCallback = callback

    def setNameGetCallbackIfUnset(self, callback: Callable[[ContextSymbol], str]) -> None:
        if self.nameGetCallback is None:
            self.nameGetCallback = callback

    def getSize(self) -> int:
        # User-declared size first
        if self.size is not None:
            return self.size

        # Infer size based on user-declared type
        if self.type is not None and not isinstance(self.type, SymbolSpecialType):
            for info in gAccessKinds.values():
                if info.typeMatchesAccess(self.type):
                    return info.size

        # Infer size based on instruction access type
        if self.accessType is not None:
            return gAccessKinds[self.accessType].size

        # Infer size based on symbol's address alignment
        if self.vram % 4 == 0:
            return 4
        if self.vram % 2 == 0:
            return 2
        return 1

    def getVrom(self) -> int:
        if self.vromAddress is None:
            return 0
        return self.vromAddress

    def setSizeIfUnset(self, size: int) -> bool:
        if size <= 0:
            return False
        if self.size is None:
            self.size = size
            return True
        return False

    def getType(self) -> str:
        if self.type is None:
            if self.accessType is not None and self.unsignedAccessType is not None:
                typeInfo = gAccessKinds[self.accessType]
                t = typeInfo.typeUnsigned if self.unsignedAccessType else typeInfo.typeSigned
                if t is not None:
                    return t
            return ""
        if isinstance(self.type, SymbolSpecialType):
            return self.type.toStr()
        return self.type

    def setTypeIfUnset(self, varType: str) -> bool:
        if self.hasNoType():
            self.type = varType
            return True
        return False

    def setAccessTypeIfUnset(self, accessType: rabbitizer.Enum, unsignedMemoryAccess: bool) -> bool:
        if self.accessType is None and self.unsignedAccessType is None:
            self.accessType = accessType
            self.unsignedAccessType = unsignedMemoryAccess
            return True
        return False

    def getSymbolPlusOffset(self, address: int) -> str:
        if self.address == address:
            return self.getName()
        if self.address > address:
            return f"{self.getName()} - 0x{self.address - address:X}"
        return f"{self.getName()} + 0x{address - self.address:X}"

    def getSymbolLabel(self) -> str:
        if not GlobalConfig.ASM_USE_SYMBOL_LABEL:
            return ""
        label = ""
        if self.isStatic():
            label += "# static variable" + GlobalConfig.LINE_ENDS
        if self.sectionType == FileSectionType.Text:
            label += GlobalConfig.ASM_TEXT_LABEL
        else:
            label += GlobalConfig.ASM_DATA_LABEL
        label += " " + self.getName()
        return label


    @staticmethod
    def getCsvHeader() -> str:
        output = "address,name,getName,getType,accessType,"
        output += "size,"
        output += "getSize,getVrom,sectionType,"
        output += "isDefined,isUserDeclared,isAutogenerated,isMaybeString,"
        output += "referenceCounter,overlayCategory,unknownSegment,"
        output += "isGot,isGotGlobal"
        return output

    def toCsv(self) -> str:
        output = f"0x{self.address:06X},{self.name},{self.getName()},{self.getType()},"
        if self.accessType is None:
            output += "None,"
        else:
            output += f"{self.accessType.name},"
        if self.size is None:
            output += "None,"
        else:
            output += f"0x{self.size:X},"
        output += f"0x{self.getSize():X},0x{self.getVrom():X},{self.sectionType.toStr()},"
        output += f"{self.isDefined},{self.isUserDeclared},{self.isAutogenerated},{self.isMaybeString},"
        output += f"{self.referenceCounter},{self.overlayCategory},{self.unknownSegment},"
        output += f"{self.isGot},{self.isGotGlobal}"
        return output


    def __eq__(self, other: object) -> bool:
        if not isinstance(other, ContextSymbol):
            return False
        return self.address == other.address and self.vromAddress == other.vromAddress

    # https://stackoverflow.com/a/56915493/6292472
    def __hash__(self):
        return hash((self.address, self.vromAddress))

class ContextOffsetSymbol(ContextSymbol):
    def __init__(self, offset: int, name: str, sectionType: FileSectionType, *args, **kwargs):
        super().__init__(offset, *args, **kwargs)
        self.name = name
        self.sectionType = sectionType

    # Relative to the start of the section
    @property
    def offset(self) -> int:
        return self.address

    def getName(self) -> str:
        if self.name is None:
            return super().getName()
        if self.isStatic():
            return self.name[1:]
        return self.name


class ContextRelocSymbol(ContextSymbol):
    relocSection: FileSectionType
    relocType: int = -1 # Same number as the .elf specification

    def __init__(self, offset: int, name: str|None, relocSection: FileSectionType, *args, **kwargs):
        super().__init__(offset, *args, **kwargs)
        self.name = name
        self.relocSection = relocSection

    # Relative to the start of the section
    @property
    def offset(self) -> int:
        return self.address

    def getNamePlusOffset(self, offset: int) -> str:
        if offset == 0:
            return self.getName()
        if offset < 0:
            return f"{self.getName()} - 0x{-offset:X}"
        return f"{self.getName()} + 0x{offset:X}"

    def toCsv(self) -> str:
        return super().toCsv() + f",{self.relocSection.toStr()},{self.relocType}"
