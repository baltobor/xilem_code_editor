// Copyright 2026 the xilem_code_editor developers
// SPDX-License-Identifier: Apache-2.0

use code_editor_core::vcs::VcsGutterDiff;
use masonry_code_editor::CodeEditorWidget;
use xilem::Pod;
use xilem::core::Mut;

/// Debugger and version-control markers in the gutter.
#[derive(Clone, Default, PartialEq)]
pub(crate) struct GutterProps {
    /// 1-based breakpoint lines.
    pub breakpoints: Vec<usize>,
    /// 1-based line the debugger is paused on.
    pub execution_line: Option<usize>,
    pub vcs_enabled: bool,
    pub vcs_diff: Option<VcsGutterDiff>,
}

impl GutterProps {
    pub(crate) fn build(&self, widget: &mut CodeEditorWidget) {
        widget.set_breakpoints_direct(self.breakpoints.clone());
        widget.set_execution_line_direct(self.execution_line);
        widget.set_vcs_gutter_enabled_direct(self.vcs_enabled);
        widget.set_vcs_diff_direct(self.vcs_diff.clone());
    }

    pub(crate) fn rebuild(&self, prev: &Self, element: &mut Mut<'_, Pod<CodeEditorWidget>>) {
        if self.breakpoints != prev.breakpoints {
            CodeEditorWidget::set_breakpoints(element, self.breakpoints.clone());
        }
        if self.execution_line != prev.execution_line {
            CodeEditorWidget::set_execution_line(element, self.execution_line);
        }
        if self.vcs_enabled != prev.vcs_enabled {
            CodeEditorWidget::set_vcs_gutter_enabled(element, self.vcs_enabled);
        }
        if self.vcs_diff != prev.vcs_diff {
            CodeEditorWidget::set_vcs_diff(element, self.vcs_diff.clone());
        }
    }
}
