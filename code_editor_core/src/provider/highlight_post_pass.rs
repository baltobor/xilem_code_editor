// Copyright 2026 the xilem_code_editor developers
// SPDX-License-Identifier: Apache-2.0

use crate::highlight::HighlightSpan;

/// Adjusts highlight spans after the [`HighlightProvider`](super::HighlightProvider)
/// ran — app-specific highlighting (TODO markers, an embedded DSL, …)
/// without touching the provider. Spans use document byte ranges; spans
/// appended later win where they overlap.
pub trait HighlightPostPass: Send + Sync {
    fn apply(&self, text: &str, language: &str, spans: &mut Vec<HighlightSpan>);
}
