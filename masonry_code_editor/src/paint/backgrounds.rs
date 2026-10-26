//! Background paint passes — column guide, gutter background,
//! active-line highlight, selection rectangles, snippet-stop
//! highlights, phantom-text backgrounds.
//!
//! These run *before* `paint_text` so the text and cursors render
//! on top.

use masonry::imaging::Painter;
use masonry::kurbo::Rect;
use masonry::peniko::Color;

use crate::properties::EditorTheme;
use crate::text_layout::Lines;
use crate::text_layout::ScreenLines;
use code_editor_core::cursor::Cursor;
use code_editor_core::document::Document;

// MARK: Static background paints (column guide, gutter, active line)

/// Paint a vertical column guide line at the specified character position.
///
/// This helps writers maintain optimal line lengths for readability.
/// `scroll_x` is the horizontal scroll offset for positioning.
pub fn paint_column_guide(
    painter: &mut Painter<'_>,
    gutter_width: f64,
    char_width: f64,
    column: usize,
    height: f64,
    scroll_x: f64,
    color: Color,
) {
    let x = gutter_width + (column as f64 * char_width) - scroll_x;
    let line_width = 1.0;

    painter.fill_rect(Rect::new(x, 0.0, x + line_width, height), color);
}

/// Paint the gutter background (the area behind line numbers).
///
/// The gutter acts as a sticky overlay - it stays fixed at x=0 and
/// covers any text that scrolls underneath it horizontally.
///
/// With internal scrolling and viewport-relative coordinates:
/// - Gutter is always at x=0 in viewport space
/// - Text content shifts left by scroll_x, but gutter stays fixed
pub fn paint_gutter(
    painter: &mut Painter<'_>,
    gutter_width: f64,
    bg_color: Color,
    _scroll_x: f64, // Kept for API compatibility but not used
    _scroll_y: f64, // Kept for API compatibility but not used
    viewport_height: f64,
) {
    if gutter_width > 0.0 {
        // Paint gutter at fixed x=0 in viewport coordinates.
        // ScreenLines now provides viewport-relative y coords, so we don't
        // need scroll offsets for positioning.
        painter.fill_rect(Rect::new(0.0, 0.0, gutter_width, viewport_height), bg_color);
    }
}

/// Paint the active line highlight (the line the cursor is on).
pub fn paint_active_line(
    painter: &mut Painter<'_>,
    cursor: &Cursor,
    doc: &dyn Document,
    lines: &Lines,
    screen: &ScreenLines,
    theme: &EditorTheme,
    viewport_width: f64,
) {
    let offset = cursor.offset();
    let cursor_line = doc.line_of_offset(offset);

    // Find this line in the screen lines
    for sl in &screen.lines {
        if sl.line == cursor_line {
            let line_height = lines
                .get_layout(sl.line)
                .map(|l| l.line_height)
                .unwrap_or(lines.line_height());

            painter.fill_rect(
                Rect::new(0.0, sl.y, viewport_width, sl.y + line_height),
                theme.active_line_bg,
            );
            break;
        }
    }
}

/// Paint all selection regions.
///
/// `scroll_x` is the horizontal scroll offset for positioning.
// MARK: Selection + snippet-stop paints

pub fn paint_selections(
    painter: &mut Painter<'_>,
    cursor: &Cursor,
    doc: &dyn Document,
    lines: &Lines,
    screen: &ScreenLines,
    selection_color: Color,
    gutter_width: f64,
    scroll_x: f64,
) {
    let x_offset = gutter_width - scroll_x;

    for region in cursor.selection.regions() {
        if region.is_caret() {
            continue;
        }

        let sel_start = region.min();
        let sel_end = region.max();

        // One rectangle per visual row: a wrapped buffer line yields
        // several, each spanning only its own row.
        for sl in &screen.lines {
            let line_start_offset = doc.offset_of_line(sl.line);
            let line_end_offset = if sl.line + 1 < doc.line_count() {
                doc.offset_of_line(sl.line + 1)
            } else {
                doc.len_bytes()
            };
            if line_end_offset <= sel_start || line_start_offset >= sel_end {
                continue;
            }

            let content_len = doc.line_content(sl.line).map_or(0, |c| c.len_bytes());
            let trailing_newline = line_end_offset - line_start_offset > content_len;
            let text_len = if trailing_newline {
                // `line_content` includes the newline; select up to it.
                content_len.saturating_sub(1)
            } else {
                content_len
            };
            let start_col = sel_start.saturating_sub(line_start_offset).min(text_len);
            let end_col = (sel_end - line_start_offset).min(text_len);
            let continues_past_line = sel_end >= line_end_offset && trailing_newline;

            let mut rects = lines.selection_rects(sl.line, start_col, end_col);
            if continues_past_line {
                // Show that the newline is selected too.
                match rects.last_mut() {
                    Some(last) => last.x1 += NEWLINE_SELECTION_WIDTH,
                    None => {
                        let caret = lines.caret_geometry(doc, line_start_offset + end_col);
                        rects.push(Rect::new(
                            caret.x,
                            caret.y,
                            caret.x + NEWLINE_SELECTION_WIDTH,
                            caret.y + caret.height,
                        ));
                    }
                }
            }
            for rect in rects {
                painter.fill_rect(
                    Rect::new(
                        rect.x0 + x_offset,
                        sl.y + rect.y0,
                        rect.x1 + x_offset,
                        sl.y + rect.y1,
                    ),
                    selection_color,
                );
            }
        }
    }
}

/// Width of the marker showing a selected line break.
const NEWLINE_SELECTION_WIDTH: f64 = 8.0;

/// Paint subtle background highlights for active snippet placeholders.
///
/// Each stop in `stops` gets a faint rectangle so the user sees what
/// they can edit; the *active* stop (`active_index` into `stops`) gets
/// a slightly stronger one. Layout/coords mirror `paint_selections`.
/// Called *before* `paint_selections` so the regular selection rectangle
/// (over the active stop) layers cleanly on top.
pub fn paint_snippet_stops(
    painter: &mut Painter<'_>,
    doc: &dyn Document,
    lines: &Lines,
    screen: &ScreenLines,
    stops: &[std::ops::Range<usize>],
    active_index: usize,
    inactive_color: Color,
    active_color: Color,
    gutter_width: f64,
    scroll_x: f64,
) {
    let x_offset = gutter_width - scroll_x;
    for (i, range) in stops.iter().enumerate() {
        let color = if i == active_index {
            active_color
        } else {
            inactive_color
        };
        let sel_start = range.start;
        let sel_end = range.end;
        if sel_start >= sel_end {
            continue;
        }
        for sl in &screen.lines {
            let line_start_offset = doc.offset_of_line(sl.line);
            let line_end_offset = if sl.line + 1 < doc.line_count() {
                doc.offset_of_line(sl.line + 1)
            } else {
                doc.len_bytes()
            };
            if line_end_offset <= sel_start || line_start_offset >= sel_end {
                continue;
            }
            let x_start = if sel_start <= line_start_offset {
                lines.offset_to_x_for_selection(doc, line_start_offset)
            } else {
                lines.offset_to_x_for_selection(doc, sel_start)
            };
            let x_end = if sel_end >= line_end_offset {
                lines.offset_to_x(doc, line_end_offset.saturating_sub(1)) + 8.0
            } else {
                lines.offset_to_x(doc, sel_end)
            };
            let line_height = lines
                .get_layout(sl.line)
                .map(|l| l.line_height)
                .unwrap_or(lines.line_height());
            painter.fill_rect(
                Rect::new(
                    x_start + x_offset,
                    sl.y,
                    x_end + x_offset,
                    sl.y + line_height,
                ),
                color,
            );
        }
    }
}

/// Paint backgrounds for phantom text (inlay hints, etc.).
///
/// Called before paint_text so backgrounds appear behind the text.
/// `scroll_x` is the horizontal scroll offset for positioning.
// MARK: Phantom-text background

pub fn paint_phantom_backgrounds(
    painter: &mut Painter<'_>,
    lines: &Lines,
    screen: &ScreenLines,
    gutter_width: f64,
    scroll_x: f64,
) {
    use masonry::parley::editing::Cursor as ParleyCursor;
    use masonry::parley::layout::Affinity;

    let x_offset = gutter_width - scroll_x;

    for sl in &screen.lines {
        let Some(layout) = lines.get_layout(sl.line) else {
            continue;
        };
        let phantoms = lines.get_phantoms(sl.line);
        if phantoms.is_empty() {
            continue;
        }

        let line_height = layout.line_height;

        // Track cumulative offset from previous phantom entries
        let mut phantom_offset = 0usize;

        for entry in phantoms.entries() {
            // Skip end-of-line phantoms (col = usize::MAX) - they don't need backgrounds
            if entry.col == usize::MAX {
                continue;
            }

            // Visual start is buffer column + offset from previous phantoms
            let visual_start = entry.col.saturating_add(phantom_offset);
            let visual_end = visual_start.saturating_add(entry.text.len());

            // Update offset for next phantom
            phantom_offset = phantom_offset.saturating_add(entry.text.len());

            let Some(bg_color) = entry.bg else {
                continue;
            };

            // Get x positions from layout
            let cursor_start =
                ParleyCursor::from_byte_index(&layout.layout, visual_start, Affinity::Downstream);
            let cursor_end =
                ParleyCursor::from_byte_index(&layout.layout, visual_end, Affinity::Upstream);

            let x_start = cursor_start.geometry(&layout.layout, 1.5).x0 as f64;
            let x_end = cursor_end.geometry(&layout.layout, 1.5).x0 as f64;

            // Draw background rectangle
            painter.fill_rect(
                Rect::new(
                    x_start + x_offset,
                    sl.y,
                    x_end + x_offset,
                    sl.y + line_height,
                ),
                bg_color,
            );
        }
    }
}
