// Copyright 2026 the xilem_code_editor developers
// SPDX-License-Identifier: Apache-2.0

use masonry_code_editor::CodeEditorAction;

use crate::CodeEditor;

impl<State, Action> CodeEditor<State, Action> {
    /// Handle every editor event besides text changes (cursor moves,
    /// completion and hover requests, gutter clicks, …). Return `None`
    /// when the app state needs no rebuild.
    pub fn on_event(
        mut self,
        handler: impl Fn(&mut State, CodeEditorAction) -> Option<Action> + Send + Sync + 'static,
    ) -> Self {
        self.on_event = Some(Box::new(handler));
        self
    }
}
