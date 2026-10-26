//! WidgetMut API — editing ops (undo / redo / cut / copy / select-all)
//! plus the `perform_goto_line` navigation entry point.
//!
//! These methods are called by the View layer's rebuild logic when
//! menu commands (Edit > Undo, Edit > Cut, etc.) want to push their
//! action through to the widget.

use crate::action::CodeEditorAction;
use crate::widgets::code_editor::CodeEditorWidget;
use code_editor_core::cursor::Cursor;
use code_editor_core::selection::Selection;

impl CodeEditorWidget {
    // MARK: Undo / redo / cut / copy / select-all

    pub fn perform_undo(this: &mut masonry::core::WidgetMut<'_, Self>) {
        if let Some(regions) = this.widget.document.undo() {
            this.widget.cursor = Cursor::from_selection(Selection::from_regions(regions));
            let line_count = this.widget.document.line_count();
            this.widget.lines.resize(line_count);
            this.widget.lines.invalidate_all();
            // Re-parse: without this the tree-sitter parse tree still
            // reflects the post-edit byte ranges, so spans point at
            // the wrong characters in the rolled-back buffer.
            this.widget.rehighlight();
            let text = this.widget.document.to_string();
            this.widget.viewport.last_synced_revision = this.widget.document.revision();
            this.ctx
                .submit_action::<CodeEditorAction>(CodeEditorAction::ContentChanged(text));
            this.ctx.request_layout();
        }
    }

    /// Perform a redo operation (called from menu command).
    pub fn perform_redo(this: &mut masonry::core::WidgetMut<'_, Self>) {
        if let Some(regions) = this.widget.document.redo() {
            this.widget.cursor = Cursor::from_selection(Selection::from_regions(regions));
            let line_count = this.widget.document.line_count();
            this.widget.lines.resize(line_count);
            this.widget.lines.invalidate_all();
            this.widget.rehighlight();
            let text = this.widget.document.to_string();
            this.widget.viewport.last_synced_revision = this.widget.document.revision();
            this.ctx
                .submit_action::<CodeEditorAction>(CodeEditorAction::ContentChanged(text));
            this.ctx.request_layout();
        }
    }

    /// Perform a cut operation (called from menu command).
    pub fn perform_cut(this: &mut masonry::core::WidgetMut<'_, Self>) {
        if let Some(text) = this
            .widget
            .cursor
            .selected_text(this.widget.document.as_ref())
        {
            if !text.is_empty() {
                this.ctx.set_clipboard(text);
                // Delete the selected text
                let cursor_snapshot = this.widget.cursor.selection.regions().to_vec();
                this.widget.document.begin_edit_group(&cursor_snapshot);
                this.widget.cursor.delete(this.widget.document.as_mut());
                this.widget
                    .document
                    .end_edit_group(&this.widget.cursor.selection.regions().to_vec());
                // Update lines and notify content changed
                let line_count = this.widget.document.line_count();
                this.widget.lines.resize(line_count);
                this.widget.lines.invalidate_all();
                let content = this.widget.document.to_string();
                this.widget.viewport.last_synced_revision = this.widget.document.revision();
                this.ctx
                    .submit_action::<CodeEditorAction>(CodeEditorAction::ContentChanged(content));
                this.ctx.request_layout();
            }
        }
    }

    /// Perform a copy operation (called from menu command).
    pub fn perform_copy(this: &mut masonry::core::WidgetMut<'_, Self>) {
        if let Some(text) = this
            .widget
            .cursor
            .selected_text(this.widget.document.as_ref())
        {
            if !text.is_empty() {
                this.ctx.set_clipboard(text);
            }
        }
    }

    /// Perform a select all operation (called from menu command).
    pub fn perform_select_all(this: &mut masonry::core::WidgetMut<'_, Self>) {
        this.widget.cursor.select_all(this.widget.document.as_ref());
        this.ctx.request_paint_only();
    }

    /// Navigate to a specific line and column (called from external navigation).
    /// Line and column are 1-based. If `highlight_len` is provided, selects that many characters.
    /// `alignment` controls how the destination is placed in the viewport.
    /// Marker: Structure Click Appearance: Click shows the function at the
    /// top of the text editor — `Top` is honored here.
    // MARK: Navigation — perform_goto_line

    pub fn perform_goto_line(
        this: &mut masonry::core::WidgetMut<'_, Self>,
        line: usize,
        col: usize,
        highlight_len: Option<usize>,
        alignment: code_editor_core::model::GotoAlignment,
    ) {
        let doc = this.widget.document.as_ref();
        let line_count = doc.line_count();

        // Clamp line to valid range (1-based)
        let line = line.max(1).min(line_count);
        let line_idx = line - 1; // Convert to 0-based

        // Get the byte offset for the start of the line
        let line_start = doc.offset_of_line(line_idx);

        // Calculate the offset for the column (1-based col)
        let (byte_offset, line_str) = if let Some(line_content) = doc.line_content(line_idx) {
            let col_idx = col.saturating_sub(1); // Convert to 0-based
            let content_str = line_content.to_string();
            let mut offset = line_start;
            for (i, c) in content_str.chars().enumerate() {
                if i >= col_idx {
                    break;
                }
                offset += c.len_utf8();
            }
            (
                offset.min(line_start + line_content.len_bytes()),
                content_str,
            )
        } else {
            (line_start, String::new())
        };

        // Set the cursor position or selection
        if let Some(len) = highlight_len {
            // Calculate end offset by advancing `len` characters
            let col_idx = col.saturating_sub(1);
            let mut end_offset = byte_offset;
            for (i, c) in line_str.chars().skip(col_idx).enumerate() {
                if i >= len {
                    break;
                }
                end_offset += c.len_utf8();
            }
            this.widget
                .cursor
                .selection
                .set_single(byte_offset, end_offset);
            // Mark as search highlight for distinct coloring
            this.widget.is_search_highlight = true;
        } else {
            this.widget.cursor.selection.set_caret(byte_offset);
            this.widget.is_search_highlight = false;
        }
        // Marker: Structure Click Appearance: Click shows the function at
        // the top of the text editor — the alignment passed in here is
        // consumed by the next `scroll_to_show_rect` call.
        this.widget.pending_scroll_alignment = alignment;
        this.widget.cursor.horiz = None;

        // Reset cursor blink so it's visible immediately
        this.widget.animator.reset();

        // Request scroll to cursor position so the target line becomes visible
        this.widget.viewport.request_scroll_to_cursor();

        // Request render to show the cursor
        this.ctx.request_render();
        this.ctx.request_anim_frame();

        // Submit cursor moved action
        this.ctx
            .submit_action::<CodeEditorAction>(CodeEditorAction::CursorMoved { line, col });
        let selection = this.widget.selection_changed_action();
        this.ctx.submit_action::<CodeEditorAction>(selection);
    }

    /// Scroll the viewport so the last line is visible at the
    /// bottom edge, **without** moving the caret and **without**
    /// touching horizontal scroll. Used by the compiler-output
    /// follow-tail logic — `perform_goto_line` is the wrong
    /// tool there because it also resets the caret, which
    /// snaps horizontal scroll to the caret's column and locks
    /// the user out of scrolling horizontally to read long
    /// lines.
    pub fn perform_scroll_to_bottom_keep_horizontal(this: &mut masonry::core::WidgetMut<'_, Self>) {
        if this.widget.scroll.scroll_to_bottom_keep_horizontal() {
            this.ctx.request_render();
        }
    }
}
