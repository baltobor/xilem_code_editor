// Copyright 2026 the xilem_code_editor developers
// SPDX-License-Identifier: Apache-2.0

use code_editor_core::model::HoverInfo;
use masonry_code_editor::CodeEditorWidget;
use xilem::Pod;
use xilem::core::Mut;

/// The host's answer to the latest `HoverAt` request.
#[derive(Clone, Default, PartialEq)]
pub(crate) struct HoverProps {
    pub info: Option<HoverInfo>,
}

impl HoverProps {
    pub(crate) fn rebuild(&self, prev: &Self, element: &mut Mut<'_, Pod<CodeEditorWidget>>) {
        if self.info != prev.info {
            CodeEditorWidget::set_hover(element, self.info.clone());
        }
    }
}
