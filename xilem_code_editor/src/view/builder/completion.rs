// Copyright 2026 the xilem_code_editor developers
// SPDX-License-Identifier: Apache-2.0

use code_editor_core::completion::CompletionItem;

use crate::CodeEditor;

impl<State, Action> CodeEditor<State, Action> {
    /// The completion popup: items, selected index, visibility.
    pub fn completion(
        mut self,
        items: Vec<CompletionItem>,
        selected: usize,
        visible: bool,
    ) -> Self {
        self.completion.items = items;
        self.completion.selected = selected;
        self.completion.visible = visible;
        self
    }

    /// The user navigated the popup (Enter accepts instead of a newline).
    pub fn completion_user_navigated(mut self, navigated: bool) -> Self {
        self.completion.user_navigated = navigated;
        self
    }

    /// Block-mode preview: (title, preview, index, total).
    pub fn completion_block_action(
        mut self,
        action: Option<(String, String, usize, usize)>,
    ) -> Self {
        self.completion.block_action = action;
        self
    }

    /// Ghost text previewing the selected completion.
    pub fn completion_ghost(mut self, ghost: Option<String>) -> Self {
        self.completion.ghost = ghost;
        self
    }

    /// What opens completion besides identifier characters: single
    /// characters (`"."`; brackets are ignored as too noisy) and
    /// multi-character sequences that fire once the text before the
    /// caret ends with them (`"[["` for wiki links, `"::"`).
    pub fn completion_trigger_chars(mut self, chars: Vec<String>) -> Self {
        self.completion.trigger_chars = chars;
        self
    }
}
