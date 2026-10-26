// Copyright 2026 the xilem_code_editor developers
// SPDX-License-Identifier: Apache-2.0

/// A quick fix / refactoring offered for a position.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct CodeAction {
    /// Opaque id for the provider to correlate with its own data.
    pub id: u64,
    pub title: String,
    pub is_preferred: bool,
}
