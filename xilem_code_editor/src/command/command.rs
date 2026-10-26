// Copyright 2026 the xilem_code_editor developers
// SPDX-License-Identifier: Apache-2.0

use super::EditorCommand;

/// An [`EditorCommand`] with an id. The editor runs each id once, so the
/// host can keep passing the same command without it repeating; issue a
/// new command with a higher id.
#[derive(Debug, Clone, PartialEq)]
pub struct Command {
    pub id: u64,
    pub command: EditorCommand,
}

impl Command {
    pub fn new(id: u64, command: EditorCommand) -> Self {
        Self { id, command }
    }
}
