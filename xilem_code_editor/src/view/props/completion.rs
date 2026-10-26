// Copyright 2026 the xilem_code_editor developers
// SPDX-License-Identifier: Apache-2.0

use code_editor_core::completion::CompletionItem;
use masonry_code_editor::CodeEditorWidget;
use xilem::Pod;
use xilem::core::Mut;

/// The completion popup and completion ghost text.
#[derive(Clone, Default, PartialEq)]
pub(crate) struct CompletionProps {
    pub items: Vec<CompletionItem>,
    pub selected: usize,
    pub visible: bool,
    /// The user moved the selection (Enter then accepts instead of
    /// inserting a newline).
    pub user_navigated: bool,
    /// Block-mode preview: (title, preview, index, total).
    pub block_action: Option<(String, String, usize, usize)>,
    pub ghost: Option<String>,
    /// Extra characters that open completion (besides identifiers).
    pub trigger_chars: Vec<String>,
}

impl CompletionProps {
    pub(crate) fn build(&self, widget: &mut CodeEditorWidget) {
        if !self.trigger_chars.is_empty() {
            widget.set_trigger_chars(self.trigger_chars.clone());
        }
    }

    pub(crate) fn rebuild(&self, prev: &Self, element: &mut Mut<'_, Pod<CodeEditorWidget>>) {
        if self.trigger_chars != prev.trigger_chars {
            element.widget.set_trigger_chars(self.trigger_chars.clone());
        }
        if self.ghost != prev.ghost {
            CodeEditorWidget::set_completion_ghost_text(element, self.ghost.clone());
        }
        if self.user_navigated != prev.user_navigated {
            CodeEditorWidget::set_popup_user_navigated(element, self.user_navigated);
        }
        if self.items != prev.items
            || self.selected != prev.selected
            || self.visible != prev.visible
        {
            CodeEditorWidget::set_completion_popup(
                element,
                self.items.clone(),
                self.selected,
                self.visible,
            );
        }
        if self.block_action != prev.block_action {
            CodeEditorWidget::set_completion_block_action(element, self.block_action.clone());
        }
    }
}
