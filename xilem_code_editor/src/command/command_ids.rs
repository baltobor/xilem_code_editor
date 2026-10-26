// Copyright 2026 the xilem_code_editor developers
// SPDX-License-Identifier: Apache-2.0

use super::{Command, EditorCommand};

/// Issues increasing command ids. Keep one in your app state and wrap
/// each one-shot request with [`issue`](Self::issue); the editor runs
/// every id once.
#[derive(Debug, Default, Clone, Copy)]
pub struct CommandIds(u64);

impl CommandIds {
    pub fn issue(&mut self, command: EditorCommand) -> Command {
        self.0 += 1;
        Command::new(self.0, command)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ids_increase() {
        let mut ids = CommandIds::default();
        let a = ids.issue(EditorCommand::Undo);
        let b = ids.issue(EditorCommand::Redo);
        assert!(b.id > a.id);
    }
}
