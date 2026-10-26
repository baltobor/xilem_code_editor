// Copyright 2026 the xilem_code_editor developers
// SPDX-License-Identifier: Apache-2.0

//! C language configuration.

use crate::capture_category::HIGHLIGHT_NAMES;
use crate::registry::LanguageConfig;
use tree_sitter_highlight::HighlightConfiguration;

/// C language configuration singleton.
pub static C: CLanguage = CLanguage;

/// C language configuration.
pub struct CLanguage;

impl LanguageConfig for CLanguage {
    fn language_id(&self) -> &'static str {
        "c"
    }

    fn extensions(&self) -> &'static [&'static str] {
        &["c", "h"]
    }

    fn highlight_config(&self) -> Option<HighlightConfiguration> {
        let mut cfg = HighlightConfiguration::new(
            tree_sitter_c::LANGUAGE.into(),
            "c",
            tree_sitter_c::HIGHLIGHT_QUERY,
            "",
            "",
        )
        .ok()?;
        cfg.configure(HIGHLIGHT_NAMES);
        Some(cfg)
    }
}
