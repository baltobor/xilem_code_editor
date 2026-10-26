// Copyright 2026 the xilem_code_editor developers
// SPDX-License-Identifier: Apache-2.0

use std::ops::Range;

/// The signature of the call around the caret.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SignatureHelp {
    /// Full signature, e.g. `fn push(&mut self, value: T)`.
    pub label: String,
    /// Byte range of the active parameter within `label`, emphasized.
    pub active_parameter: Option<Range<usize>>,
    /// Markdown documentation.
    pub documentation: Option<String>,
}
