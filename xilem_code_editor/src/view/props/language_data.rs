// Copyright 2026 the xilem_code_editor developers
// SPDX-License-Identifier: Apache-2.0

use std::collections::HashMap;

use code_editor_core::model::{CodeAction, Diagnostic, InlayHint};
use masonry_code_editor::CodeEditorWidget;
use xilem::Pod;
use xilem::core::Mut;

/// Language-server data shown in the text: diagnostics, inlay hints and
/// the quick fixes offered inside a diagnostic bubble.
#[derive(Clone, Default, PartialEq)]
pub(crate) struct LanguageDataProps {
    pub diagnostics: Vec<Diagnostic>,
    pub inlay_hints: Vec<InlayHint>,
    /// Quick fixes per 0-based line.
    pub bubble_code_actions: HashMap<usize, Vec<CodeAction>>,
}

impl LanguageDataProps {
    pub(crate) fn build(&self, widget: &mut CodeEditorWidget) {
        widget.set_overlays_direct(&self.diagnostics, &self.inlay_hints);
    }

    pub(crate) fn rebuild(&self, prev: &Self, element: &mut Mut<'_, Pod<CodeEditorWidget>>) {
        if self.diagnostics != prev.diagnostics || self.inlay_hints != prev.inlay_hints {
            CodeEditorWidget::set_overlays(element, &self.diagnostics, &self.inlay_hints);
        }
        if self.bubble_code_actions != prev.bubble_code_actions {
            CodeEditorWidget::set_bubble_code_actions(element, self.bubble_code_actions.clone());
        }
    }
}
