/* SPDX-FileCopyrightText: © 2024 Decompollaborate */
/* SPDX-License-Identifier: MIT */

use core::{fmt, hash::Hash};

// use alloc::boxed::Box;
use alloc::string::String;
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

#[derive(Clone)]
#[allow(dead_code)]
pub struct SymbolMetadata {
    generated_by: GeneratedBy,
    vram: Vram,
    rom: Option<RomAddress>,

    user_declared_name: Option<String>,
    user_declared_name_end: Option<String>,

    // TODO: Is this still necessary?
    /// Used to register a name of a symbol which may change in the future.
    ///
    /// The only parameter is the ContextSymbol itself, and it should return a string containing the name of the symbol.
    ///
    /// Used by .get_name() instead of using the setted name or the default generated name.
    // name_get_callback: Option<Box<dyn FnOnce(&SymbolMetadata) -> String>>,
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

            user_declared_name: None,
            user_declared_name_end: None,

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
            // name_get_callback: None,
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
    pub(crate) fn rom_mut(&mut self) -> &mut Option<RomAddress> {
        &mut self.rom
    }

    /*
    pub fn autogenerate_name(&self) -> String {
        "I haven't implemented this yet, shorry".into()
    }
    pub fn name(&self) -> String {
        escape_name(if let Some(user_declared_name) = &self.user_declared_name {
            user_declared_name.clone()
        } else {
            self.autogenerate_name()
        })
    }
    */
    pub fn display_name(&self) -> SymbolMetadataNameDisplay {
        SymbolMetadataNameDisplay { sym: self }
    }

    pub fn user_declared_name_mut(&mut self) -> &mut Option<String> {
        &mut self.user_declared_name
    }

    pub fn user_declared_name_end_mut(&mut self) -> &mut Option<String> {
        &mut self.user_declared_name_end
    }

    pub fn user_declared_size(&self) -> Option<Size> {
        self.user_declared_size
    }
    pub fn user_declared_size_mut(&mut self) -> &mut Option<Size> {
        &mut self.user_declared_size
    }
    pub fn autodetected_size(&self) -> Option<Size> {
        self.autodetected_size
    }
    pub(crate) fn autodetected_size_mut(&mut self) -> &mut Option<Size> {
        &mut self.autodetected_size
    }
    pub fn size(&self) -> Option<Size> {
        // TODO

        if let Some(siz) = self.user_declared_size {
            return Some(siz);
        }
        if let Some(siz) = self.autodetected_size {
            return Some(siz);
        }

        // TODO: Infer size based on user-declared type

        // TODO: Infer size based on instruction access type

        None
    }

    pub fn sym_type(&self) -> Option<&SymbolType> {
        if let Some(t) = &self.user_declared_type {
            Some(t)
        } else {
            self.autodetected_type.as_ref()
        }
    }
    pub fn user_declared_type(&self) -> Option<&SymbolType> {
        self.user_declared_type.as_ref()
    }
    pub fn user_declared_type_mut(&mut self) -> &mut Option<SymbolType> {
        &mut self.user_declared_type
    }
    pub fn autodetected_type(&self) -> Option<&SymbolType> {
        self.autodetected_type.as_ref()
    }
    pub(crate) fn set_type(&mut self, new_type: SymbolType, generated_by: GeneratedBy) {
        match generated_by {
            GeneratedBy::Autogenerated => self.autodetected_type = Some(new_type),
            GeneratedBy::UserDeclared => self.user_declared_type = Some(new_type),
        }
    }

    pub fn section_type(&self) -> Option<SectionType> {
        self.section_type
    }
    pub(crate) fn section_type_mut(&mut self) -> &mut Option<SectionType> {
        &mut self.section_type
    }

    pub fn is_defined(&self) -> bool {
        self.is_defined
    }
    pub(crate) fn set_defined(&mut self) {
        self.is_defined = true;
    }

    pub fn rodata_migration_behavior_mut(&mut self) -> &mut RodataMigrationBehavior {
        &mut self.rodata_migration_behavior
    }

    pub fn set_dont_allow_addend(&mut self) {
        // TODO: actually do something
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
impl Hash for SymbolMetadata {
    fn hash<H: core::hash::Hasher>(&self, state: &mut H) {
        self.vram.hash(state);
        self.rom.hash(state);
    }
}

impl fmt::Debug for SymbolMetadata {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "SymbolMetadata {{ vram: 0x{}, name: \"{}\" }}",
            self.vram,
            self.display_name()
        )
    }
}

fn should_escape_symbol(name: &str) -> bool {
    name.contains('@')
}

#[derive(Debug, Copy, Clone, Hash, PartialEq, PartialOrd)]
pub struct SymbolMetadataNameDisplay<'sym> {
    sym: &'sym SymbolMetadata,
}

impl SymbolMetadataNameDisplay<'_> {
    fn display_section_prefix(&self, _f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // TODO
        Ok(())
    }

    fn display_type_prefix(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.sym.sym_type() {
            Some(SymbolType::Function) => write!(f, "func_"),
            Some(SymbolType::BranchLabel) | Some(SymbolType::JumptableLabel) => write!(f, ".L"),
            Some(SymbolType::Jumptable) => write!(f, "jtbl_"),
            Some(SymbolType::GccExceptTable) => write!(f, "ehtbl_"),
            Some(SymbolType::GccExceptTableLabel) => write!(f, "$LEH_"),
            Some(SymbolType::UserDeclared(_)) | None => Ok(()),
        }
    }

    fn display_unique_identifier(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // TODO: logic for types and such
        write!(f, "{}", self.sym.vram)
    }

    pub fn autogenerate_name(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.display_section_prefix(f)?;
        self.display_type_prefix(f)?;
        self.display_unique_identifier(f)
    }
}

impl fmt::Display for SymbolMetadataNameDisplay<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(user_declared_name) = &self.sym.user_declared_name {
            let should_escape = should_escape_symbol(user_declared_name);

            if should_escape {
                write!(f, "\"")?;
            }
            write!(f, "{}", user_declared_name)?;
            if should_escape {
                write!(f, "\"")?;
            }
            Ok(())
        } else {
            self.autogenerate_name(f)
        }
    }
}
