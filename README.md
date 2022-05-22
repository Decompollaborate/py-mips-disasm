# spimdisasm

A matching MIPS disassembler API and front-ends with built-in instruction analysis.

Currently supports all the CPU instructions for MIPS I, II, III and IV.

Mainly focused on supporting Nintendo 64 binaries, but it should work with other MIPS platforms too.

## Features

- Produces matching assembly.
- Supports `.text`, `.data`, `.rodata` and `.bss` disassembly.
  - The reloc section from Zelda 64 and some other games is supported too, but no front-end script uses it yet.
- Generates separated files for each section of a file (`.text`, `.data`, `.rodata` and `.bss`).
- Supports multiple files spliting from a single input binary.
- Automatic function detection.
  - Can detect if a function is handwritten too.
- `hi`/`lo` pairing with high success rate.
- Automatic pointer and symbol detection.
- Function spliting with rodata migration.
- Supports floats and doubles in rodata.
- String detection with medium to high success rate.
- Allows to set user-defined function and symbol names.
- Big, little and middle endian support.
- Autogenerated symbols can be named after the section they come from (`R_` and `B_` for `.rodata` and `.bss` sections) or its type (`STR_`, `FLT_` and `DBL_` for string, floats and doubles respectively).
- Simple file boundary detection.
  - Detects boundaries on .text and .rodata sections
- Lots of features can be turned on and off.
- MIPS instructions features:
  - Named registers for MIPS VR4300's coprocessors.
  - Support for many pseudoinstructions.
  - Properly handle move to/from coprocessor instructions.
  - Support for numeric, o32, n32 and n64 ABI register names.
- Some workarounds for some specific compilers/assemblers:
  - `SN64`:
    - `div`/`divu` fix: tweaks a bit the produced `div`, `divu` and `break` instructions.
- (Experimental) N64 RSP disassembly support.
  - NOTE: This mode has not been tested to even be assemblable.
- (Experimental) Same VRAM overlay support.
  - Overlays which are able to reference symbols from other overlays in other categories/types is supported too.
  - NOTE: This feature lacks lots of testing and probably has many bugs.

## How to use

This repo can be used either by using the existing front-end scripts or by creating new programs on top of the back-end API.

### Front-end

Every front-end script has its own `--help` screen.

- `singleFileDisasm.py`: Allows to disassemble a single binary file, producing matching assembly files.

- `disasmdis.py`: Disassembles raw hex passed to the CLI as a MIPS instruction.

- `elfObjDisasm.py`: \[EXPERIMENTAL\] Allows to disassemble `.o` elf files. Generated assembly files are not guaranteed to match or be assemblable.

### Back-end

TODO

Check `spimdisasm/__main__.py` for a minimal disassembly working example on how to use the API. Checking the front-ends is recommended too.
