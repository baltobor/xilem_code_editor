// Copyright 2026 the xilem_code_editor developers
// SPDX-License-Identifier: Apache-2.0

//! YAML language configuration.

use crate::capture_category::HIGHLIGHT_NAMES;
use crate::registry::LanguageConfig;
use tree_sitter_highlight::HighlightConfiguration;

/// YAML language configuration singleton.
pub static YAML: YamlLanguage = YamlLanguage;

/// YAML language configuration.
pub struct YamlLanguage;

impl LanguageConfig for YamlLanguage {
    fn language_id(&self) -> &'static str {
        "yaml"
    }

    fn extensions(&self) -> &'static [&'static str] {
        &["yaml", "yml"]
    }

    fn highlight_config(&self) -> Option<HighlightConfiguration> {
        let mut cfg = HighlightConfiguration::new(
            tree_sitter_yaml::LANGUAGE.into(),
            "yaml",
            tree_sitter_yaml::HIGHLIGHTS_QUERY,
            "",
            "",
        )
        .ok()?;
        cfg.configure(HIGHLIGHT_NAMES);
        Some(cfg)
    }

    // No LSP for YAML by default
}
