// Copyright 2026 the xilem_code_editor developers
// SPDX-License-Identifier: Apache-2.0

use code_editor_core::model::HoverInfo;

use crate::CodeEditor;

impl<State, Action> CodeEditor<State, Action> {
    /// The documentation balloon (Markdown) and its scroll offset.
    pub fn quick_help(mut self, text: Option<String>, scroll: f64) -> Self {
        self.quick_help.text = text;
        self.quick_help.scroll = scroll;
        self
    }

    /// Answer to the latest `CodeEditorAction::HoverAt`: the popup appears
    /// with zed's timing (300 ms after the pointer came to rest).
    pub fn hover(mut self, info: Option<HoverInfo>) -> Self {
        self.hover.info = info;
        self
    }
}
