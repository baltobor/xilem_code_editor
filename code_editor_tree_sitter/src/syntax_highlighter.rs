// Copyright 2026 the xilem_code_editor developers
// SPDX-License-Identifier: Apache-2.0

use std::collections::HashMap;

use code_editor_core::highlight::{HighlightCategory, HighlightSpan};
use tree_sitter_highlight::{HighlightConfiguration, HighlightEvent, Highlighter};

use crate::capture_category::capture_to_category;
use crate::registry::{LanguageConfig, registry};

/// tree-sitter highlighting with a per-language configuration cache.
/// Fenced code blocks in Markdown are highlighted through injections.
pub struct SyntaxHighlighter {
    highlighter: Highlighter,
    configs: HashMap<&'static str, Option<HighlightConfiguration>>,
}

impl Default for SyntaxHighlighter {
    fn default() -> Self {
        Self::new()
    }
}

impl SyntaxHighlighter {
    pub fn new() -> Self {
        Self {
            highlighter: Highlighter::new(),
            configs: HashMap::new(),
        }
    }

    /// Highlight `source` for `language` (id or file extension). Spans use
    /// document byte ranges; unknown languages yield no spans.
    pub fn highlight(&mut self, source: &str, language: &str) -> Vec<HighlightSpan> {
        let lang = registry().resolve(language);

        #[cfg(feature = "lang-pem")]
        if lang.language_id() == crate::languages::certificate::CERTIFICATE.language_id() {
            return crate::pem::certificate_spans(source);
        }

        self.ensure_config(lang);
        for injected in lang.injection_languages() {
            if let Some(injected) = registry().from_id(injected) {
                self.ensure_config(injected);
            }
        }

        // Disjoint field borrows: the config cache is only read while the
        // highlighter runs, so no unsafe aliasing is needed.
        let Self {
            highlighter,
            configs,
        } = self;
        let Some(Some(config)) = configs.get(lang.language_id()) else {
            return Vec::new();
        };
        let resolve_injection = |name: &str| configs.get(name).and_then(Option::as_ref);
        let Ok(events) = highlighter.highlight(config, source.as_bytes(), None, resolve_injection)
        else {
            return Vec::new();
        };

        let mut spans = Vec::new();
        let mut stack: Vec<HighlightCategory> = Vec::new();
        for event in events {
            match event {
                Ok(HighlightEvent::Source { start, end }) => {
                    let category = stack.last().copied().unwrap_or(HighlightCategory::Default);
                    if category != HighlightCategory::Default && start < end {
                        spans.push(HighlightSpan {
                            byte_range: start..end,
                            category,
                        });
                    }
                }
                Ok(HighlightEvent::HighlightStart(h)) => stack.push(capture_to_category(h.0)),
                Ok(HighlightEvent::HighlightEnd) => {
                    stack.pop();
                }
                Err(_) => break,
            }
        }
        spans
    }

    fn ensure_config(&mut self, lang: &'static dyn LanguageConfig) {
        self.configs
            .entry(lang.language_id())
            .or_insert_with(|| lang.highlight_config());
    }
}
