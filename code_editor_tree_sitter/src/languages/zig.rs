// Copyright 2026 the xilem_code_editor developers
// SPDX-License-Identifier: Apache-2.0

//! Zig language configuration.
//!
//! Editing-only support: tree-sitter highlighting via
//! the upstream `tree-sitter-zig` grammar, no LSP wiring
//! yet (ZLS would be the natural choice when we add it).

use crate::capture_category::HIGHLIGHT_NAMES;
use crate::registry::LanguageConfig;
use tree_sitter_highlight::HighlightConfiguration;

/// Zig language configuration singleton.
pub static ZIG: ZigLanguage = ZigLanguage;

/// Zig language configuration.
pub struct ZigLanguage;

impl LanguageConfig for ZigLanguage {
    fn language_id(&self) -> &'static str {
        "zig"
    }

    fn extensions(&self) -> &'static [&'static str] {
        // `.zig` is the canonical Zig source extension; `.zon`
        // is Zig's Object Notation, which the Zig grammar parses
        // (it's a subset of the language used for build manifests
        // like `build.zig.zon`).
        &["zig", "zon"]
    }

    fn highlight_config(&self) -> Option<HighlightConfiguration> {
        let mut cfg = HighlightConfiguration::new(
            tree_sitter_zig::LANGUAGE.into(),
            "zig",
            tree_sitter_zig::HIGHLIGHTS_QUERY,
            tree_sitter_zig::INJECTIONS_QUERY,
            "",
        )
        .ok()?;
        cfg.configure(HIGHLIGHT_NAMES);
        Some(cfg)
    }
}
