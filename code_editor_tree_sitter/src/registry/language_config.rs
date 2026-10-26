// Copyright 2026 the xilem_code_editor developers
// SPDX-License-Identifier: Apache-2.0

use tree_sitter_highlight::HighlightConfiguration;

/// One language: its id, file extensions and tree-sitter configuration.
pub trait LanguageConfig: Send + Sync {
    /// Language identifier for LSP (e.g., "rust", "c", "cpp").
    fn language_id(&self) -> &'static str;

    /// File extensions this language handles (without the dot).
    fn extensions(&self) -> &'static [&'static str];

    /// Create tree-sitter highlight configuration.
    /// Returns None if highlighting is not available for this language.
    fn highlight_config(&self) -> Option<HighlightConfiguration>;

    /// Injection languages supported (for Markdown code blocks).
    /// Returns an empty slice if injections are not supported.
    fn injection_languages(&self) -> &'static [&'static str] {
        &[]
    }
}
