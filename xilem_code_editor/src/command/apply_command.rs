// Copyright 2026 the xilem_code_editor developers
// SPDX-License-Identifier: Apache-2.0

use masonry_code_editor::CodeEditorWidget;
use xilem::Pod;
use xilem::core::Mut;

use super::EditorCommand;

/// Run `command` on the editor widget.
pub(crate) fn apply_command(element: &mut Mut<'_, Pod<CodeEditorWidget>>, command: &EditorCommand) {
    match command {
        EditorCommand::Undo => CodeEditorWidget::perform_undo(element),
        EditorCommand::Redo => CodeEditorWidget::perform_redo(element),
        EditorCommand::Cut => CodeEditorWidget::perform_cut(element),
        EditorCommand::Copy => CodeEditorWidget::perform_copy(element),
        EditorCommand::SelectAll => CodeEditorWidget::perform_select_all(element),
        EditorCommand::GotoLine {
            line,
            col,
            highlight_len,
            alignment,
        } => {
            CodeEditorWidget::perform_goto_line(element, *line, *col, *highlight_len, *alignment);
        }
        EditorCommand::ScrollToBottom => {
            CodeEditorWidget::perform_scroll_to_bottom_keep_horizontal(element)
        }
        EditorCommand::Insert { text, select } => {
            CodeEditorWidget::perform_insert(element, text, *select)
        }
        EditorCommand::Replace { range, text } => {
            CodeEditorWidget::perform_replace(element, range.clone(), text)
        }
        EditorCommand::ApplyEdit {
            edit,
            cursor_offset,
            tab_stops,
        } => {
            CodeEditorWidget::perform_text_edit(element, edit, *cursor_offset, tab_stops);
        }
        EditorCommand::ApplyEdits(edits) => CodeEditorWidget::perform_text_edits(element, edits),
    }
}

/// Whether `command` changes the text (then the stale `contents` of the
/// same rebuild must not overwrite the edit).
pub(crate) fn edits_text(command: &EditorCommand) -> bool {
    matches!(
        command,
        EditorCommand::Insert { .. }
            | EditorCommand::Replace { .. }
            | EditorCommand::ApplyEdit { .. }
            | EditorCommand::ApplyEdits(_)
            | EditorCommand::Cut
            | EditorCommand::Undo
            | EditorCommand::Redo
    )
}
