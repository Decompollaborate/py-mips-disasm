/* SPDX-FileCopyrightText: © 2024-2025 Decompollaborate */
/* SPDX-License-Identifier: MIT */

use crate::{
    addresses::{AddressRange, Size, Vram},
    config::Compiler,
    context::Context,
    metadata::{ParentSectionMetadata, SymbolMetadata},
    parent_segment_info::ParentSegmentInfo,
    section_type::SectionType,
};

use super::{
    display::{
        InternalSymDisplSettings, SymDisplayError, SymNoloadDisplay, SymNoloadDisplaySettings,
    },
    Symbol, SymbolCreationError, SymbolPostProcessError,
};

const SECTION_TYPE: SectionType = SectionType::Bss;

#[derive(Debug, Clone, Hash, PartialEq)]
pub struct SymbolNoload {
    vram_range: AddressRange<Vram>,
    parent_segment_info: ParentSegmentInfo,
}

impl SymbolNoload {
    pub(crate) fn new(
        context: &mut Context,
        vram_range: AddressRange<Vram>,
        _in_section_offset: usize,
        parent_segment_info: ParentSegmentInfo,
        properties: SymbolNoloadProperties,
    ) -> Result<Self, SymbolCreationError> {
        let metadata = context
            .find_owned_segment_mut(&parent_segment_info)?
            .add_symbol(vram_range.start(), false)?;
        *metadata.section_type_mut() = Some(SECTION_TYPE);
        *metadata.autodetected_size_mut() = Some(vram_range.size());
        metadata.set_defined();
        metadata.set_trailing_padding_size(Size::new(0));
        metadata.set_in_overlay(parent_segment_info.overlay_category_name().is_some());

        properties.apply_to_metadata(metadata);

        Ok(Self {
            vram_range,
            parent_segment_info,
        })
    }
}

impl SymbolNoload {
    pub fn post_process(&mut self, _context: &Context) -> Result<(), SymbolPostProcessError> {
        Ok(())
    }
}

impl<'ctx, 'sym, 'flg> SymbolNoload {
    pub fn display(
        &'sym self,
        context: &'ctx Context,
        settings: &'flg SymNoloadDisplaySettings,
    ) -> Result<SymNoloadDisplay<'ctx, 'sym, 'flg>, SymDisplayError> {
        self.display_internal(context, settings, InternalSymDisplSettings::new(false))
    }

    pub(crate) fn display_internal(
        &'sym self,
        context: &'ctx Context,
        settings: &'flg SymNoloadDisplaySettings,
        internal_settings: InternalSymDisplSettings,
    ) -> Result<SymNoloadDisplay<'ctx, 'sym, 'flg>, SymDisplayError> {
        SymNoloadDisplay::new(context, self, settings, internal_settings)
    }
}

impl Symbol for SymbolNoload {
    fn vram_range(&self) -> &AddressRange<Vram> {
        &self.vram_range
    }

    fn parent_segment_info(&self) -> &ParentSegmentInfo {
        &self.parent_segment_info
    }

    #[must_use]
    fn section_type(&self) -> SectionType {
        SECTION_TYPE
    }
}

#[derive(Debug, Clone, Hash, PartialEq)]
pub(crate) struct SymbolNoloadProperties {
    pub parent_metadata: ParentSectionMetadata,
    pub compiler: Option<Compiler>,
    pub auto_pad_by: Option<Vram>,
}

impl SymbolNoloadProperties {
    fn apply_to_metadata(self, metadata: &mut SymbolMetadata) {
        metadata.set_parent_metadata(self.parent_metadata);

        if let Some(compiler) = self.compiler {
            metadata.set_compiler(compiler);
        }

        if let Some(auto_pad_by) = self.auto_pad_by {
            metadata.set_auto_created_pad_by(auto_pad_by);
        }
    }
}
