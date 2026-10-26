// Copyright 2026 the xilem_code_editor developers
// SPDX-License-Identifier: Apache-2.0

use std::sync::Arc;

use code_editor_core::highlight::HighlightSpan;
use code_editor_core::provider::{HighlightPostPass, HighlightProvider};

use crate::CodeEditor;

impl<State, Action> CodeEditor<State, Action> {
    /// Language for highlighting: a file extension (`"md"`) or id.
    pub fn language(mut self, language: impl Into<String>) -> Self {
        self.highlighting.language = language.into();
        self
    }

    /// Syntax highlighting source, e.g. `code_editor_tree_sitter::TreeSitterHighlighter`.
    /// Share one `Arc` between editors.
    pub fn highlighter(mut self, highlighter: Arc<dyn HighlightProvider>) -> Self {
        self.highlighting.highlighter = Some(highlighter);
        self
    }

    /// Add a post-pass run after the highlighter (TODO markers, a DSL, …).
    ///
    /// Passes are compared by `Arc` identity: keep the `Arc` in your app
    /// state and pass the same one on every rebuild. A new `Arc` per
    /// rebuild re-runs highlighting each time.
    pub fn highlight_pass(mut self, pass: Arc<dyn HighlightPostPass>) -> Self {
        self.highlighting.passes.push(pass);
        self
    }

    /// Set all post-passes at once (see [`highlight_pass`](Self::highlight_pass)).
    pub fn highlight_passes(mut self, passes: Vec<Arc<dyn HighlightPostPass>>) -> Self {
        self.highlighting.passes = passes;
        self
    }

    /// Precomputed per-line spans instead of a highlighter (e.g. ANSI
    /// colored terminal output).
    pub fn highlights(mut self, spans: Vec<Vec<HighlightSpan>>) -> Self {
        self.highlighting.custom = Some(spans);
        self
    }
}
