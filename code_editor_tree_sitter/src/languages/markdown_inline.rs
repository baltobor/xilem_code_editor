// Copyright 2026 the xilem_code_editor developers
// SPDX-License-Identifier: Apache-2.0

//! Markdown *inline* language configuration.
//!
//! tree-sitter-md ships two grammars: the block grammar (`LANGUAGE`)
//! which sees headings / lists / fenced blocks / paragraphs, and the
//! inline grammar (`INLINE_LANGUAGE`) which sees emphasis / strong /
//! code spans / links / autolinks inside paragraph text. The block
//! grammar's `injections.scm` matches every `(inline)` node and
//! requests the language id `"markdown_inline"` — this file
//! registers that id with its own `HighlightConfiguration` so the
//! injection callback in `editor::highlight` can resolve it.
//!
//! Without this, opening a normal markdown file showed no highlight
//! on paragraph text — the block grammar covers structural nodes
//! (headings, code-fence delimiters, list markers) but paragraphs
//! and their inline emphasis/code/links sit inside `(inline)` and
//! relied on the injection that wasn't wired up.

use crate::capture_category::HIGHLIGHT_NAMES;
use crate::registry::LanguageConfig;
use tree_sitter_highlight::HighlightConfiguration;

pub static MARKDOWN_INLINE: MarkdownInlineLanguage = MarkdownInlineLanguage;

pub struct MarkdownInlineLanguage;

impl LanguageConfig for MarkdownInlineLanguage {
    fn language_id(&self) -> &'static str {
        // Must match the injection.language value the block
        // grammar's injections.scm sets — see the `(inline) →
        // "markdown_inline"` rule in tree-sitter-md.
        "markdown_inline"
    }

    fn extensions(&self) -> &'static [&'static str] {
        // No standalone files — the inline grammar is reached
        // exclusively via injection from the block grammar.
        &[]
    }

    fn highlight_config(&self) -> Option<HighlightConfiguration> {
        let mut cfg = HighlightConfiguration::new(
            tree_sitter_md::INLINE_LANGUAGE.into(),
            "markdown_inline",
            tree_sitter_md::HIGHLIGHT_QUERY_INLINE,
            tree_sitter_md::INJECTION_QUERY_INLINE,
            "",
        )
        .ok()?;
        cfg.configure(HIGHLIGHT_NAMES);
        Some(cfg)
    }
}
