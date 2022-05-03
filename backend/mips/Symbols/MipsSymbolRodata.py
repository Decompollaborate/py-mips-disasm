#!/usr/bin/env python3

# SPDX-FileCopyrightText: © 2022 Decompollaborate
# SPDX-License-Identifier: MIT

from __future__ import annotations

from ...common.Utils import *
from ...common.GlobalConfig import GlobalConfig
from ...common.Context import Context, ContextSymbol, ContextOffsetSymbol
from ...common.FileSectionType import FileSectionType

from .MipsSymbolBase import SymbolBase


class SymbolRodata(SymbolBase):
    def __init__(self, context: Context, inFileOffset: int, vram: int|None, name: str, words: list[int]):
        super().__init__(context, inFileOffset, vram, name, words, FileSectionType.Rodata)


    def isFloat(self, type: str, index: int) -> bool:
        if type in ("f32", "Vec3f"):
            # Filter out NaN and infinity
            if (self.words[index] & 0x7F800000) != 0x7F800000:
                return True
        return False

    def isDouble(self, type: str, index: int) -> bool:
        if type == "f64":
            if index + 1 < self.sizew:
                word0 = self.words[index]
                word1 = self.words[index+1]
                # Filter out NaN and infinity
                if (((word0 << 32) | word1) & 0x7FF0000000000000) != 0x7FF0000000000000:
                    # Prevent accidentally losing symbols
                    currentVram = self.getVramOffset(index*4)
                    if self.context.getGenericSymbol(currentVram+4, False) is None:
                        return True
        return False

    def isString(self, type: str) -> bool:
        if type == "char":
            return True
        elif type == "": # no type information, let's try to guess
            if GlobalConfig.STRING_GUESSER:
                if self.contextSym is not None:
                    if self.contextSym.isMaybeString:
                        return True
        return False


    def renameBasedOnType(self):
        if not GlobalConfig.AUTOGENERATED_NAMES_BASED_ON_DATA_TYPE:
            return

        if self.vram is None:
            return

        if self.contextSym is not None:
            if not self.contextSym.isAutogenerated:
                return

            symbolType = self.contextSym.getType()
            if symbolType != "@jumptable":
                if self.isFloat(symbolType, 0):
                    self.contextSym.name = f"FLT_{self.vram:08X}"
                elif self.isDouble(symbolType, 0):
                    self.contextSym.name = f"DBL_{self.vram:08X}"
                elif self.isString(symbolType):
                    self.contextSym.name = f"STR_{self.vram:08X}"
            self.name = self.contextSym.name

    def analyze(self):
        if self.contextSym is not None:
            if self.contextSym.getType() in ("f32", "Vec3f", "f64"):
                self.contextSym.isLateRodata = True

        super().analyze()

    def getNthWord(self, i: int) -> Tuple[str, int]:
        localOffset = 4*i
        inFileOffset = self.inFileOffset
        w = self.words[i]

        label = ""
        rodataWord = w
        value: Any = toHex(w, 8)

        # try to get the symbol name from the offset of the file (possibly from a .o elf file)
        possibleSymbolName = self.context.getOffsetGenericSymbol(inFileOffset, FileSectionType.Rodata)
        if possibleSymbolName is not None:
            label = possibleSymbolName.getSymbolLabel() + "\n"

        possibleReference = self.context.getRelocSymbol(inFileOffset, FileSectionType.Rodata)
        if possibleReference is not None:
            value = possibleReference.getNamePlusOffset(w)
            if possibleReference.type == "@jumptablelabel":
                if w in self.context.offsetJumpTablesLabels:
                    value = self.context.offsetJumpTablesLabels[w].name

        dotType = ".word"
        skip = 0
        symbolType = ""

        if self.contextSym is not None:
            symbolType = self.contextSym.getType()

        if self.isFloat(symbolType, i):
            dotType = ".float"
            value = wordToFloat(w)
        elif self.isDouble(symbolType, i):
            dotType = ".double"
            otherHalf = self.words[i+1]
            value = qwordToDouble((w << 32) | otherHalf)
            rodataWord = (w << 32) | otherHalf
            skip = 1
        elif w in self.context.jumpTablesLabels:
            value = self.context.jumpTablesLabels[w].name
        elif self.isString(symbolType):
            try:
                buffer = bytearray(4*len(self.words))
                beWordsToBytes(self.words, buffer)
                decodedValue, rawStringSize = decodeString(buffer, 4*i)
                dotType = ".asciz"
                value = f'"{decodedValue}"'
                value += "\n" + (22 * " ") + ".balign 4"
                rodataWord = None
                skip = rawStringSize // 4
            except (UnicodeDecodeError, RuntimeError):
                # Not a string
                pass

        comment = self.generateAsmLineComment(localOffset, rodataWord)
        return f"{label}{comment} {dotType} {value}", skip


    def disassembleAsRodata(self) -> str:
        output = self.getLabel()

        i = 0
        while i < len(self.words):
            data, skip = self.getNthWord(i)
            output += data + "\n"

            i += skip
            i += 1
        return output

    def disassemble(self) -> str:
        return self.disassembleAsRodata()
