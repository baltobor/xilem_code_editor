// Copyright 2026 the xilem_code_editor developers
// SPDX-License-Identifier: Apache-2.0

//! A code editor for [Xilem] apps, from a plain Markdown editor to a full
//! IDE editor.
//!
//! ```ignore
//! code_editor(state.text.clone(), |state: &mut State, text| state.text = text)
//!     .language("md")
//!     .word_wrap(true)
//! ```
//!
//! Everything beyond text changes arrives through [`CodeEditor::on_event`];
//! one-shot instructions (undo, goto line, apply edits) go in through
//! [`CodeEditor::command`].
//!
//! [Xilem]: https://github.com/linebender/xilem

mod command;
mod view;

pub use code_editor_core as core;
pub use command::{Command, CommandIds, EditorCommand};
pub use masonry_code_editor as widget;
pub use masonry_code_editor::{CodeEditorAction, EditorFontFamily, EditorTheme};
pub use view::{CodeEditor, CodeEditorViewState, GutterControlFactory, code_editor};
