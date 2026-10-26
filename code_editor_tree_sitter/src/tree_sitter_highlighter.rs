// Copyright 2026 the xilem_code_editor developers
// SPDX-License-Identifier: Apache-2.0

use std::sync::Mutex;

use code_editor_core::highlight::HighlightSpan;
use code_editor_core::provider::HighlightProvider;

use crate::SyntaxHighlighter;
use crate::languages::plain::PLAIN_TEXT;
use crate::registry::{LanguageConfig, registry};

/// [`HighlightProvider`] backed by the compiled-in tree-sitter grammars.
/// Shareable between editors (`Arc<TreeSitterHighlighter>`).
#[derive(Default)]
pub struct TreeSitterHighlighter {
    inner: Mutex<SyntaxHighlighter>,
}

impl TreeSitterHighlighter {
    pub fn new() -> Self {
        Self::default()
    }
}

impl HighlightProvider for TreeSitterHighlighter {
    fn supports(&self, language: &str) -> bool {
        registry().resolve(language).language_id() != PLAIN_TEXT.language_id()
    }

    fn highlight(&self, text: &str, language: &str) -> Vec<HighlightSpan> {
        match self.inner.lock() {
            Ok(mut highlighter) => highlighter.highlight(text, language),
            Err(_) => Vec::new(),
        }
    }
}
