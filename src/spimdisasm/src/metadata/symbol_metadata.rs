/* SPDX-FileCopyrightText: © 2024 Decompollaborate */
/* SPDX-License-Identifier: MIT */

use core::fmt;

use alloc::{boxed::Box, string::String};
use rabbitizer::{access_type::AccessType, Vram};

use crate::{rom_address::RomAddress, section_type::SectionType, size::Size};

use super::SymbolType;

#[derive(Debug, Copy, Clone, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub /*(crate)*/ enum GeneratedBy {
    /// This symbol was automatically generated by the disassembler
    Autogenerated,
    /// Declared externally by the user, but it may have not been found yet
    UserDeclared,
}

#[derive(Debug, Copy, Clone, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct StringInfo {
    is_maybe_string: bool,
    failed_string_decoding: bool,
}

#[derive(Debug, Copy, Clone, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct GotInfo {
    is_got: bool, // TODO: maybe redundant?
    is_got_global: bool,
    is_got_local: bool,
    got_index: Option<usize>, // TODO: maybe remove Option?
}

#[derive(Debug, Clone, Hash, PartialEq, Eq, PartialOrd, Ord, Default)]
#[non_exhaustive]
pub enum RodataMigrationBehavior {
    /// Let spimdisasm handle if it should migrate this rodata symbol.
    #[default]
    Default,

    /// Ignore rules for migrating rodata and force migration of this symbol to any
    /// function which references it.
    ForceMigrate,
    /// Ignore rules for migrating rodata and prevent migration of this symbol to
    /// any function which references it.
    ForceNotMigrate,

    /// Force migrating to the function that matches the specified name.
    ///
    /// Overrides all logic for determining if this symbol should be migrated or
    /// not and to which function should be migrated.
    ///
    /// This can be specially useful for unreferenced symbols that should be
    /// defined in-between actually referenced symbols.
    ///
    /// This field is ignored if applied on anything that is not a rodata symbol.
    ///
    /// WARNING: It is undefined behavior if during rodata migration the listed
    /// function does not exists on the given text section. For example this symbol
    /// may get lost in limbo.
    MigrateToSpecificFunction(String),
}

#[allow(dead_code)]
pub struct SymbolMetadata {
    generated_by: GeneratedBy,
    vram: Vram,
    rom: Option<RomAddress>,

    name: Option<String>,
    name_end: Option<String>,

    // TODO: Is this still necessary?
    /// Used to register a name of a symbol which may change in the future.
    ///
    /// The only parameter is the ContextSymbol itself, and it should return a string containing the name of the symbol.
    ///
    /// Used by .get_name() instead of using the setted name or the default generated name.
    name_get_callback: Option<Box<dyn FnOnce(&SymbolMetadata) -> String>>,

    user_declared_size: Option<Size>,
    autodetected_size: Option<Size>,

    user_declared_type: Option<SymbolType>,
    autodetected_type: Option<SymbolType>,

    section_type: Option<SectionType>,

    /// This symbol exists (was found) in any of the analyzed sections
    is_defined: bool,

    access_type: Option<(AccessType, bool)>,

    c_string_info: Option<StringInfo>,
    pascal_string_info: Option<StringInfo>,

    /// How much this symbol is referenced by something else
    reference_counter: usize,

    // TODO: how to reimplement these crossreferences?
    // Which functions reference this symbol
    // reference_functions: BTreeSet<>,
    // Which symbols reference this symbol
    // reference_symbols: BTreeSet<>,

    // parentFunction: ContextSymbol|None = None
    // "Parent function for branch labels, jump tables, and jump table labels"
    // branchLabels: SortedDict[ContextSymbol] = dataclasses.field(default_factory=SortedDict)
    // "For functions, the branch and jump table labels which are contained in this function"
    // jumpTables: SortedDict[ContextSymbol] = dataclasses.field(default_factory=SortedDict)
    // "For functions, the jump tables which are contained in this function"

    // parentFileName: str|None = None
    // "Name of the file containing this symbol"
    // inFileOffset: int|None = None
    // "Offset relative to the start of the file"

    // TODO: is this even necessary?
    // overlayCategory: str|None = None

    // unknownSegment: bool = False

    //
    got_info: Option<GotInfo>,

    accessed_as_gp_rel: bool,

    // _isStatic: bool = False

    // TODO: These two are kinda redundant
    // isAutoCreatedPad: bool = False
    // autoCreatedPadMainSymbol: ContextSymbol|None = None

    // firstLoAccess: int|None = None

    // isElfNotype: bool = False

    //
    rodata_migration_behavior: RodataMigrationBehavior,

    /*
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
    */
    //
    is_autocreated_sym_from_other_sized_sym: bool,

    is_mips1_double: bool,

    visibility: Option<String>,
}

impl SymbolMetadata {
    pub(crate) fn new(generated_by: GeneratedBy, vram: Vram) -> Self {
        Self {
            generated_by,
            vram,
            rom: None,

            name: None,
            name_end: None,

            user_declared_size: None,
            autodetected_size: None,
            user_declared_type: None,
            autodetected_type: None,

            section_type: None,

            is_defined: false,

            access_type: None,
            c_string_info: None,
            pascal_string_info: None,
            reference_counter: 0,
            name_get_callback: None,
            got_info: None,
            accessed_as_gp_rel: false,
            rodata_migration_behavior: RodataMigrationBehavior::Default,
            is_autocreated_sym_from_other_sized_sym: false,
            is_mips1_double: false,
            visibility: None,
        }
    }

    pub fn generated_by(&self) -> GeneratedBy {
        self.generated_by
    }

    pub const fn vram(&self) -> Vram {
        self.vram
    }

    pub fn rom(&self) -> Option<RomAddress> {
        self.rom
    }
    pub fn update_rom(&mut self, new_rom: Option<RomAddress>) {
        if self.rom.is_none() {
            self.rom = new_rom;
        }
    }

    pub fn user_declared_size(&self) -> Option<Size> {
        self.user_declared_size
    }
    pub fn autodetected_size(&self) -> Option<Size> {
        self.autodetected_size
    }
    pub fn set_autodetected_size(&mut self, size: Size) {
        self.autodetected_size = Some(size);
    }
    pub fn size(&self) -> Size {
        // TODO

        if let Some(siz) = self.user_declared_size {
            return siz;
        }
        if let Some(siz) = self.autodetected_size {
            return siz;
        }

        // TODO: Infer size based on user-declared type

        // TODO: Infer size based on instruction access type

        // Infer size based on symbol's address alignment
        if self.vram.inner() % 4 == 0 {
            Size::new(4)
        } else if self.vram.inner() % 2 == 0 {
            Size::new(2)
        } else {
            Size::new(1)
        }
    }

    pub fn sym_type(&self) -> Option<&SymbolType> {
        if let Some(t) = &self.user_declared_type {
            Some(t)
        } else {
            self.autodetected_type.as_ref()
        }
    }
    pub fn set_type(&mut self, new_type: Option<SymbolType>, generated_by: GeneratedBy) {
        match generated_by {
            GeneratedBy::Autogenerated => self.autodetected_type = new_type,
            GeneratedBy::UserDeclared => self.user_declared_type = new_type,
        }
    }

    pub fn section_type(&self) -> Option<SectionType> {
        self.section_type
    }
    pub fn update_section_type(&mut self, new_section_type: Option<SectionType>) {
        if self.section_type.is_none() {
            self.section_type = new_section_type;
        }
    }

    pub(crate) fn set_autocreated_from_other_sized_sym(&mut self) {
        self.is_autocreated_sym_from_other_sized_sym = true;
    }
}

impl SymbolMetadata {
    pub fn is_trustable_function(&self) -> bool {
        // TODO
        true
    }
}

impl PartialEq for SymbolMetadata {
    fn eq(&self, other: &Self) -> bool {
        self.vram == other.vram && self.rom == other.rom
    }
}
impl PartialOrd for SymbolMetadata {
    fn partial_cmp(&self, other: &Self) -> Option<core::cmp::Ordering> {
        match self.vram.partial_cmp(&other.vram) {
            Some(core::cmp::Ordering::Equal) => {}
            ord => return ord,
        };
        self.rom.partial_cmp(&other.rom)
    }
}

impl fmt::Debug for SymbolMetadata {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "SymbolMetadata {{ vram: 0x{}, name: {:?} }}",
            self.vram, self.name
        )
    }
}
