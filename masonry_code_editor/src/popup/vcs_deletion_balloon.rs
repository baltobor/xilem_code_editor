//! Removed-text balloon for the VCS gutter.
//!
//! When the user clicks a beige deletion marker in the gutter, this paints a
//! small bordered popup showing the lines that were removed (the HEAD version),
//! anchored at the marker, with an **Insert** button that re-inserts them.
//! (It re-inserts, rather than truly reverting: for a modified line it adds the
//! old text back rather than replacing the new line.) Plain monospace — no
//! markdown. Returns the balloon rect and the Insert button rect so the widget
//! can hit-test clicks (inside the button inserts, inside the balloon keeps it
//! open, outside closes it).

use masonry::core::{BrushIndex, PaintCtx, render_text};
use masonry::imaging::Painter;
use masonry::kurbo::{Affine, Rect, RoundedRect, Stroke};
use masonry::parley;
use masonry::peniko::Brush;
use masonry::peniko::Color;

const MAX_WIDTH: f64 = 600.0;
const PAD: f64 = 8.0;
const RADIUS: f64 = 6.0;
const FONT_SIZE: f32 = 12.0;
/// Cap the previewed removed lines so the balloon stays bounded; the full text
/// is still inserted on click (the widget holds it).
const MAX_LINES: usize = 12;
const BTN_H: f64 = 22.0;
const BTN_PAD_X: f64 = 12.0;
const BTN_GAP: f64 = 8.0;

/// Result of painting the balloon: rects for hit-testing.
pub struct VcsBalloonPaint {
    pub balloon_rect: Rect,
    pub insert_rect: Rect,
}

/// Where the balloon anchors, in widget-local coordinates, and the viewport
/// bounds it must stay clear of.
pub struct VcsBalloonAnchor {
    pub x: f64,
    /// The marker's boundary y — the balloon prefers to open above it,
    /// flipping below when there isn't room.
    pub y: f64,
    pub viewport_width: f64,
    pub viewport_height: f64,
}

/// Theme colours the balloon paints with.
pub struct VcsBalloonColors {
    pub bg: Color,
    pub border: Color,
    pub text: Color,
    pub accent_bg: Color,
}

/// Paint the removed-text balloon anchored at the deletion marker.
/// Returns the balloon + Insert-button rects (`Rect::ZERO` when nothing was
/// painted).
pub fn paint_vcs_deletion_balloon(
    ctx: &mut PaintCtx<'_>,
    painter: &mut Painter<'_>,
    removed_text: &str,
    anchor: VcsBalloonAnchor,
    colors: VcsBalloonColors,
) -> VcsBalloonPaint {
    let zero = VcsBalloonPaint {
        balloon_rect: Rect::ZERO,
        insert_rect: Rect::ZERO,
    };
    if removed_text.is_empty() {
        return zero;
    }

    let removed_text_preview = truncate_to_max_lines(removed_text);
    let content_width = MAX_WIDTH - PAD * 2.0;
    let removed_text_layout = build_removed_text_layout(ctx, &removed_text_preview, content_width);
    let insert_label_layout = build_insert_label_layout(ctx);

    let button_width = insert_label_layout.width() as f64 + BTN_PAD_X * 2.0;
    let balloon_size = balloon_size(&removed_text_layout, button_width);
    let balloon_origin = balloon_origin(&anchor, balloon_size);

    let balloon_rect = Rect::from_origin_size(balloon_origin, balloon_size);
    paint_balloon_background(painter, balloon_rect, &colors);

    let text_brushes = [Brush::Solid(colors.text)];
    render_text(
        painter,
        Affine::translate((balloon_rect.x0 + PAD, balloon_rect.y0 + PAD)),
        &removed_text_layout,
        &text_brushes,
        false,
    );

    let insert_rect = paint_insert_button(
        painter,
        balloon_rect,
        button_width,
        &insert_label_layout,
        &text_brushes,
        &colors,
    );

    VcsBalloonPaint {
        balloon_rect,
        insert_rect,
    }
}

/// Cap the previewed removed lines to `MAX_LINES` so the balloon (and its
/// button) stay sized; the full text is still inserted on click (the widget
/// holds it, not this preview).
fn truncate_to_max_lines(removed_text: &str) -> String {
    let mut preview: String = removed_text
        .lines()
        .take(MAX_LINES)
        .collect::<Vec<_>>()
        .join("\n");
    if removed_text.lines().count() > MAX_LINES {
        preview.push_str("\n…");
    }
    preview
}

fn build_removed_text_layout(
    ctx: &mut PaintCtx<'_>,
    text: &str,
    content_width: f64,
) -> parley::Layout<BrushIndex> {
    let (font_ctx, layout_ctx) = ctx.text_contexts();
    let mut builder = layout_ctx.ranged_builder(font_ctx, text, 1.0, true);
    builder.push_default(parley::StyleProperty::FontSize(FONT_SIZE));
    builder.push_default(parley::StyleProperty::FontFamily(
        parley::FontFamily::Single(parley::FontFamilyName::Generic(
            parley::GenericFamily::Monospace,
        )),
    ));
    builder.push_default(parley::StyleProperty::LineHeight(
        parley::LineHeight::FontSizeRelative(1.2),
    ));
    builder.push_default(parley::StyleProperty::Brush(BrushIndex(0)));
    let mut layout = builder.build(text);
    layout.break_all_lines(Some(content_width as f32));
    layout
}

fn build_insert_label_layout(ctx: &mut PaintCtx<'_>) -> parley::Layout<BrushIndex> {
    let (font_ctx, layout_ctx) = ctx.text_contexts();
    let mut builder = layout_ctx.ranged_builder(font_ctx, "Insert", 1.0, true);
    builder.push_default(parley::StyleProperty::FontSize(FONT_SIZE));
    builder.push_default(parley::StyleProperty::Brush(BrushIndex(0)));
    let mut layout = builder.build("Insert");
    layout.break_all_lines(None);
    layout
}

fn balloon_size(
    removed_text_layout: &parley::Layout<BrushIndex>,
    button_width: f64,
) -> masonry::kurbo::Size {
    let text_width = removed_text_layout.width() as f64;
    let text_height = removed_text_layout.height() as f64;
    let width = (text_width.max(button_width) + PAD * 2.0).clamp(140.0, MAX_WIDTH);
    let height = PAD + text_height + BTN_GAP + BTN_H + PAD;
    masonry::kurbo::Size::new(width, height)
}

/// Anchor right of the gutter; prefer opening above the marker, flip below
/// when there isn't room, and keep the balloon inside the viewport.
fn balloon_origin(
    anchor: &VcsBalloonAnchor,
    balloon_size: masonry::kurbo::Size,
) -> masonry::kurbo::Point {
    const GAP: f64 = 2.0;
    let x = anchor
        .x
        .min(anchor.viewport_width - balloon_size.width - 4.0)
        .max(4.0);
    let y = if anchor.y - GAP >= balloon_size.height {
        anchor.y - GAP - balloon_size.height
    } else {
        (anchor.y + GAP)
            .min(anchor.viewport_height - balloon_size.height - 4.0)
            .max(4.0)
    };
    masonry::kurbo::Point::new(x, y)
}

fn paint_balloon_background(
    painter: &mut Painter<'_>,
    balloon_rect: Rect,
    colors: &VcsBalloonColors,
) {
    let rounded = RoundedRect::from_rect(balloon_rect, RADIUS);
    painter.fill(rounded, colors.bg).draw();
    painter
        .stroke(rounded, &Stroke::new(1.0), colors.border)
        .draw();
}

/// Paint the "Insert" button bottom-right of the balloon and return its rect
/// for hit-testing.
fn paint_insert_button(
    painter: &mut Painter<'_>,
    balloon_rect: Rect,
    button_width: f64,
    label_layout: &parley::Layout<BrushIndex>,
    label_brushes: &[Brush],
    colors: &VcsBalloonColors,
) -> Rect {
    let label_width = label_layout.width() as f64;
    let label_height = label_layout.height() as f64;
    let bx = balloon_rect.x1 - PAD - button_width;
    let by = balloon_rect.y1 - PAD - BTN_H;
    let insert_rect = Rect::new(bx, by, bx + button_width, by + BTN_H);

    let btn_rounded = RoundedRect::from_rect(insert_rect, 4.0);
    painter.fill(btn_rounded, colors.accent_bg).draw();
    painter
        .stroke(btn_rounded, &Stroke::new(1.0), colors.border)
        .draw();
    render_text(
        painter,
        Affine::translate((
            bx + (button_width - label_width) / 2.0,
            by + (BTN_H - label_height) / 2.0,
        )),
        label_layout,
        label_brushes,
        false,
    );

    insert_rect
}
