#!/usr/bin/env python3

# SPDX-FileCopyrightText: © 2022 Decompollaborate
# SPDX-License-Identifier: MIT

from __future__ import annotations

from ...common.Utils import *
from ...common.GlobalConfig import GlobalConfig
from ...common.Context import Context, ContextSymbol, ContextOffsetSymbol
from ...common.FileSectionType import FileSectionType

from .MipsSymbolBase import SymbolBase


class SymbolText(SymbolBase):
    def __init__(self, context: Context, inFileOffset: int, vram: int|None, name: str, words: list[int]):
        super().__init__(context, inFileOffset, vram, name, words, FileSectionType.Text)
