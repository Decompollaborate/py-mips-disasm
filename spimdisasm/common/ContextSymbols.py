#!/usr/bin/env python3

# SPDX-FileCopyrightText: © 2022-2024 Decompollaborate
# SPDX-License-Identifier: MIT

from __future__ import annotations

import dataclasses
import enum
from typing import Callable
import rabbitizer

from .GlobalConfig import GlobalConfig, Compiler
from .FileSectionType import FileSectionType
from .SortedDict import SortedDict


class SymbolSpecialType(enum.Enum):
    function            = enum.auto()
    branchlabel         = enum.auto()
    jumptable           = enum.auto()
    jumptablelabel      = enum.auto()
    hardwarereg         = enum.auto()
    constant            = enum.auto()
    gccexcepttable      = enum.auto()
    gccexcepttablelabel = enum.auto()


    def isTargetLabel(self) -> bool:
        if self == SymbolSpecialType.branchlabel:
            return True
        if self == SymbolSpecialType.jumptablelabel:
            return True
        if self == SymbolSpecialType.gccexcepttablelabel:
            return True
        return False

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
        if symTypeStr == "@gccexcepttable":
            return SymbolSpecialType.gccexcepttable
        if symTypeStr == "@gccexcepttablelabel":
            return SymbolSpecialType.gccexcepttablelabel
        return None


@dataclasses.dataclass
class AccessTypeInfo:
    size: int
    typeSigned: str|None
    typeUnsigned: str|None
    typeNameAliases: set[str] = dataclasses.field(default_factory=set)

    def typeMatchesAccess(self, typename: SymbolSpecialType|str|None) -> bool:
        if typename is None:
            return False
        if typename == self.typeSigned:
            return True
        if typename == self.typeUnsigned:
            return True
        return typename in self.typeNameAliases

    def getAllTypes(self) -> set[str]:
        types: set[str] = set()

        if self.typeSigned is not None:
            types.add(self.typeSigned)
        if self.typeUnsigned is not None:
            types.add(self.typeUnsigned)

        types |= self.typeNameAliases
        return types

gAccessKinds: dict[rabbitizer.Enum, AccessTypeInfo] = {
    rabbitizer.AccessType.BYTE: AccessTypeInfo(1, "s8", "u8", {"vs8", "vu8"}),
    rabbitizer.AccessType.SHORT: AccessTypeInfo(2, "s16", "u16", {"vs16", "vu16"}),
    # Ignore signed WORD since it tends to not give a proper type
    rabbitizer.AccessType.WORD: AccessTypeInfo(4, None, "u32", {"s32", "vs32", "vu32"}),
    rabbitizer.AccessType.DOUBLEWORD: AccessTypeInfo(8, "s64", "u64", {"vs64", "vu64"}),
    rabbitizer.AccessType.FLOAT: AccessTypeInfo(4, "f32", None, {"Vec3f"}),
    rabbitizer.AccessType.DOUBLEFLOAT: AccessTypeInfo(8, "f64", None),
}


gKnownTypes: set[str] = {
    "asciz", "char", "char*", "String", "Char"
}

for kind in gAccessKinds.values():
    gKnownTypes |= kind.getAllTypes()


@dataclasses.dataclass
class ContextSymbol:
    address: int
    name: str|None = None
    nameEnd: str|None = None
    userDeclaredSize: int|None = None
    autodetectedSize: int|None = None
    userDeclaredType: SymbolSpecialType|str|None = None
    autodetectedType: SymbolSpecialType|str|None = None

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
    failedStringDecoding: bool = False
    _ranStringCheck: bool = False

    isMaybePascalString: bool = False
    failedPascalStringDecoding: bool = False
    _ranPascalStringCheck: bool = False

    referenceCounter: int = 0
    "How much this symbol is referenced by something else"

    referenceFunctions: set[ContextSymbol] = dataclasses.field(default_factory=set)
    "Which functions reference this symbol"
    referenceSymbols: set[ContextSymbol] = dataclasses.field(default_factory=set)
    "Which symbols reference this symbol"

    parentFunction: ContextSymbol|None = None
    "Parent function for branch labels, jump tables, and jump table labels"
    branchLabels: SortedDict[ContextSymbol] = dataclasses.field(default_factory=SortedDict)
    "For functions, the branch and jump table labels which are contained in this function"
    jumpTables: SortedDict[ContextSymbol] = dataclasses.field(default_factory=SortedDict)
    "For functions, the jump tables which are contained in this function"

    parentFileName: str|None = None
    "Name of the file containing this symbol"
    inFileOffset: int|None = None
    "Offset relative to the start of the file"

    overlayCategory: str|None = None

    nameGetCallback: Callable[[ContextSymbol], str]|None = None
    """Used to register a name of a symbol which may change in the future outside of here

    The only parameter is the ContextSymbol itself, and it should return a string containing the name of the symbol.

    Used by .getName() instead of using the setted name or the default generated name.
    """

    unknownSegment: bool = False

    isGot: bool = False
    isGotGlobal: bool = False
    isGotLocal: bool = False
    gotIndex: int|None = None

    accessedAsGpRel: bool = False

    _isStatic: bool = False

    isAutoCreatedPad: bool = False
    autoCreatedPadMainSymbol: ContextSymbol|None = None

    firstLoAccess: int|None = None

    isElfNotype: bool = False

    forceMigration: bool = False
    """Ignore rules for migrating rodata and force migration of this symbol to any
    function which references it.

    Enabling both forceMigration and forceNotMigration on the same symbol is
    undefined behaviour.
    """
    forceNotMigration: bool = False
    """Ignore rules for migrating rodata and prevent migration of this symbol to any
    function which references it.

    Enabling both forceMigration and forceNotMigration on the same symbol is
    undefined behaviour.
    """

    allowedToReferenceAddends: bool = False
    notAllowedToReferenceAddends: bool = False

    allowedToReferenceConstants: bool = False
    notAllowedToReferenceConstants: bool = False

    allowedToReferenceSymbols: bool = True
    """
    Allow or prohibit this symbol to reference other symbols.
    """

    allowedToBeReferenced: bool = True
    """
    Allow or prohibit this symbol to be referenced by other symbols.
    """

    isAutocreatedSymFromOtherSizedSym: bool = False

    isMips1Double: bool = False

    visibility: str|None = None


    @property
    def vram(self) -> int:
        return self.address

    #! @deprecated
    @property
    def size(self) -> int|None:
        return self.userDeclaredSize

    #! @deprecated
    @size.setter
    def size(self, value: int|None) -> None:
        self.userDeclaredSize = value

    #! @deprecated
    @property
    def type(self) -> SymbolSpecialType|str|None:
        return self.getTypeSpecial()

    #! @deprecated
    @type.setter
    def type(self, value: SymbolSpecialType|str|None) -> None:
        self.setTypeSpecial(value, self.isAutogenerated)

    def hasNoType(self) -> bool:
        currentType = self.getTypeSpecial()
        return (currentType is None or currentType == "") and self.accessType is None

    def hasOnlyAutodetectedType(self) -> bool:
        if self.userDeclaredType is not None and self.userDeclaredType != "":
            return False
        return (self.autodetectedType is not None and self.autodetectedType != "") or self.accessType is not None


    def isTrustableFunction(self, rsp: bool=False) -> bool:
        """Checks if the function symbol should be trusted based on the current disassembler settings"""
        if self.unknownSegment:
            return False

        if self.isGotLocal:
            return False

        if self.isAutocreatedSymFromOtherSizedSym:
            return True

        currentType = self.getTypeSpecial()

        if GlobalConfig.TRUST_USER_FUNCTIONS and self.isUserDeclared:
            if currentType == SymbolSpecialType.branchlabel:
                return False
            return True

        if GlobalConfig.TRUST_JAL_FUNCTIONS and self.isAutogenerated and currentType == SymbolSpecialType.function:
            return True

        if rsp:
            return True

        return False


    def isByte(self) -> bool:
        if not GlobalConfig.USE_DOT_BYTE:
            return False

        currentType = self.getTypeSpecial()

        # Type is checked first to favour user-declared type over the autodetected one
        if gAccessKinds[rabbitizer.AccessType.BYTE].typeMatchesAccess(currentType):
            return True
        if currentType is not None:
            return False
        if self.accessType == rabbitizer.AccessType.BYTE:
            return True
        return False

    def isShort(self) -> bool:
        if not GlobalConfig.USE_DOT_SHORT:
            return False

        currentType = self.getTypeSpecial()

        if gAccessKinds[rabbitizer.AccessType.SHORT].typeMatchesAccess(currentType):
            return True
        if currentType is not None:
            return False
        if self.accessType == rabbitizer.AccessType.SHORT:
            return True
        return False


    def isString(self) -> bool:
        if self.failedStringDecoding:
            return False

        currentType = self.getTypeSpecial()

        if self.sectionType == FileSectionType.Rodata:
            stringGuesserLevel = GlobalConfig.RODATA_STRING_GUESSER_LEVEL
        else:
            stringGuesserLevel = GlobalConfig.DATA_STRING_GUESSER_LEVEL

        if currentType in {"char", "char*", "asciz"}:
            return True
        if not self.isMaybeString:
            return False

        if stringGuesserLevel < 1:
            return False

        if self.hasNoType():
            # no type information, let's try to guess
            return True

        if self.hasOnlyAutodetectedType():
            if stringGuesserLevel >= 4:
                # There's autodetected type information, but we are going to ignore it and try to guess
                return True
        return False

    def isPascalString(self) -> bool:
        if self.failedPascalStringDecoding:
            return False

        currentType = self.getTypeSpecial()

        if self.sectionType == FileSectionType.Rodata:
            stringGuesserLevel = GlobalConfig.PASCAL_RODATA_STRING_GUESSER_LEVEL
        else:
            stringGuesserLevel = GlobalConfig.PASCAL_DATA_STRING_GUESSER_LEVEL

        if currentType in {"String", "Char"}:
            return True
        if not self.isMaybePascalString:
            return False

        if stringGuesserLevel < 1:
            return False

        if self.hasNoType():
            # no type information, let's try to guess
            return True

        if self.hasOnlyAutodetectedType():
            if stringGuesserLevel >= 4:
                # There's autodetected type information, but we are going to ignore it and try to guess
                return True
        return False

    def isFloat(self) -> bool:
        if self.vram % 4 != 0:
            return False

        currentType = self.getTypeSpecial()

        if gAccessKinds[rabbitizer.AccessType.FLOAT].typeMatchesAccess(currentType):
            return True
        if currentType is not None:
            return False
        if self.accessType == rabbitizer.AccessType.FLOAT:
            return True
        return False

    def isDouble(self) -> bool:
        if self.vram % 8 != 0:
            # Double needs to be 8 aligned
            return False

        currentType = self.getTypeSpecial()

        if gAccessKinds[rabbitizer.AccessType.DOUBLEFLOAT].typeMatchesAccess(currentType):
            return True
        if currentType is not None:
            return False
        if self.accessType == rabbitizer.AccessType.DOUBLEFLOAT:
            return True
        return False

    def isJumpTable(self) -> bool:
        return self.getTypeSpecial() == SymbolSpecialType.jumptable

    def isGccExceptTable(self) -> bool:
        return self.getTypeSpecial() == SymbolSpecialType.gccexcepttable


    def isStatic(self) -> bool:
        currentType = self.getTypeSpecial()

        if currentType == SymbolSpecialType.jumptablelabel:
            return False
        if self._isStatic:
            return True
        if self.name is None:
            return False
        return self.name.startswith(".")

    def isLateRodata(self) -> bool:
        if not GlobalConfig.COMPILER.value.hasLateRodata:
            # late rodata only exists in IDO world
            return False
        # if self.referenceCounter > 1: return False # ?
        return self.isJumpTable() or self.isFloat() or self.isDouble()


    def notPointerByType(self) -> bool:
        if self.isByte():
            return True
        if self.isShort():
            return True
        if self.isFloat():
            return True
        if self.isDouble():
            return True
        if self.isString():
            return True
        if self.isPascalString():
            return True
        return False


    def hasUserDeclaredSize(self) -> bool:
        return self.userDeclaredSize is not None

    def _defaultName_suffix(self) -> str:
        suffix = ""
        if self.overlayCategory is not None:
            suffix = "_"
            if self.vromAddress is not None:
                suffix += f"{self.vromAddress:06X}"

        if GlobalConfig.CUSTOM_SUFFIX:
            suffix += GlobalConfig.CUSTOM_SUFFIX
        return suffix

    def _defaultName_uniqueIdentifier(self, symType: SymbolSpecialType|str|None) -> str:
        if GlobalConfig.SEQUENTIAL_LABEL_NAMES and self.parentFunction is not None:
            if symType in {SymbolSpecialType.branchlabel, SymbolSpecialType.jumptablelabel}:
                index = self.parentFunction.branchLabels.index(self.vram)
                if index is not None:
                    return f"{self.parentFunction.getName()}_{index + 1}"
            elif symType == SymbolSpecialType.jumptable:
                index = self.parentFunction.jumpTables.index(self.vram)
                if index is not None:
                    return f"{self.parentFunction.getName()}_{index + 1}"

        if GlobalConfig.AUTOGENERATED_NAMES_BASED_ON_FILE_NAME:
            if self.parentFileName is not None and self.inFileOffset is not None:
                sectionName = self.sectionType.toStr().replace(".", "_")
                return f"{self.parentFileName}{sectionName}_{self.inFileOffset:06X}"

        suffix = self._defaultName_suffix()

        # Stringify the address
        if GlobalConfig.LEGACY_SYM_ADDR_ZERO_PADDING:
            return f"{self.address:06X}{suffix}"
        return f"{self.address:08X}{suffix}"

    def _defaultName_sectionPrefix(self, symType: SymbolSpecialType|str|None) -> str:
        # Functions, labels and jumptables don't get a section prefix because most of the time they are in their respective sections
        if symType in {SymbolSpecialType.function, SymbolSpecialType.branchlabel, SymbolSpecialType.jumptablelabel, SymbolSpecialType.jumptable, SymbolSpecialType.gccexcepttable, SymbolSpecialType.gccexcepttablelabel}:
            return ""

        # Determine the section type prefix
        if GlobalConfig.AUTOGENERATED_NAMES_BASED_ON_SECTION_TYPE:
            if self.sectionType == FileSectionType.Rodata:
                return "RO_"
            elif self.sectionType == FileSectionType.Bss:
                return "B_"
            elif self.sectionType == FileSectionType.Text:
                return "T_"
            elif self.sectionType == FileSectionType.Reloc:
                return "REL_"
            if self.sectionType == FileSectionType.GccExceptTable:
                return "EHTBL_"
        return "D_"

    def _defaultName_typePrefix(self, symType: SymbolSpecialType|str|None) -> str:
        if symType == SymbolSpecialType.function:
            return f"func_"
        if symType in {SymbolSpecialType.branchlabel, SymbolSpecialType.jumptablelabel}:
            return f".L"
        if symType == SymbolSpecialType.jumptable:
            return f"jtbl_"
        if symType == SymbolSpecialType.gccexcepttable:
            return f"ehtbl_"
        if symType == SymbolSpecialType.gccexcepttablelabel:
            return f"$LEH_"

        if GlobalConfig.AUTOGENERATED_NAMES_BASED_ON_DATA_TYPE:
            if self.isFloat():
                return f"FLT_"
            elif self.isDouble():
                return f"DBL_"
            elif self.isString():
                return f"STR_"
            elif self.isPascalString():
                return f"PSTR_"
        return ""

    def getDefaultName(self) -> str:
        currentType = self.getTypeSpecial()

        uniqueIdentifier = self._defaultName_uniqueIdentifier(currentType)
        sectionPrefix = self._defaultName_sectionPrefix(currentType)
        typePrefix = self._defaultName_typePrefix(currentType)
        return f"{sectionPrefix}{typePrefix}{uniqueIdentifier}"

    def getName(self) -> str:
        if self.nameGetCallback is not None:
            name = self.nameGetCallback(self)
        elif self.name is None:
            name = self.getDefaultName()
        else:
            name = self.name
        if "@" in name or "<" in name or "\\" in name or "-" in name or "+" in name:
            return f'"{name}"'
        return name

    def setNameIfUnset(self, name: str) -> bool:
        if self.name is None:
            self.name = name
            return True
        return False

    def setNameGetCallback(self, callback: Callable[[ContextSymbol], str]) -> None:
        self.nameGetCallback = callback

    def setNameGetCallbackIfUnset(self, callback: Callable[[ContextSymbol], str]) -> None:
        if self.nameGetCallback is None:
            self.nameGetCallback = callback

    def getNameEnd(self) -> str|None:
        return self.nameEnd

    def getSize(self) -> int:
        # User-declared size first
        if self.userDeclaredSize is not None:
            return self.userDeclaredSize

        if self.autodetectedSize is not None:
            return self.autodetectedSize

        currentType = self.getTypeSpecial()

        # Infer size based on user-declared type
        if currentType is not None and not isinstance(currentType, SymbolSpecialType):
            for info in gAccessKinds.values():
                if info.typeMatchesAccess(currentType):
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
        if self.userDeclaredSize is None:
            self.userDeclaredSize = size
            return True
        return False

    def getTypeSpecial(self) -> SymbolSpecialType|str|None:
        if self.userDeclaredType is not None:
            return self.userDeclaredType
        return self.autodetectedType

    def getType(self) -> str:
        currentType = self.getTypeSpecial()

        if currentType is None:
            if self.accessType is not None and self.unsignedAccessType is not None:
                typeInfo = gAccessKinds[self.accessType]
                t = typeInfo.typeUnsigned if self.unsignedAccessType else typeInfo.typeSigned
                if t is not None:
                    return t
            return ""
        if isinstance(currentType, SymbolSpecialType):
            return currentType.toStr()
        return currentType

    def setTypeSpecial(self, newType: SymbolSpecialType|str|None, isAutogenerated: bool) -> None:
        if isAutogenerated:
            self.autodetectedType = newType
        else:
            self.userDeclaredType = newType

    def setTypeIfUnset(self, newType: SymbolSpecialType|str|None, isAutogenerated: bool) -> bool:
        if self.hasNoType():
            self.setTypeSpecial(newType, isAutogenerated=isAutogenerated)
            return True
        return False

    def setAccessTypeIfUnset(self, accessType: rabbitizer.Enum, unsignedMemoryAccess: bool) -> bool:
        if self.accessType is None and self.unsignedAccessType is None:
            self.accessType = accessType
            self.unsignedAccessType = unsignedMemoryAccess
            return True
        return False

    def setFirstLoAccessIfUnset(self, loOffset: int) -> None:
        if self.firstLoAccess is None:
            self.firstLoAccess = loOffset

    def getSymbolPlusOffset(self, address: int) -> str:
        if self.address == address:
            return self.getName()
        if self.address > address:
            return f"{self.getName()} - 0x{self.address - address:X}"
        return f"{self.getName()} + 0x{address - self.address:X}"

    def getLabelMacro(self, isInMiddleLabel: bool=False) -> str|None:
        if not GlobalConfig.ASM_USE_SYMBOL_LABEL:
            return None
        label = ""
        if GlobalConfig.ASM_COMMENT:
            if self.isStatic():
                label += f"/* static variable */{GlobalConfig.LINE_ENDS}"
            if self.isAutogeneratedPad():
                mainSymbolInfo = ""
                if self.autoCreatedPadMainSymbol is not None:
                    mainSymbolInfo = f" (generated by the size of {self.autoCreatedPadMainSymbol.getName()})"
                label += f"/* Automatically generated and unreferenced pad{mainSymbolInfo} */{GlobalConfig.LINE_ENDS}"

        currentType = self.getTypeSpecial()
        if currentType == SymbolSpecialType.jumptablelabel:
            label += GlobalConfig.ASM_JTBL_LABEL
        elif currentType == SymbolSpecialType.gccexcepttablelabel:
            label += GlobalConfig.ASM_EHTBL_LABEL
        elif self.sectionType == FileSectionType.Text:
            if isInMiddleLabel:
                label += GlobalConfig.ASM_TEXT_ALT_LABEL
            else:
                label += GlobalConfig.ASM_TEXT_LABEL
        else:
            label += GlobalConfig.ASM_DATA_LABEL
        return label

    def isAutogeneratedPad(self) -> bool:
        return self.isAutoCreatedPad and self.referenceCounter == 0 and self.isAutogenerated

    def getReferenceeSymbols(self) -> str:
        if not GlobalConfig.ASM_COMMENT or not GlobalConfig.ASM_REFERENCEE_SYMBOLS:
            return ""

        if len(self.referenceFunctions):
            output = "/* Functions referencing this symbol:"
            for sym in self.referenceFunctions:
                output += f" {sym.getName()}"
            return f"{output} */{GlobalConfig.LINE_ENDS}"

        if len(self.referenceSymbols):
            output = "/* Symbols referencing this symbol:"
            for sym in self.referenceSymbols:
                output += f" {sym.getName()}"
            return f"{output} */{GlobalConfig.LINE_ENDS}"
        return ""


    @staticmethod
    def getCsvHeader() -> str:
        output = "address,name,getName,getNameEnd,"

        output += "userDeclaredType,autodetectedType,getType,"

        output += "accessType,"

        output += "userDeclaredSize,"
        output += "autodetectedSize,"
        output += "getSize,getVrom,sectionType,"

        output += "isDefined,isUserDeclared,isAutogenerated,"
        output += "isMaybeString,failedStringDecoding,isMaybePascalString,failedPascalStringDecoding,"
        output += "referenceCounter,"
        output += "parentFunction,"
        output += "parentFileName,"
        output += "inFileOffset,"
        output += "overlayCategory,unknownSegment,"
        output += "isGot,isGotGlobal,isGotLocal,gotIndex,accessedAsGpRel,"
        output += "firstLoAccess,isAutogeneratedPad,autoCreatedPadMainSymbol,isElfNotype,"
        output += "isAutocreatedSymFromOtherSizedSym,isMips1Double,visibility"
        return output

    def toCsv(self) -> str:
        output = f"0x{self.address:08X},{self.name},{self.getName()},{self.getNameEnd()},"

        output += f"{self.userDeclaredType},{self.autodetectedType},{self.getType()},"

        if self.accessType is None:
            output += "None,"
        else:
            output += f"{self.accessType.name},"

        if self.userDeclaredSize is None:
            output += "None,"
        else:
            output += f"0x{self.userDeclaredSize:X},"
        if self.autodetectedSize is None:
            output += "None,"
        else:
            output += f"0x{self.autodetectedSize:X},"

        output += f"0x{self.getSize():X},0x{self.getVrom():X},{self.sectionType.toStr()},"
        output += f"{self.isDefined},{self.isUserDeclared},{self.isAutogenerated},"
        output += f"{self.isMaybeString},{self.failedStringDecoding},{self.isMaybePascalString},{self.failedPascalStringDecoding},"
        output += f"{self.referenceCounter},"

        if self.parentFunction is not None:
            output += f"{self.parentFunction.getName()},"
        else:
            output += f"None,"
        if self.parentFileName is not None:
            output += f"{self.parentFileName},"
        else:
            output += f"None,"
        if self.inFileOffset is not None:
            output += f"{self.inFileOffset},"
        else:
            output += f"None,"

        output += f"{self.overlayCategory},{self.unknownSegment},"
        output += f"{self.isGot},{self.isGotGlobal},{self.isGotLocal},{self.gotIndex},{self.accessedAsGpRel},"
        autoCreatedPadMainSymbolName = ""
        if self.autoCreatedPadMainSymbol is not None:
            autoCreatedPadMainSymbolName = self.autoCreatedPadMainSymbol.getName()
        output += f"{self.firstLoAccess},{self.isAutogeneratedPad()},{autoCreatedPadMainSymbolName},{self.isElfNotype},"
        output += f"{self.isAutocreatedSymFromOtherSizedSym},{self.isMips1Double},{self.visibility}"
        return output


    def __eq__(self, other: object) -> bool:
        if not isinstance(other, ContextSymbol):
            return False
        return self.address == other.address and self.vromAddress == other.vromAddress

    # https://stackoverflow.com/a/56915493/6292472
    def __hash__(self) -> int:
        return hash((self.address, self.vromAddress))

    def __str__(self) -> str:
        return f"0x{self.address:08X} {self.name} ({self.getName()})"

    def __repr__(self) -> str:
        return self.__str__()
