// Copyright 2026 the xilem_code_editor developers
// SPDX-License-Identifier: Apache-2.0

use crate::highlight::HighlightSpan;

/// Turns source text into highlight spans.
///
/// Synchronous: highlighting runs on every edit and must be cheap enough
/// to do inline (tree-sitter is). Spans use **document** byte ranges; the
/// editor runs any [`HighlightPostPass`](super::HighlightPostPass) and
/// then splits them into lines with
/// [`split_into_lines`](crate::highlight::split_into_lines).
pub trait HighlightProvider: Send + Sync {
    /// Whether `language` (a file extension or language id) is
    /// highlighted by this provider.
    fn supports(&self, language: &str) -> bool;

    fn highlight(&self, text: &str, language: &str) -> Vec<HighlightSpan>;
}
