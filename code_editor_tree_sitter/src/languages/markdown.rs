// Copyright 2026 the xilem_code_editor developers
// SPDX-License-Identifier: Apache-2.0

//! Markdown language configuration with code block injection support.

use crate::capture_category::HIGHLIGHT_NAMES;
use crate::registry::LanguageConfig;
use tree_sitter_highlight::HighlightConfiguration;

/// Markdown language configuration singleton.
pub static MARKDOWN: MarkdownLanguage = MarkdownLanguage;

/// Markdown language configuration.
pub struct MarkdownLanguage;

impl LanguageConfig for MarkdownLanguage {
    fn language_id(&self) -> &'static str {
        "markdown"
    }

    fn extensions(&self) -> &'static [&'static str] {
        &["md", "markdown"]
    }

    fn highlight_config(&self) -> Option<HighlightConfiguration> {
        // `HighlightConfiguration::new` parameters are
        // `(language, name, highlights_query, injection_query,
        // locals_query)`. Earlier this passed
        // `INJECTION_QUERY_BLOCK` in the locals slot, which
        // silently disabled injections — fenced code blocks
        // never had their inner language resolved.
        //
        // The vendored INJECTION_QUERY_BLOCK matches `(inline)`
        // nodes for the markdown_inline injection but doesn't set
        // `injection.include-children`. tree-sitter-highlight's
        // `intersect_ranges` defaults to *excluding* children of
        // the content node — and tree-sitter-md's `(inline)`
        // exposes its parsed structure as children — which left
        // the inline grammar with zero-length ranges and no
        // events ever made it back to us. The replacement below
        // appends the include-children property to that one
        // pattern; everything else (fenced code, html, yaml/toml
        // metadata) keeps its original semantics.
        let injection_query = tree_sitter_md::INJECTION_QUERY_BLOCK.replace(
            "((inline) @injection.content\n  (#set! injection.language \"markdown_inline\"))",
            "((inline) @injection.content\n  (#set! injection.language \"markdown_inline\")\n  (#set! injection.include-children))",
        );
        let mut cfg = HighlightConfiguration::new(
            tree_sitter_md::LANGUAGE.into(),
            "markdown",
            tree_sitter_md::HIGHLIGHT_QUERY_BLOCK,
            &injection_query,
            "",
        )
        .ok()?;
        cfg.configure(HIGHLIGHT_NAMES);
        Some(cfg)
    }

    fn injection_languages(&self) -> &'static [&'static str] {
        // First entry is the inline-paragraph injection — the
        // block grammar's injections.scm matches every `(inline)`
        // node and requests `markdown_inline`. Without this,
        // emphasis / strong / code-spans / link text in plain
        // paragraphs render as undecorated default text. The
        // remaining names are the languages we ship for fenced
        // code-block injection.
        &[
            "markdown_inline",
            "rust",
            "c",
            "cpp",
            "python",
            "javascript",
            "typescript",
            "json",
            "yaml",
            "toml",
            "bash",
            "sh",
            "shell",
        ]
    }

    // No LSP for Markdown
}
