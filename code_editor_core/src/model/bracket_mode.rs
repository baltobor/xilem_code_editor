// Copyright 2026 the xilem_code_editor developers
// SPDX-License-Identifier: Apache-2.0

/// How the editor helps with closing brackets and quotes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum BracketMode {
    /// Show the closing bracket as ghost text; Tab or typing it accepts.
    #[default]
    Ghost,
    /// Insert the closing bracket immediately.
    AutoClose,
}
