// Copyright 2026 the xilem_code_editor developers
// SPDX-License-Identifier: Apache-2.0

use std::ops::Range;

use code_editor_core::edit::TextEdit;
use code_editor_core::model::GotoAlignment;
use code_editor_core::snippet::TabStop;

/// A one-shot instruction from the host to the editor.
#[derive(Debug, Clone, PartialEq)]
pub enum EditorCommand {
    Undo,
    Redo,
    Cut,
    Copy,
    SelectAll,
    /// Move the caret to 1-based `line` / `col`, optionally selecting
    /// `highlight_len` bytes (search results).
    GotoLine {
        line: usize,
        col: usize,
        highlight_len: Option<usize>,
        alignment: GotoAlignment,
    },
    ScrollToBottom,
    /// Insert at the caret; `select` is relative to the insertion point.
    Insert {
        text: String,
        select: Option<(usize, usize)>,
    },
    /// Replace a byte range.
    Replace {
        range: Range<usize>,
        text: String,
    },
    /// Apply one edit (an accepted completion), with snippet tab stops.
    ApplyEdit {
        edit: TextEdit,
        cursor_offset: Option<usize>,
        tab_stops: Vec<TabStop>,
    },
    /// Apply a batch of edits as one undo step (a quick fix).
    ApplyEdits(Vec<TextEdit>),
}
