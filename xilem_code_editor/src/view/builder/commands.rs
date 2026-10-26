// Copyright 2026 the xilem_code_editor developers
// SPDX-License-Identifier: Apache-2.0

use crate::{CodeEditor, Command};

impl<State, Action> CodeEditor<State, Action> {
    /// Run `command` once (repeated ids are ignored).
    pub fn command(mut self, command: Option<Command>) -> Self {
        self.commands.extend(command);
        self
    }

    /// Run several commands, in order, each once.
    pub fn commands(mut self, commands: impl IntoIterator<Item = Command>) -> Self {
        self.commands.extend(commands);
        self
    }
}
