// Copyright 2026 the xilem_code_editor developers
// SPDX-License-Identifier: Apache-2.0

use std::ops::Range;

use super::TextPosition;

/// Replace `range` with `new_text` — the neutral form of an LSP
/// `TextEdit`, so the editor doesn't depend on `lsp-types`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextEdit {
    pub range: Range<TextPosition>,
    pub new_text: String,
}

impl TextEdit {
    pub fn new(range: Range<TextPosition>, new_text: impl Into<String>) -> Self {
        Self {
            range,
            new_text: new_text.into(),
        }
    }

    /// The edit's byte range in `text`, or `None` when it is inverted.
    pub fn byte_range(&self, text: &str) -> Option<Range<usize>> {
        let start = self.range.start.to_byte_offset(text);
        let end = self.range.end.to_byte_offset(text);
        (start <= end).then_some(start..end)
    }
}
