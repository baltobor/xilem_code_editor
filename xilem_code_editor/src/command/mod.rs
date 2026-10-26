// Copyright 2026 the xilem_code_editor developers
// SPDX-License-Identifier: Apache-2.0

//! One-shot commands from the host (undo, goto line, insert, apply edits).

mod apply_command;
mod command;
mod command_ids;
mod editor_command;

pub(crate) use apply_command::{apply_command, edits_text};
pub use command::Command;
pub use command_ids::CommandIds;
pub use editor_command::EditorCommand;
