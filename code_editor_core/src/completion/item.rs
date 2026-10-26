// Copyright 2026 the xilem_code_editor developers
// SPDX-License-Identifier: Apache-2.0

use crate::edit::TextEdit;

use super::CompletionKind;

/// One entry of the completion popup.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct CompletionItem {
    /// Opaque id for the provider to correlate the item with its own data.
    pub id: u64,
    pub label: String,
    pub kind: CompletionKind,
    /// Short type/signature text shown next to the label.
    pub detail: Option<String>,
    /// Markdown documentation, shown in the side panel.
    pub documentation: Option<String>,
    /// Text matched against the typed prefix; empty means `label`.
    pub filter_text: String,
    /// Text inserted at the caret when `edit` is `None`.
    pub insert_text: String,
    /// Exact replacement (typically replacing the typed prefix).
    pub edit: Option<TextEdit>,
    /// `insert_text` / `edit.new_text` uses snippet syntax (`${1:arg}`, `$0`).
    pub is_snippet: bool,
}

impl CompletionItem {
    /// The text the typed prefix is matched against.
    pub fn filter_key(&self) -> &str {
        if self.filter_text.is_empty() {
            &self.label
        } else {
            &self.filter_text
        }
    }
}
