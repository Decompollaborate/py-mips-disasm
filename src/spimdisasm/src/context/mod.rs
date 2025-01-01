/* SPDX-FileCopyrightText: © 2024-2025 Decompollaborate */
/* SPDX-License-Identifier: MIT */

pub mod builder;
mod the_context;

pub use builder::ContextBuilder;
pub use the_context::{Context, OwnedSegmentNotFoundError};
