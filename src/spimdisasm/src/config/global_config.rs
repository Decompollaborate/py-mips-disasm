/* SPDX-FileCopyrightText: © 2024 Decompollaborate */
/* SPDX-License-Identifier: MIT */

use super::{Endian, GpConfig};

#[derive(Debug, Copy, Clone, Hash, PartialEq, Eq)]
pub struct GlobalConfig {
    endian: Endian,
    gp_config: Option<GpConfig>,
}

impl GlobalConfig {
    pub fn new(endian: Endian) -> Self {
        Self {
            endian,
            gp_config: None,
        }
    }
}

impl GlobalConfig {
    pub const fn endian(&self) -> Endian {
        self.endian
    }
    pub fn endian_mut(&mut self) -> &mut Endian {
        &mut self.endian
    }
    pub const fn with_endian(self, endian: Endian) -> Self {
        Self { endian, ..self }
    }

    pub const fn gp_config(&self) -> Option<GpConfig> {
        self.gp_config
    }
    pub fn gp_config_mut(&mut self) -> &mut Option<GpConfig> {
        &mut self.gp_config
    }
    pub const fn with_gp_config(self, gp_config: Option<GpConfig>) -> Self {
        Self { gp_config, ..self }
    }
}
