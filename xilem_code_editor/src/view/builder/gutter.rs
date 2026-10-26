// Copyright 2026 the xilem_code_editor developers
// SPDX-License-Identifier: Apache-2.0

use code_editor_core::vcs::VcsGutterDiff;

use crate::CodeEditor;
use crate::view::GutterControlFactory;

impl<State, Action> CodeEditor<State, Action> {
    /// 1-based breakpoint lines.
    pub fn breakpoints(mut self, lines: Vec<usize>) -> Self {
        self.gutter.breakpoints = lines;
        self
    }

    /// 1-based line the debugger is paused on.
    pub fn execution_line(mut self, line: Option<usize>) -> Self {
        self.gutter.execution_line = line;
        self
    }

    /// Version-control change markers (`None`: hide them).
    pub fn vcs_diff(mut self, diff: Option<VcsGutterDiff>) -> Self {
        self.gutter.vcs_enabled = diff.is_some();
        self.gutter.vcs_diff = diff;
        self
    }

    /// App-specific gutter controls (created when the editor is built).
    /// Clicks they consume arrive as `CodeEditorAction::GutterControlClicked`.
    pub fn gutter_controls(mut self, factory: GutterControlFactory) -> Self {
        self.gutter_controls.factory = Some(factory);
        self
    }
}
