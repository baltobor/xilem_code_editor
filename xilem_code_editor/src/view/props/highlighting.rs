// Copyright 2026 the xilem_code_editor developers
// SPDX-License-Identifier: Apache-2.0

use std::sync::Arc;

use code_editor_core::highlight::HighlightSpan;
use code_editor_core::provider::{HighlightPostPass, HighlightProvider};
use masonry_code_editor::CodeEditorWidget;
use xilem::Pod;
use xilem::core::Mut;

/// Where syntax colors come from.
#[derive(Clone, Default)]
pub(crate) struct HighlightingProps {
    /// File extension or language id passed to the highlighter.
    pub language: String,
    pub highlighter: Option<Arc<dyn HighlightProvider>>,
    pub passes: Vec<Arc<dyn HighlightPostPass>>,
    /// Precomputed per-line spans (e.g. ANSI-colored terminal output);
    /// replaces the highlighter.
    pub custom: Option<Vec<Vec<HighlightSpan>>>,
}

impl HighlightingProps {
    pub(crate) fn build(&self, widget: &mut CodeEditorWidget) {
        widget.set_language_direct(self.language.clone());
        match &self.custom {
            Some(spans) => widget.set_highlights_direct(spans.clone()),
            None => widget.set_highlighting_direct(self.highlighter.clone(), self.passes.clone()),
        }
    }

    pub(crate) fn rebuild(&self, prev: &Self, element: &mut Mut<'_, Pod<CodeEditorWidget>>) {
        if self.custom.is_some() {
            if self.custom != prev.custom {
                CodeEditorWidget::set_highlights(element, self.custom.clone().unwrap_or_default());
            }
            return;
        }
        if self.language != prev.language {
            CodeEditorWidget::set_language(element, &self.language);
        }
        if !same_provider(&self.highlighter, &prev.highlighter) || prev.custom.is_some() {
            CodeEditorWidget::set_highlighter(element, self.highlighter.clone());
        }
        if !same_passes(&self.passes, &prev.passes) {
            CodeEditorWidget::set_highlight_passes(element, self.passes.clone());
        }
    }
}

fn same_provider(
    a: &Option<Arc<dyn HighlightProvider>>,
    b: &Option<Arc<dyn HighlightProvider>>,
) -> bool {
    match (a, b) {
        (Some(a), Some(b)) => Arc::ptr_eq(a, b),
        (None, None) => true,
        _ => false,
    }
}

fn same_passes(a: &[Arc<dyn HighlightPostPass>], b: &[Arc<dyn HighlightPostPass>]) -> bool {
    a.len() == b.len() && a.iter().zip(b).all(|(a, b)| Arc::ptr_eq(a, b))
}
