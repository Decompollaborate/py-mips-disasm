# spimdisasm

[![PyPI - Downloads](https://img.shields.io/pypi/dm/spimdisasm)](https://pypi.org/project/spimdisasm/)
[![GitHub License](https://img.shields.io/github/license/Decompollaborate/spimdisasm)](https://github.com/Decompollaborate/spimdisasm/releases/latest)
[![GitHub release (latest SemVer)](https://img.shields.io/github/v/release/Decompollaborate/spimdisasm)](https://github.com/Decompollaborate/spimdisasm/releases/latest)
[![PyPI](https://img.shields.io/pypi/v/spimdisasm)](https://pypi.org/project/spimdisasm/)
[![GitHub contributors](https://img.shields.io/github/contributors/Decompollaborate/spimdisasm?logo=purple)](https://github.com/Decompollaborate/spimdisasm/graphs/contributors)

A matching MIPS disassembler API and front-ends with built-in instruction analysis.

Currently supports all the CPU instructions for MIPS I, II, III and IV.

Mainly focused on supporting Nintendo 64 binaries, but it should work with other
MIPS platforms too.

## Features

- Produces matching assembly.
- Supports `.text`, `.data`, `.rodata` and `.bss` disassembly.
  - The reloc section from Zelda 64 and some other games is supported too, but
    no front-end script uses it yet.
- Generates separated files for each section of a file (`.text`, `.data`,
  `.rodata` and `.bss`).
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
- Autogenerated symbols can be named after the section they come from (`RO_` and
  `B_` for `.rodata` and `.bss` sections) or its type (`STR_`, `FLT_` and `DBL_`
  for string, floats and doubles respectively).
- Simple file boundary detection.
  - Detects boundaries on .text and .rodata sections
- Lots of features can be turned on and off.
- MIPS instructions features:
  - Named registers for MIPS VR4300's coprocessors.
  - Support for many pseudoinstructions.
  - Properly handle move to/from coprocessor instructions.
  - Support for numeric, o32, n32 and n64 ABI register names.
- Some workarounds for some specific compilers/assemblers:
  - `SN64`/`PSYQ`:
    - `div`/`divu` fix: tweaks a bit the produced `div`, `divu` and `break` instructions.
- Support for specific MIPS instruction sets:
  - N64's RSP instruction disassembly support.
    - RSP decoding has been tested to build back to matching assemblies with
      [armips](https://github.com/Kingcom/armips/).
  - PS1's R3000 GTE instruction set support.
  - PSP's R4000 ALLEGREX instruction set support.
  - PS2's R5900 EE instruction set support.
- (Experimental) Same VRAM overlay support.
  - Overlays which are able to reference symbols from other overlays in other
    categories/types is supported too.
  - NOTE: This feature lacks lots of testing and probably has many bugs.

## Installing

The recommended way to install is using from the PyPi release, via `pip`:

```bash
python3 -m pip install -U spimdisasm
```

If you use a `requirements.txt` file in your repository, then you can add
this library with the following line:

```txt
spimdisasm>=1.32.0,<2.0.0
```

### Development version

The unstable development version is located at the [develop](https://github.com/Decompollaborate/spimdisasm/tree/develop)
branch. PRs should be made into that branch instead of the main one.

The recommended way to install a locally cloned repo is by passing the `-e`
(editable) flag to `pip`.

```bash
python3 -m pip install -e .
```

In case you want to mess with the latest development version without wanting to
clone the repository, then you could use the following command:

```bash
python3 -m pip uninstall spimdisasm
python3 -m pip install git+https://github.com/Decompollaborate/spimdisasm.git@develop
```

NOTE: Installing the development version is not recommended unless you know what
you are doing. Proceed at your own risk.

## Versioning and changelog

This library follows [Semantic Versioning](https://semver.org/spec/v2.0.0.html).
We try to always keep backwards compatibility, so no breaking changes should
happen until a major release (i.e. jumping from 1.X.X to 2.0.0).

To see what changed on each release check either the [CHANGELOG.md](CHANGELOG.md)
file or check the [releases page on Github](https://github.com/Decompollaborate/spimdisasm/releases).
You can also use [this link](https://github.com/Decompollaborate/spimdisasm/releases/latest)
to check the latest release.

## How to use

This repo can be used either by using the existing front-end scripts or by
creating new programs on top of the back-end API.

### Front-end

Every front-end CLI tool has its own `--help` screen.

The included tool can be executed with either `spimdisasm modulename` (for
example `spimdisasm disasmdis --help`) or directly `modulename` (for example
`spimdisasm --help`)

- `singleFileDisasm`: Allows to disassemble a single binary file, producing
  matching assembly files.

- `disasmdis`: Disassembles raw hex passed to the CLI as a MIPS instruction.

- `elfObjDisasm`: \[EXPERIMENTAL\] Allows to disassemble elf files. Generated
  assembly files are not guaranteed to match or even be assemblable.

- `rspDisasm`: Disassemblies RSP binaries.

### Back-end

TODO

Check the already existing front-ends is recommended for now.

## References

- <https://techpubs.jurassic.nl/manuals/0620/developer/Cplr_PTG/sgi_html/apa.html>
- <http://www.iquebrew.org/index.php?title=IO>
