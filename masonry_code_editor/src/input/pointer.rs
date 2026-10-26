//! Mouse / pointer / scroll event router.
//!
//! Translates a raw `PointerEvent` into either a cursor mutation
//! (move, drag, word-select) or an `InputResult` flag the widget
//! interprets as a higher-level intent (open code-actions popup,
//! toggle a breakpoint).
//!
//! Word-boundary helpers used by double-click word-select live in
//! `word_helpers.rs` so the same logic is shared with
//! `key.rs::handle_key_event`'s Tab indent logic.

use masonry::core::{EventCtx, PointerButton, PointerButtonEvent, PointerEvent, PointerUpdate};

use super::InputResult;
use crate::text_layout::Lines;
use code_editor_core::cursor::word_helpers::{
    find_word_end, find_word_start, select_word_or_trailing_ws, trailing_whitespace_range,
};
use code_editor_core::cursor::{Cursor, find_matching_bracket, find_nearby_bracket};
use code_editor_core::document::Document;

// MARK: pointer math — viewport ⇄ document coordinates
//
// Layout overview (one editor pane):
//
//   ┌──── widget ─────────────────────────────────────────┐
//   │ gutter │  visible text area (a "window" into the    │
//   │  strip │  full document, sized to whatever fits)    │
//   │ ←── gutter_width ──→                                │
//   │        │ ←──── viewport ────→                       │
//   └────────┴────────────────────────────────────────────┘
//
// `pos` arrives in widget-local coordinates: pos.x = 0 at the very
// left of the widget (inside the gutter), pos.y = 0 at the top.
// `x_to_offset` and `line_at_y` expect document coordinates: x is
// the horizontal distance from the start of the text run, y is the
// absolute distance from the document top.
//
// The two transforms:
//
//   doc_x = max(0, pos.x - gutter_width) + scroll_x
//   doc_y = pos.y + scroll_y
//
// The `max(0, …)` for x floors the gutter strip at 0. A click on
// the gutter itself is dispatched upstream (breakpoint toggle,
// job-button), so this code path never sees a sub-zero text_x in
// practice — but the clamp keeps `x_to_offset` from being fed a
// nonsense negative.
//
// `+ scroll_x` is the fix for the long-standing "cursor lands to
// the left of the click when the editor is horizontally scrolled"
// bug (see Image 2 in the report). The original code computed
// only `text_x = pos.x - gutter_width`, which is the visible-on-
// screen offset. With `scroll_x = 0` that *equals* the document
// offset, so the bug was invisible until the user scrolled
// horizontally; from that point on every click was off by
// `scroll_x` pixels, which on a 14 px monospace font is about
// `scroll_x / 7` characters. The y-axis already added `scroll_y`,
// hence the asymmetry — vertical scroll didn't break clicks. The
// scroll value is read from `self.viewport_pos.x` at the call
// site in `pointer_impl.rs` and threaded through here so this
// pure function stays widget-agnostic.
fn pointer_to_doc_pos(
    pos_x: f64,
    pos_y: f64,
    gutter_width: f64,
    scroll_x: f64,
    scroll_y: f64,
) -> (f64, f64) {
    let text_x = (pos_x - gutter_width).max(0.0) + scroll_x;
    let text_y = pos_y + scroll_y;
    (text_x, text_y)
}

pub fn handle_pointer_event(
    ctx: &mut EventCtx<'_>,
    cursor: &mut Cursor,
    doc: &dyn Document,
    lines: &Lines,
    event: &PointerEvent,
    gutter_width: f64,
    scroll_x: f64,
    scroll_y: f64,
) -> InputResult {
    match event {
        PointerEvent::Down(PointerButtonEvent {
            button: None | Some(PointerButton::Primary),
            state,
            ..
        }) => {
            let pos = ctx.local_position(state.position);
            let (text_x, text_y) =
                pointer_to_doc_pos(pos.x, pos.y, gutter_width, scroll_x, scroll_y);
            let clicked_line = lines.line_at_y(text_y);

            cursor.click_count = state.count as u8;
            let shift_held = state.modifiers.shift();
            let alt_held = state.modifiers.alt();

            match state.count {
                2 => handle_double_click(doc, lines, cursor, text_x, text_y, shift_held),
                3 => handle_triple_click(doc, cursor, clicked_line),
                _ => handle_single_click(
                    doc,
                    lines,
                    cursor,
                    clicked_line,
                    text_x,
                    text_y,
                    shift_held,
                    alt_held,
                ),
            }

            cursor.horiz = None;
            cursor.column_anchor = if alt_held { cursor.column_anchor } else { None };
            ctx.request_focus();
            ctx.capture_pointer();
            // Click dismisses popups (completion, code actions, etc.)
            InputResult {
                cursor_moved: true,
                handled: true,
                dismiss_popups: true,
                clear_bracket_ghost: true,
                ..InputResult::default()
            }
        }

        PointerEvent::Move(PointerUpdate { current, .. }) => {
            if !ctx.is_active() {
                // Drag ended, clear column anchor and click count
                cursor.column_anchor = None;
                cursor.click_count = 0;
                cursor.word_select_anchor = None;
                return InputResult::nop();
            }

            let pos = ctx.local_position(current.position);
            // Same viewport→document transform as the Down arm. Drag-selection
            // has to agree with the initial click about where in the document
            // the pointer is, so we apply the identical `+ scroll_x`
            // correction here. Without this the selection endpoint would
            // diverge from the cursor by `scroll_x` pixels as soon as the
            // user dragged a selection in a scrolled view.
            let (text_x, text_y) =
                pointer_to_doc_pos(pos.x, pos.y, gutter_width, scroll_x, scroll_y);
            let drag_line = lines.line_at_y(text_y);

            if current.modifiers.alt() {
                handle_alt_drag(doc, lines, cursor, drag_line, text_x);
            } else if cursor.click_count == 2 {
                handle_word_drag(doc, lines, cursor, text_x, text_y);
            } else if cursor.click_count == 3 {
                handle_line_drag(doc, cursor, drag_line);
            } else {
                handle_normal_drag(doc, lines, cursor, text_x, text_y);
            }

            cursor.horiz = None;
            InputResult::cursor_only()
        }

        _ => InputResult::nop(),
    }
}

// MARK: click handling (Down arm)

/// Double-click: bracket matching > trailing-ws selection > word selection.
fn handle_double_click(
    doc: &dyn Document,
    lines: &Lines,
    cursor: &mut Cursor,
    text_x: f64,
    text_y: f64,
    shift_held: bool,
) {
    let offset = lines.offset_at_point(doc, text_x, text_y);
    let text = doc.text();

    // Try to find a bracket at or near the click position (forgiving)
    let Some((bracket_offset, _bracket_char)) = find_nearby_bracket(doc, offset) else {
        if let Some(ws_range) = trailing_whitespace_range(doc, offset) {
            // Click landed in (or past) the trailing whitespace of a
            // non-empty line — select all trailing whitespace up to the
            // line end. Lets the user grab it for delete or replace.
            cursor.selection.set_single(ws_range.start, ws_range.end);
            cursor.word_select_anchor = Some(ws_range.start);
        } else {
            // Normal word selection
            let word_start = find_word_start(doc, offset);
            let word_end = find_word_end(doc, offset);
            cursor.selection.set_single(word_start, word_end);
            // Store anchor for word-level drag extension
            cursor.word_select_anchor = Some(offset);
        }
        return;
    };

    let Some(matching_offset) = find_matching_bracket(doc, bracket_offset) else {
        // No matching bracket found, fall back to trailing-whitespace or
        // word selection.
        select_word_or_trailing_ws(doc, cursor, offset);
        return;
    };

    if shift_held {
        // Shift+double-click: select from bracket to matching bracket (inclusive)
        let start = bracket_offset.min(matching_offset);
        let end_char_idx = text.byte_to_char(bracket_offset.max(matching_offset));
        let end = text.char_to_byte(end_char_idx + 1);
        cursor.selection.set_single(start, end);
    } else {
        // Double-click: jump to matching bracket
        cursor.selection.set_caret(matching_offset);
    }
    cursor.word_select_anchor = None;
}

/// Triple-click: select the entire clicked line.
fn handle_triple_click(doc: &dyn Document, cursor: &mut Cursor, clicked_line: usize) {
    let line_start = doc.offset_of_line(clicked_line);
    let line_end = if clicked_line + 1 < doc.line_count() {
        doc.offset_of_line(clicked_line + 1)
    } else {
        doc.len_bytes()
    };
    cursor.selection.set_single(line_start, line_end);
    // Store anchor for line-level drag extension
    cursor.word_select_anchor = Some(line_start);
}

/// Single click: place the caret, or extend/column-select with a modifier.
fn handle_single_click(
    doc: &dyn Document,
    lines: &Lines,
    cursor: &mut Cursor,
    clicked_line: usize,
    text_x: f64,
    text_y: f64,
    shift_held: bool,
    alt_held: bool,
) {
    let offset = lines.offset_at_point(doc, text_x, text_y);

    if shift_held {
        // Shift+click: extend selection
        cursor.selection.primary_mut().end = offset;
    } else if alt_held {
        // Alt+click: set anchor for column selection at click position
        cursor.selection.set_caret(offset);
        cursor.column_anchor = Some((clicked_line, text_x));
    } else {
        // Normal click: move cursor
        cursor.selection.set_caret(offset);
    }
    cursor.word_select_anchor = None;
}

// MARK: drag handling (Move arm)

/// ALT+drag vertical (column/box) text marking.
fn handle_alt_drag(
    doc: &dyn Document,
    lines: &Lines,
    cursor: &mut Cursor,
    drag_line: usize,
    text_x: f64,
) {
    // Use anchor from Alt+click, or fall back to current cursor position
    let (anchor_line, anchor_x) = cursor.column_anchor.unwrap_or_else(|| {
        // Alt pressed during drag without Alt+click: use cursor position
        let anchor = cursor.selection.primary().start;
        let line = doc.line_of_offset(anchor);
        let x = lines.offset_to_x(doc, anchor);
        (line, x)
    });

    cursor.selection = code_editor_core::selection::Selection::column_select(
        doc,
        lines,
        anchor_line,
        drag_line,
        anchor_x,
        text_x,
    );
}

/// Word-level drag (started from a double-click): extend selection by words.
fn handle_word_drag(
    doc: &dyn Document,
    lines: &Lines,
    cursor: &mut Cursor,
    text_x: f64,
    text_y: f64,
) {
    let offset = lines.offset_at_point(doc, text_x, text_y);
    let Some(anchor) = cursor.word_select_anchor else {
        return;
    };

    let anchor_word_start = find_word_start(doc, anchor);
    let anchor_word_end = find_word_end(doc, anchor);
    let drag_word_start = find_word_start(doc, offset);
    let drag_word_end = find_word_end(doc, offset);

    // Extend selection to cover full words
    let sel_start = anchor_word_start.min(drag_word_start);
    let sel_end = anchor_word_end.max(drag_word_end);
    cursor.selection.set_single(sel_start, sel_end);
}

/// Line-level drag (started from a triple-click): extend selection by lines.
fn handle_line_drag(doc: &dyn Document, cursor: &mut Cursor, drag_line: usize) {
    let Some(anchor) = cursor.word_select_anchor else {
        return;
    };

    let anchor_line = doc.line_of_offset(anchor);
    let anchor_line_start = doc.offset_of_line(anchor_line);
    let anchor_line_end = if anchor_line + 1 < doc.line_count() {
        doc.offset_of_line(anchor_line + 1)
    } else {
        doc.len_bytes()
    };

    let drag_line_start = doc.offset_of_line(drag_line);
    let drag_line_end = if drag_line + 1 < doc.line_count() {
        doc.offset_of_line(drag_line + 1)
    } else {
        doc.len_bytes()
    };

    let sel_start = anchor_line_start.min(drag_line_start);
    let sel_end = anchor_line_end.max(drag_line_end);
    cursor.selection.set_single(sel_start, sel_end);
}

/// Normal drag: extend the selection endpoint to the pointer.
fn handle_normal_drag(
    doc: &dyn Document,
    lines: &Lines,
    cursor: &mut Cursor,
    text_x: f64,
    text_y: f64,
) {
    cursor.column_anchor = None; // Clear column anchor if not Alt+dragging
    let offset = lines.offset_at_point(doc, text_x, text_y);
    cursor.selection.primary_mut().end = offset;
}
