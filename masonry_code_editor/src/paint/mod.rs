//! All static rendering logic for the code editor.
//!
//! Extracted from `widget.rs` to keep rendering concerns separate
//! from event handling and state management. The widget calls these
//! functions from its `paint_impl` method.
//!
//! ## What lives here vs in `editor/overlays/`
//!
//! `painting/` holds the *static* render passes — anything that
//! produces pixels but does not double as a click target:
//!
//! - column guide, gutter background, active-line highlight
//!   (`backgrounds.rs`)
//! - selections + snippet-stop highlights (`backgrounds.rs`)
//! - phantom-text backgrounds (`backgrounds.rs`)
//! - text + cursors (this `mod.rs`)
//! - gutter overlays — diagnostic markers, breakpoints, section
//!   markers, line numbers (`gutter_overlays.rs`)
//! - scrollbars (`scrollbars.rs`)
//! - ghost text — completion preview (this `mod.rs`)
//!
//! The two *interactive* overlays — Xcode-style diagnostic bubbles
//! and the caret-anchored completion popup — live in
//! `editor/overlays/` because they emit click-target bounds the
//! widget hit-tests. Re-exports at the bottom of this file keep
//! `crate::paint::paint_inline_diagnostic_bubbles` and
//! `crate::paint::paint_completion_popup` callable for legacy callers.

mod backgrounds;
mod gutter_overlays;
mod scrollbars;

pub use backgrounds::{
    paint_active_line, paint_column_guide, paint_gutter, paint_phantom_backgrounds,
    paint_selections, paint_snippet_stops,
};
pub use gutter_overlays::{paint_popup_focus_glyph, paint_section_markers};
pub use scrollbars::{
    SCROLLBAR_MARGIN, horizontal_scrollbar_layout, paint_scrollbars, vertical_scrollbar_layout,
};

use masonry::core::{BrushIndex, PaintCtx, render_text};
use masonry::imaging::Painter;
use masonry::kurbo::{Affine, Rect};
use masonry::parley;
use masonry::peniko::Brush;
use masonry::peniko::Color;

use crate::text_layout::Lines;
use crate::text_layout::ScreenLines;
use code_editor_core::cursor::Cursor;
use code_editor_core::document::Document;

// MARK: Text + cursors

pub fn paint_text(
    painter: &mut Painter<'_>,
    lines: &Lines,
    screen: &ScreenLines,
    brushes: &[Brush],
    gutter_width: f64,
    scroll_x: f64,
    hint: bool,
) {
    let x_offset = gutter_width - scroll_x;
    for sl in &screen.lines {
        if let Some(layout) = lines.get_layout(sl.line) {
            render_text(
                painter,
                Affine::translate((x_offset, sl.y)),
                &layout.layout,
                brushes,
                hint,
            );
        }
    }
}

/// Paint cursor carets for all cursor positions.
///
/// `scroll_x` is the horizontal scroll offset for positioning.
pub fn paint_cursors(
    painter: &mut Painter<'_>,
    cursor: &Cursor,
    doc: &dyn Document,
    lines: &Lines,
    screen: &ScreenLines,
    caret_color: Color,
    gutter_width: f64,
    scroll_x: f64,
    cursor_visible: bool,
) {
    if !cursor_visible {
        return;
    }

    let x_offset = gutter_width - scroll_x;
    let caret_width = 1.5;

    for region in cursor.selection.regions() {
        let offset = region.end;
        let cursor_line = doc.line_of_offset(offset);

        // Only paint if the cursor's line is visible
        for sl in &screen.lines {
            if sl.line == cursor_line {
                // Row-aware: on a wrapped line the caret sits on its own row.
                let caret = lines.caret_geometry(doc, offset);
                let x = caret.x + x_offset;
                let y = sl.y + caret.y;
                painter.fill_rect(
                    Rect::new(x, y, x + caret_width, y + caret.height),
                    caret_color,
                );
                break;
            }
        }
    }
}

/// Paint diagnostic markers (colored vertical bars) in the gutter for visible lines.

// MARK: Ghost text (completion preview)

pub fn paint_ghost_text(
    ctx: &mut PaintCtx<'_>,
    painter: &mut Painter<'_>,
    cursor: &Cursor,
    doc: &dyn Document,
    lines: &Lines,
    screen: &ScreenLines,
    ghost_text: &str,
    gutter_width: f64,
    scroll_x: f64,
) {
    if ghost_text.is_empty() {
        return;
    }

    let offset = cursor.offset();
    let cursor_line = doc.line_of_offset(offset);

    // Find this line in the screen lines
    let Some(sl) = screen.lines.iter().find(|sl| sl.line == cursor_line) else {
        return;
    };

    let x_offset = gutter_width - scroll_x;
    let x = lines.offset_to_x(doc, offset) + x_offset;

    // Build text layout for ghost text
    let (fctx, lctx) = ctx.text_contexts();
    let mut builder = lctx.ranged_builder(fctx, ghost_text, 1.0, true);
    builder.push_default(parley::StyleProperty::FontSize(lines.font_size()));
    builder.push_default(parley::StyleProperty::FontFamily(
        parley::FontFamily::Single(parley::FontFamilyName::Generic(
            parley::GenericFamily::Monospace,
        )),
    ));
    builder.push_default(parley::StyleProperty::Brush(BrushIndex(0)));
    let mut text_layout = builder.build(ghost_text);
    text_layout.break_all_lines(None);

    // Ghost text color: faded gray
    let ghost_color = Color::from_rgba8(150, 150, 150, 160);

    render_text(
        painter,
        Affine::translate((x, sl.y)),
        &text_layout,
        &[ghost_color.into()],
        true,
    );
}

// MARK: Interactive overlay re-exports
//
// The bubble + completion popup rendering moved into `editor/overlays/`
// in the SOLID refactor. Re-exporting them here keeps existing callers
// (notably `widget.rs::paint`) working through the legacy
// `crate::paint::paint_inline_diagnostic_bubbles` /
// `crate::paint::paint_completion_popup` paths without churning every
// reference.

pub use crate::popup::{
    VcsBalloonAnchor, VcsBalloonColors, paint_completion_block_popup, paint_completion_popup,
    paint_inline_diagnostic_bubbles, paint_quick_help_balloon, paint_vcs_deletion_balloon,
};
