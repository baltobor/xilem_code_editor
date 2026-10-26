// Copyright 2026 the xilem_code_editor developers
// SPDX-License-Identifier: Apache-2.0

use std::collections::HashMap;

use code_editor_core::model::{CodeAction, Diagnostic, InlayHint};

use crate::CodeEditor;

impl<State, Action> CodeEditor<State, Action> {
    pub fn diagnostics(mut self, diagnostics: Vec<Diagnostic>) -> Self {
        self.language_data.diagnostics = diagnostics;
        self
    }

    pub fn inlay_hints(mut self, hints: Vec<InlayHint>) -> Self {
        self.language_data.inlay_hints = hints;
        self
    }

    /// Quick fixes offered inside a line's diagnostic bubble (0-based lines).
    pub fn bubble_code_actions(mut self, actions: HashMap<usize, Vec<CodeAction>>) -> Self {
        self.language_data.bubble_code_actions = actions;
        self
    }
}
