/* SPDX-FileCopyrightText: © 2024 Decompollaborate */
/* SPDX-License-Identifier: MIT */

use core::fmt;

#[cfg(feature = "pyo3")]
use pyo3::prelude::*;

use crate::{
    context::Context,
    metadata::segment_metadata::FindSettings,
    symbols::{Symbol, SymbolNoload},
};

use super::SymCommonDisplaySettings;

#[derive(Debug, Clone, Hash, PartialEq, Eq, PartialOrd, Ord)]
#[cfg_attr(feature = "pyo3", pyclass(module = "spimdisasm"))]
pub struct SymNoloadDisplaySettings {
    common: SymCommonDisplaySettings,
}

impl Default for SymNoloadDisplaySettings {
    fn default() -> Self {
        Self::new()
    }
}

impl SymNoloadDisplaySettings {
    pub fn new() -> Self {
        Self {
            common: SymCommonDisplaySettings::new(),
        }
    }
}

#[derive(Debug, Copy, Clone, Hash, PartialEq)]
pub struct SymNoloadDisplay<'ctx, 'sym, 'flg> {
    context: &'ctx Context,
    sym: &'sym SymbolNoload,
    settings: &'flg SymNoloadDisplaySettings,
}

impl<'ctx, 'sym, 'flg> SymNoloadDisplay<'ctx, 'sym, 'flg> {
    pub(crate) fn new(
        context: &'ctx Context,
        sym: &'sym SymbolNoload,
        settings: &'flg SymNoloadDisplaySettings,
    ) -> Self {
        Self {
            context,
            sym,
            settings,
        }
    }
}

impl fmt::Display for SymNoloadDisplay<'_, '_, '_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let owned_segment = self
            .context
            .find_owned_segment(self.sym.parent_segment_info())?;
        let find_settings = FindSettings::default().with_allow_addend(false);
        let metadata = owned_segment
            .find_symbol(self.sym.vram_range().start(), find_settings)
            .ok_or(fmt::Error)?;

        let name = metadata.display_name();
        write!(f, ".globl {}{}", name, self.settings.common.line_end())?;

        write!(f, "{}:{}", name, self.settings.common.line_end())?;

        self.settings
            .common
            .display_asm_comment(f, None, self.sym.vram_range().start(), None)?;
        write!(
            f,
            " .space 0x{:02X}{}",
            self.sym.size(),
            self.settings.common.line_end()
        )?;

        Ok(())
    }
}

#[cfg(feature = "pyo3")]
pub(crate) mod python_bindings {
    use super::*;

    #[pymethods]
    impl SymNoloadDisplaySettings {
        #[new]
        pub fn py_new() -> Self {
            Self::new()
        }
    }
}
