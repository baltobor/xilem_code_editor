//! Gutter overlay paint passes — diagnostic markers (squiggle bars),
//! breakpoint dots, section-marker rules, and line numbers.
//!
//! These run *after* `paint_text` so the gutter overlays sit on top
//! of any text that scrolled under the gutter horizontally.

use masonry::core::{BrushIndex, PaintCtx, render_text};
use masonry::imaging::Painter;
use masonry::kurbo::{Affine, Rect};
use masonry::parley;
use masonry::peniko::Brush;
use masonry::peniko::Color;

use crate::properties::EditorTheme;
use crate::text_layout::Lines;
use crate::text_layout::ScreenLines;

// MARK: Gutter overlays — diagnostic markers, breakpoints, section markers, line numbers

/// Paint the popup return-armed glyph (`⏎`, U+23CE) on the
/// caret line. Drawn in the gutter as a discreet "Return is
/// armed" indicator: the user has navigated the popup with
/// Up/Down at least once, so Return will accept the
/// highlighted item (otherwise Return inserts a newline).
///
/// Spec: `src/app_model/lsp/popup_mode/mod.rs` —
/// "Visual indicator: gutter return-glyph when Return is
/// armed."
///
/// `caret_line` is the 0-based document line. The glyph paints
/// only when the caret is on a visible screen line. Color is
/// the editor's accent color at ~70% alpha so the glyph reads
/// as a hint, not an attention grab.
pub fn paint_popup_focus_glyph(
    ctx: &mut PaintCtx<'_>,
    painter: &mut Painter<'_>,
    screen: &ScreenLines,
    lines: &Lines,
    caret_line: usize,
    gutter_width: f64,
    scroll_x: f64,
    accent: Color,
) {
    let Some(sl) = screen.lines.iter().find(|sl| sl.line == caret_line) else {
        return;
    };
    let line_height = lines
        .get_layout(sl.line)
        .map(|l| l.line_height)
        .unwrap_or(lines.line_height());

    // Position: just inside the gutter's right edge, on the
    // caret line. The glyph itself is small enough not to
    // crowd the line-number column.
    let glyph = "\u{23CE}"; // ⏎ RETURN SYMBOL
    let (fctx, lctx) = ctx.text_contexts();
    let mut b = lctx.ranged_builder(fctx, glyph, 1.0, true);
    b.push_default(parley::StyleProperty::FontSize(11.0));
    b.push_default(parley::StyleProperty::FontFamily(
        parley::FontFamily::Single(parley::FontFamilyName::Generic(
            parley::GenericFamily::SansSerif,
        )),
    ));
    b.push_default(parley::StyleProperty::Brush(BrushIndex(0)));
    let mut layout = b.build(glyph);
    layout.break_all_lines(None);

    let glyph_w = layout.width() as f64;
    let glyph_h = layout.height() as f64;
    // Right-align inside the gutter, with a 4px right margin
    // so the glyph doesn't touch the gutter/text boundary.
    // The scroll_x offset keeps the glyph anchored to the
    // gutter even when the document scrolls horizontally.
    let x = scroll_x + gutter_width - glyph_w - 4.0;
    let y = sl.y + (line_height - glyph_h) / 2.0;

    // Mute the accent color so the glyph reads as a hint.
    // ~70% alpha keeps it visible without dominating.
    let [r, g, bl, _a] = accent.components;
    let muted = Color::new([r, g, bl, 0.7]);

    render_text(
        painter,
        Affine::translate((x, y)),
        &layout,
        &[Brush::from(muted).into()],
        true,
    );
}

/// Paint section-marker decorations for the visible region.
///
/// For each marker that requested a separator we draw a 1px horizontal
/// rule across the whole editor width (gutter → viewport edge). The
/// rule sits at the *top* of the marker line so it visually divides
/// the section above it from the section being announced below.
///
/// We deliberately do **not** overdraw the comment text itself with a
/// bold section name — the original `// MARK: …` text stays exactly
/// as written so the user can still click it, edit it, and see it in
/// the same monospace font as the rest of the source. The rule alone
/// is enough of a visual section break, and "what you wrote is what
/// you see" is the friendlier rule for an editor.
///
/// Color: a manually-darkened blend over the theme's editor background
/// so the rule reads cleanly without depending on the column-guide
/// color (which is intentionally faint and hard to see against most
/// backgrounds at 1px height).
///
/// `markers` should be the full list of parsed section markers; we
/// internally filter to the visible region using `screen`.
pub fn paint_section_markers(
    _ctx: &mut PaintCtx<'_>,
    painter: &mut Painter<'_>,
    screen: &ScreenLines,
    _lines: &Lines,
    markers: &[code_editor_core::model::SectionMarker],
    theme: &EditorTheme,
    viewport_width: f64,
    gutter_width: f64,
    _scroll_x: f64,
) {
    if markers.is_empty() || screen.is_empty() {
        return;
    }

    // Make the rule clearly visible against the editor background.
    // The theme's `column_guide` is too faint at 1px; pick a value
    // that's distinctly darker than `editor_bg` regardless of light/
    // dark mode by mixing toward the line-number color (which the
    // theme already tunes for medium contrast).
    let rule_color = darken_for_rule(theme.line_number, theme.editor_bg);

    for marker in markers {
        if !marker.draw_separator {
            continue;
        }
        // Only paint markers whose line is currently in view. Linear
        // scan over `screen.lines` is fine — the visible window is
        // tiny relative to total markers in practical files.
        let Some(sl) = screen.lines.iter().find(|sl| sl.line == marker.line) else {
            continue;
        };

        // Sticky to the gutter on the left, runs to viewport_width on
        // the right. Sticky means scroll_x doesn't apply — the rule
        // always spans the same physical pixel range.
        let rule_y = sl.y + 0.5; // 1px line, centered on a half-pixel for crispness
        let rect = Rect::new(gutter_width, rule_y, viewport_width, rule_y + 1.0);
        painter.fill_rect(rect, rule_color);
    }
}

/// Return a color that's clearly darker (or lighter, for dark themes)
/// than `bg`, derived from `accent`. Used as the section-marker rule
/// color so it reads sharply at 1px regardless of theme.
fn darken_for_rule(accent: Color, bg: Color) -> Color {
    let a = accent.to_rgba8();
    let b = bg.to_rgba8();
    // Decide light-vs-dark from the background luminance, then bias
    // the accent strongly toward "more contrasty than the line-number
    // color" — i.e. darker on a light background, lighter on a dark.
    let bg_luma = 0.299 * b.r as f32 + 0.587 * b.g as f32 + 0.114 * b.b as f32;
    let blend = if bg_luma > 128.0 {
        // Light background — darken: 70% accent, 30% black.
        |c: u8| (c as f32 * 0.7) as u8
    } else {
        // Dark background — lighten: 70% accent, 30% white.
        |c: u8| (c as f32 * 0.7 + 255.0 * 0.3) as u8
    };
    Color::from_rgb8(blend(a.r), blend(a.g), blend(a.b))
}
