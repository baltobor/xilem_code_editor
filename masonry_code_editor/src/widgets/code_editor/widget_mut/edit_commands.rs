// Copyright 2026 the xilem_code_editor developers
// SPDX-License-Identifier: Apache-2.0

//! Programmatic edits from the host (insert, replace, completion and
//! quick-fix edits). Each is one undo step; the edit logic lives in
//! `code_editor_core::edit`, this file only refreshes the widget.

use std::ops::Range;

use code_editor_core::edit::{self, TextEdit};
use code_editor_core::snippet::TabStop;
use masonry::core::WidgetMut;

use crate::action::CodeEditorAction;
use crate::widgets::CodeEditorWidget;

impl CodeEditorWidget {
    /// Insert `text` at the caret; `select` (relative to the insertion
    /// point) selects part of it, e.g. a snippet placeholder.
    pub fn perform_insert(
        this: &mut WidgetMut<'_, Self>,
        text: &str,
        select: Option<(usize, usize)>,
    ) {
        edit::insert_at_caret(
            this.widget.document.as_mut(),
            &mut this.widget.cursor,
            text,
            select,
        );
        Self::after_programmatic_edit(this);
    }

    /// Replace the byte range `range` with `new_text`.
    pub fn perform_replace(this: &mut WidgetMut<'_, Self>, range: Range<usize>, new_text: &str) {
        if edit::replace_range(
            this.widget.document.as_mut(),
            &mut this.widget.cursor,
            range,
            new_text,
        ) {
            Self::after_programmatic_edit(this);
        }
    }

    /// Apply one edit (an accepted completion), placing the caret at
    /// `cursor_offset_in_new_text` and starting a snippet session for
    /// `tab_stops`.
    pub fn perform_text_edit(
        this: &mut WidgetMut<'_, Self>,
        text_edit: &TextEdit,
        cursor_offset_in_new_text: Option<usize>,
        tab_stops: &[TabStop],
    ) {
        let widget = &mut *this.widget;
        let session = edit::apply_text_edit(
            widget.document.as_mut(),
            &mut widget.cursor,
            text_edit,
            cursor_offset_in_new_text,
            tab_stops,
        );
        if session.is_some() {
            widget.snippet_session = session;
        }
        Self::after_programmatic_edit(this);
    }

    /// Apply a batch of edits (a quick fix) as one undo step.
    pub fn perform_text_edits(this: &mut WidgetMut<'_, Self>, edits: &[TextEdit]) {
        if edit::apply_text_edits(
            this.widget.document.as_mut(),
            &mut this.widget.cursor,
            edits,
        ) {
            Self::after_programmatic_edit(this);
        }
    }

    /// Relayout, rehighlight and report the new text to the host. The
    /// post-edit snapshot lets `set_text` ignore a rebuild that still
    /// carries the pre-edit text instead of rolling the edit back.
    fn after_programmatic_edit(this: &mut WidgetMut<'_, Self>) {
        let widget = &mut *this.widget;
        let line_count = widget.document.line_count();
        widget.lines.resize(line_count);
        widget.lines.invalidate_all();
        widget.viewport.last_synced_revision = widget.document.revision();
        widget.rehighlight();
        let content = widget.document.to_string();
        widget.pending_workspace_edit_text = Some(content.clone());
        this.ctx
            .submit_action::<CodeEditorAction>(CodeEditorAction::ContentChanged(content));
        this.ctx.request_layout();
    }
}
