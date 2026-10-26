// Copyright 2026 the xilem_code_editor developers
// SPDX-License-Identifier: Apache-2.0

//! Just (justfile) language configuration.
//!
//! `just` is a command runner — its files are typically named
//! `Justfile` / `justfile` (no extension) or `*.just`. The
//! grammar is `tree-sitter-just`; the highlight queries here
//! are taken from the helix flavor that the crate ships, which
//! the build script gates behind a missing `queries-src/`
//! directory — so we inline the pattern strings instead of
//! importing them via `tree_sitter_just::HIGHLIGHTS_QUERY`.
//!
//! The dotted capture names (e.g. `keyword.control.import`,
//! `constant.builtin.boolean`) collapse via tree-sitter-highlight's
//! longest-prefix match against the project-wide
//! `HIGHLIGHT_NAMES` list — `keyword.*` lands on `keyword`,
//! `constant.builtin.*` on `constant.builtin`, `comment.line`
//! is already an exact entry, and so on.

use crate::capture_category::HIGHLIGHT_NAMES;
use crate::registry::LanguageConfig;
use tree_sitter_highlight::HighlightConfiguration;

pub static JUST: JustLanguage = JustLanguage;

pub struct JustLanguage;

const HIGHLIGHTS_QUERY: &str = include_str!("just_highlights.scm");

impl LanguageConfig for JustLanguage {
    fn language_id(&self) -> &'static str {
        "just"
    }

    /// File extensions this language handles.
    ///
    /// `just` is the obvious one. `justfile` is also listed —
    /// the file is conventionally named `Justfile` or `justfile`
    /// with no real extension; `EditorTab::extension()` returns
    /// the whole filename when there's no dot, so registering
    /// `justfile` as an "extension" here lets that path route
    /// to this language without a separate filename hook.
    fn extensions(&self) -> &'static [&'static str] {
        &["just", "justfile"]
    }

    fn highlight_config(&self) -> Option<HighlightConfiguration> {
        let mut cfg = HighlightConfiguration::new(
            tree_sitter_just::LANGUAGE.into(),
            "just",
            HIGHLIGHTS_QUERY,
            "",
            "",
        )
        .ok()?;
        cfg.configure(HIGHLIGHT_NAMES);
        Some(cfg)
    }
}
