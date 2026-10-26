// Copyright 2026 the xilem_code_editor developers
// SPDX-License-Identifier: Apache-2.0

use masonry_code_editor::CodeEditorWidget;
use xilem::Pod;
use xilem::core::Mut;

/// The documentation balloon (Markdown).
#[derive(Clone, Default, PartialEq)]
pub(crate) struct QuickHelpProps {
    pub text: Option<String>,
    pub scroll: f64,
}

impl QuickHelpProps {
    pub(crate) fn rebuild(&self, prev: &Self, element: &mut Mut<'_, Pod<CodeEditorWidget>>) {
        if self != prev {
            CodeEditorWidget::set_quick_help(element, self.text.clone(), self.scroll);
        }
    }
}
