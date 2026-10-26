// Copyright 2026 the xilem_code_editor developers
// SPDX-License-Identifier: Apache-2.0

/// Information shown in the hover popover.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Hover {
    /// Markdown; fenced code blocks are syntax highlighted.
    pub markdown: String,
}
