// Copyright 2026 the xilem_code_editor developers
// SPDX-License-Identifier: Apache-2.0

/// The host's answer to a pointer hover: documentation for the word at the
/// 1-based `line`/`col` it was asked about.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HoverInfo {
    pub line: usize,
    pub col: usize,
    /// Markdown; `None` when there is nothing to show (the popup stays closed).
    pub markdown: Option<String>,
}
