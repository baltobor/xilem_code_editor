// Copyright 2026 the xilem_code_editor developers
// SPDX-License-Identifier: Apache-2.0

//! Plain text language configuration (no syntax highlighting).

use crate::registry::LanguageConfig;
use tree_sitter_highlight::HighlightConfiguration;

/// Plain text language configuration singleton.
pub static PLAIN_TEXT: PlainTextLanguage = PlainTextLanguage;

/// Plain text language configuration.
pub struct PlainTextLanguage;

impl LanguageConfig for PlainTextLanguage {
    fn language_id(&self) -> &'static str {
        "plaintext"
    }

    fn extensions(&self) -> &'static [&'static str] {
        &["txt"]
    }

    fn highlight_config(&self) -> Option<HighlightConfiguration> {
        // No syntax highlighting for plain text
        None
    }

    // No LSP for plain text
}
