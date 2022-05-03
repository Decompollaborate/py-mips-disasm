#!/usr/bin/env python3

# SPDX-FileCopyrightText: © 2022 Decompollaborate
# SPDX-License-Identifier: MIT

from __future__ import annotations

from ..common.Utils import *
from ..common.GlobalConfig import GlobalConfig
from ..common.Context import Context
from ..common.FileSectionType import FileSectionType

from .MipsSection import Section
from .Symbols import SymbolBss


class Bss(Section):
    def __init__(self, context: Context, bssVramStart: int, bssVramEnd: int, filename: str):
        super().__init__(context, bssVramStart, filename, bytearray(), FileSectionType.Bss)

        self.bssVramStart: int = bssVramStart
        self.bssVramEnd: int = bssVramEnd

        self.bssTotalSize: int = bssVramEnd - bssVramStart

        self.vram = bssVramStart


    def setVram(self, vram: int):
        super().setVram(vram)

        self.bssVramStart = vram
        self.bssVramEnd = vram + self.bssTotalSize

    def analyze(self):
        # Check if the very start of the file has a bss variable and create it if it doesn't exist yet
        if self.context.getSymbol(self.bssVramStart, False) is None:
            contextSym = self.context.addSymbol(self.bssVramStart, None, FileSectionType.Bss)
            contextSym.isDefined = True
            contextSym.isAutogenerated = True

        # If something that could be a pointer found in data happens to be in the middle of this bss file's addresses space
        # Then consider it a new bss variable
        for ptr in sorted(self.context.newPointersInData):
            if ptr < self.bssVramStart:
                continue
            if ptr >= self.bssVramEnd:
                break

            contextSym = self.context.getGenericSymbol(ptr)
            if contextSym is None:
                contextSym = self.context.addSymbol(ptr, None, FileSectionType.Bss)
                contextSym.isAutogenerated = True
                contextSym.isDefined = True

        # Mark every known symbol that happens to be in this address space as defined
        for vram in self.context.symbols.irange(minimum=self.bssVramStart, maximum=self.bssVramEnd, inclusive=(True, False)):
            contextSym = self.context.symbols[vram]
            contextSym.isDefined = True
            contextSym.sectionType = FileSectionType.Bss


        offsetSymbolsInSection = self.context.offsetSymbols[FileSectionType.Bss]
        bssSymbolOffsets = {offset: sym for offset, sym in offsetSymbolsInSection.items()}

        # Needs to move this to a list because the algorithm requires to check the size of a bss variable based on the next bss variable' vram
        if self.bssVramStart > 0:
            for symbolVram in self.context.symbols.irange(minimum=self.bssVramStart, maximum=self.bssVramEnd, inclusive=(True, False)):
                bssSymbolOffsets[symbolVram-self.bssVramStart] = self.context.symbols[symbolVram]

        sortedOffsets = sorted(bssSymbolOffsets.items())

        i = 0
        while i < len(sortedOffsets):
            symbolOffset, contextSym = sortedOffsets[i]
            symbolVram = self.bssVramStart + symbolOffset

            # Calculate the space of the bss variable
            space = self.bssTotalSize - symbolOffset
            if i + 1 < len(sortedOffsets):
                nextSymbolOffset, _ = sortedOffsets[i+1]
                if nextSymbolOffset <= self.bssTotalSize:
                    space = nextSymbolOffset - symbolOffset

            sym = SymbolBss(self.context, symbolOffset + self.inFileOffset, symbolVram, contextSym.name, space)
            sym.setCommentOffset(self.commentOffset)
            sym.analyze()
            self.symbolList.append(sym)

            i += 1
