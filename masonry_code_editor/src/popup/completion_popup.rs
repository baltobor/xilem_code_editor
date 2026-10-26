//! Caret-anchored autocomplete popup — Xcode-style balloon below the
//! cursor.
//!
//! Renders a list of completion candidates in a rounded rectangle just
//! beneath the caret. The selected row is highlighted and prefixed
//! with a triangular marker so the user can tell what Tab / Return
//! will accept.
//!
//! ## Contract
//!
//! - **Inputs**: completion items, selection index, caret screen
//!   position, viewport size, theme colours.
//! - **Output**: a list of `(item_index, Rect)` tuples for click
//!   hit-testing. Empty when the popup wasn't drawn (no items).
//! - **Position**: anchored just below the caret. If the popup would
//!   overflow the viewport bottom, it flips above the caret instead.
//! - **Scrolling**: the visible window slides so the selected item
//!   stays on screen — the widget never has to manage a scroll
//!   offset; selection drives visibility.
//! - **No mutation**: pure render pass. The widget is responsible for
//!   maintaining `selected_index` in response to ↑/↓ keys.
//!
//! ## Style
//!
//! Each row carries a 2-character "kind prefix" (`fn`, `m`, `f`, …)
//! taken from the LSP `CompletionItemKind`, and the item label.
//! Long labels are truncated to fit the popup width. Fonts and
//! sizes are chosen to match the existing inline-bubble overlay so
//! the two interactive surfaces feel like siblings.

// TODO: I don't like the look of the popups. This design should be more beautiful!

use masonry::core::{BrushIndex, PaintCtx, render_text};
use masonry::imaging::Painter;
use masonry::kurbo::{Affine, Rect, RoundedRect};
use masonry::parley;
use masonry::peniko::Color;

/// Result of painting the caret-anchored completion popup. The widget
/// stashes `item_bounds` for click hit-testing.
pub struct CompletionPopupPaintResult {
    /// `(item_index, rect)` per painted row. Empty when the popup
    /// wasn't drawn.
    pub item_bounds: Vec<(usize, Rect)>,
}

/// Map a `CompletionKind` to a one-letter prefix glyph used in the
/// popup's leftmost column. Mirrors the previous bar-style popup so
/// users see the same shorthand (`fn`, `m`, `f`, …) in the new
/// caret-anchored balloon.
fn completion_kind_prefix(kind: code_editor_core::completion::CompletionKind) -> &'static str {
    use code_editor_core::completion::CompletionKind;
    match kind {
        CompletionKind::Function => "fn",
        CompletionKind::Method => "m",
        CompletionKind::Field => "f",
        CompletionKind::Variable => "v",
        CompletionKind::Constant => "K",
        CompletionKind::Struct => "s",
        CompletionKind::Enum => "E",
        CompletionKind::Module => "M",
        CompletionKind::Keyword => "k",
        CompletionKind::Snippet => "S",
        CompletionKind::Other => "?",
    }
}

/// Paint a Xcode-style autocomplete balloon anchored just below the
/// caret. Items render kind glyph + label; the row at
/// `selected_index` gets a highlighted background.
///
/// Coordinate system: same viewport-local pixel space as the bubble
/// painter — `caret_x` and `caret_y` are widget-local pixels.
///
/// `theme_bg`, `theme_border`, `theme_text`, `theme_text_secondary`,
/// `theme_selection` come from the editor theme so the popup blends
/// with the editor's palette.
pub fn paint_completion_popup(
    ctx: &mut PaintCtx<'_>,
    painter: &mut Painter<'_>,
    items: &[code_editor_core::completion::CompletionItem],
    selected_index: usize,
    caret_x: f64,
    caret_y: f64,
    line_height: f64,
    viewport_width: f64,
    viewport_height: f64,
    theme_bg: Color,
    theme_border: Color,
    theme_text: Color,
    theme_text_secondary: Color,
    theme_selection: Color,
) -> CompletionPopupPaintResult {
    let mut item_bounds: Vec<(usize, Rect)> = Vec::new();
    if items.is_empty() {
        return CompletionPopupPaintResult { item_bounds };
    }

    // Layout constants. The popup sits below the caret with a small
    // vertical gap so it doesn't visually collide with the cursor.
    let row_height: f64 = 22.0;
    let max_visible_rows: usize = 10;
    let visible_rows = items.len().min(max_visible_rows);
    let popup_height = (visible_rows as f64) * row_height;
    let pad_x = 8.0;
    // Wider widths look right for typical method names; cap at the
    // viewport width minus a small margin so the popup never spills
    // past the editor.
    let popup_width: f64 = 360.0_f64.min(viewport_width - 16.0);

    let popup_x = (caret_x - 8.0).max(8.0);
    let mut popup_y = caret_y + line_height + 4.0;
    // Flip above the caret if there isn't room below.
    if popup_y + popup_height > viewport_height - 4.0 {
        popup_y = (caret_y - popup_height - 4.0).max(4.0);
    }

    let popup_rect = Rect::new(
        popup_x,
        popup_y,
        popup_x + popup_width,
        popup_y + popup_height,
    );
    let rounded = RoundedRect::from_rect(popup_rect, 6.0);
    painter.fill(rounded, theme_bg).draw();
    use masonry::kurbo::Stroke;
    painter
        .stroke(rounded, &Stroke::new(1.0), theme_border)
        .draw();

    // Scroll the visible window so `selected_index` is always shown.
    let start = if selected_index >= visible_rows {
        selected_index + 1 - visible_rows
    } else {
        0
    };
    let end = (start + visible_rows).min(items.len());

    for (slot, idx) in (start..end).enumerate() {
        let item = &items[idx];
        let row_y = popup_y + (slot as f64) * row_height;
        let row_rect = Rect::new(popup_x, row_y, popup_x + popup_width, row_y + row_height);

        // Highlight selected row.
        if idx == selected_index {
            painter.fill(row_rect, theme_selection).draw();
        }

        // Marker triangle on the selected row, in the gutter
        // position; signals "this is what Tab/Return will accept".
        let marker_x = popup_x + 4.0;
        let marker_y = row_y + row_height / 2.0;
        if idx == selected_index {
            use masonry::kurbo::BezPath;
            let mut path = BezPath::new();
            path.move_to((marker_x, marker_y - 4.0));
            path.line_to((marker_x + 5.0, marker_y));
            path.line_to((marker_x, marker_y + 4.0));
            path.close_path();
            painter.fill(&path, theme_text).draw();
        }

        // Kind prefix (small monospace badge).
        let prefix = completion_kind_prefix(item.kind);
        let (fctx_p, lctx_p) = ctx.text_contexts();
        let mut pb = lctx_p.ranged_builder(fctx_p, prefix, 1.0, true);
        pb.push_default(parley::StyleProperty::FontSize(11.0));
        pb.push_default(parley::StyleProperty::FontFamily(
            parley::FontFamily::Single(parley::FontFamilyName::Generic(
                parley::GenericFamily::Monospace,
            )),
        ));
        pb.push_default(parley::StyleProperty::FontWeight(parley::FontWeight::BOLD));
        pb.push_default(parley::StyleProperty::Brush(BrushIndex(0)));
        let mut prefix_layout = pb.build(prefix);
        prefix_layout.break_all_lines(None);
        let prefix_y = row_y + (row_height - prefix_layout.height() as f64) / 2.0;
        render_text(
            painter,
            Affine::translate((popup_x + 14.0, prefix_y)),
            &prefix_layout,
            &[theme_text_secondary.into()],
            true,
        );

        // Item label. When the LSP supplied a `detail` (function
        // signature, type, etc.) we right-align it on the same row
        // in `theme_text_secondary` so the user sees signature info
        // as they navigate. The label gets the leftmost ~60% of the
        // row, the detail gets the right side; both truncate.
        let label_left = popup_x + 36.0;
        let detail_text = item.detail.as_deref().unwrap_or("").trim();
        let detail_visible = !detail_text.is_empty();
        let detail_max_width = if detail_visible {
            (popup_width * 0.45).min(180.0)
        } else {
            0.0
        };
        let label_max_width =
            popup_width - 50.0 - pad_x - detail_max_width - if detail_visible { 8.0 } else { 0.0 };

        let (fctx_l, lctx_l) = ctx.text_contexts();
        let mut lb = lctx_l.ranged_builder(fctx_l, &item.label, 1.0, true);
        lb.push_default(parley::StyleProperty::FontSize(13.0));
        lb.push_default(parley::StyleProperty::FontFamily(
            parley::FontFamily::Single(parley::FontFamilyName::Generic(
                parley::GenericFamily::Monospace,
            )),
        ));
        lb.push_default(parley::StyleProperty::Brush(BrushIndex(0)));
        let mut label_layout = lb.build(&item.label);
        label_layout.break_all_lines(Some(label_max_width as f32));
        let label_y = row_y + (row_height - label_layout.height() as f64) / 2.0;
        render_text(
            painter,
            Affine::translate((label_left, label_y)),
            &label_layout,
            &[theme_text.into()],
            true,
        );

        // Right-aligned detail (signature / type info from LSP).
        // Only the selected row gets the full string; non-selected
        // rows get a compact preview so the popup doesn't turn into
        // a wall of text.
        if detail_visible {
            let preview = if idx == selected_index {
                detail_text.to_string()
            } else {
                first_line_of(detail_text)
            };
            let (fctx_d, lctx_d) = ctx.text_contexts();
            let mut db = lctx_d.ranged_builder(fctx_d, &preview, 1.0, true);
            db.push_default(parley::StyleProperty::FontSize(11.0));
            db.push_default(parley::StyleProperty::FontFamily(
                parley::FontFamily::Single(parley::FontFamilyName::Generic(
                    parley::GenericFamily::Monospace,
                )),
            ));
            db.push_default(parley::StyleProperty::Brush(BrushIndex(0)));
            let mut detail_layout = db.build(&preview);
            detail_layout.break_all_lines(Some(detail_max_width as f32));
            let detail_y = row_y + (row_height - detail_layout.height() as f64) / 2.0;
            let detail_width = detail_layout.width() as f64;
            // Right-align inside the popup, leaving an 8px right margin.
            let detail_x = popup_x + popup_width - 8.0 - detail_width;
            render_text(
                painter,
                Affine::translate((detail_x, detail_y)),
                &detail_layout,
                &[theme_text_secondary.into()],
                true,
            );
        }

        item_bounds.push((idx, row_rect));
    }

    // Docs panel — rendered below the list when the selected item has
    // documentation. Capped to ~5 lines; wraps at the popup width.
    let doc_text = items
        .get(selected_index)
        .and_then(|e| e.documentation.as_deref());
    if let Some(doc) = doc_text {
        let doc_to_show = strip_markdown_noise(doc);
        if !doc_to_show.is_empty() {
            let max_doc_lines = 5usize;
            let doc_line_h: f64 = 16.0;
            let doc_pad_x = 8.0;
            let doc_pad_y = 4.0;
            let doc_lines: Vec<&str> = doc_to_show
                .lines()
                .filter(|l| !l.trim().is_empty())
                .take(max_doc_lines)
                .collect();
            if !doc_lines.is_empty() {
                let doc_panel_h = doc_lines.len() as f64 * doc_line_h + doc_pad_y * 2.0 + 1.0; // +1 divider
                let doc_panel_y = popup_y + popup_height;
                let doc_panel_rect = Rect::new(
                    popup_x,
                    doc_panel_y,
                    popup_x + popup_width,
                    doc_panel_y + doc_panel_h,
                );
                let doc_rounded = RoundedRect::from_rect(doc_panel_rect, 6.0);

                // Slightly darker background to visually separate from list.
                let doc_bg = darken(theme_bg, 0.06);
                painter.fill(doc_rounded, doc_bg).draw();
                use masonry::kurbo::Stroke;
                painter
                    .stroke(doc_rounded, &Stroke::new(1.0), theme_border)
                    .draw();

                // Divider between list and doc panel.
                let div = Rect::new(
                    popup_x + doc_pad_x,
                    doc_panel_y,
                    popup_x + popup_width - doc_pad_x,
                    doc_panel_y + 1.0,
                );
                painter.fill(div, theme_border).draw();

                let text_top = doc_panel_y + 1.0 + doc_pad_y;
                let max_text_w = popup_width - doc_pad_x * 2.0;
                for (i, line) in doc_lines.iter().enumerate() {
                    let row_y = text_top + i as f64 * doc_line_h;
                    let (fctx, lctx) = ctx.text_contexts();
                    let mut b = lctx.ranged_builder(fctx, *line, 1.0, true);
                    b.push_default(parley::StyleProperty::FontSize(11.5));
                    b.push_default(parley::StyleProperty::FontFamily(
                        parley::FontFamily::Single(parley::FontFamilyName::Generic(
                            parley::GenericFamily::SansSerif,
                        )),
                    ));
                    b.push_default(parley::StyleProperty::Brush(BrushIndex(0)));
                    let mut layout = b.build(*line);
                    layout.break_all_lines(Some(max_text_w as f32));
                    render_text(
                        painter,
                        Affine::translate((popup_x + doc_pad_x, row_y)),
                        &layout,
                        &[theme_text_secondary.into()],
                        true,
                    );
                }
            }
        }
    }

    CompletionPopupPaintResult { item_bounds }
}

/// Remove Markdown noise (fenced code fences, bold/italic markers,
/// backticks) so docs read cleanly as plain text in the small panel.
fn strip_markdown_noise(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut in_fence = false;
    for line in s.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("```") {
            in_fence = !in_fence;
            continue;
        }
        if in_fence {
            // Keep code lines as-is (they're still readable).
            out.push_str(line);
            out.push('\n');
            continue;
        }
        // Strip inline Markdown: **bold**, *italic*, `code`, [link](url)
        let clean = trimmed.replace("**", "").replace('*', "").replace('`', "");
        // Strip [text](url) → text
        let clean = strip_md_links(&clean);
        out.push_str(&clean);
        out.push('\n');
    }
    out
}

fn strip_md_links(s: &str) -> String {
    let mut out = String::new();
    let mut rest = s;
    while let Some(lb) = rest.find('[') {
        out.push_str(&rest[..lb]);
        rest = &rest[lb + 1..];
        if let Some(rb) = rest.find(']') {
            let text = &rest[..rb];
            rest = &rest[rb + 1..];
            if rest.starts_with('(') {
                if let Some(rp) = rest.find(')') {
                    rest = &rest[rp + 1..];
                }
            }
            out.push_str(text);
        }
    }
    out.push_str(rest);
    out
}

/// Darken a color by reducing each RGB component by `amount` (0.0–1.0).
fn darken(c: Color, amount: f32) -> Color {
    let [r, g, b, a] = c.components;
    Color::new([
        (r - amount).max(0.0),
        (g - amount).max(0.0),
        (b - amount).max(0.0),
        a,
    ])
}

/// Paint the block-mode popup — a single full-preview code-action
/// (e.g. "Fill match arms") shown as a multi-line preview at the
/// **start** of the line containing the caret (not below the caret).
/// The preview overlays where the snippet would land, so the user
/// can see *where* the code will be inserted, much like ghost text
/// would.
///
/// Style differs from the regular list popup in two ways:
/// - Slightly green-tinted background (visual hint that this is a
///   structural snippet, not a regular completion).
/// - Title row separated from the preview body by a thin divider.
///
/// The preview is the action's `WorkspaceEdit.new_text`, truncated
/// to ~6 lines so a long snippet doesn't dominate the editor.
///
/// Coordinate system: viewport-local pixels, same as the list popup.
/// `gutter_w` lets the popup's left edge align with the start of the
/// text column (right of the line-number gutter), not the widget
/// origin.
pub fn paint_completion_block_popup(
    ctx: &mut PaintCtx<'_>,
    painter: &mut Painter<'_>,
    title: &str,
    preview: &str,
    source_idx: usize,
    source_total: usize,
    line_y: f64,
    line_height: f64,
    gutter_w: f64,
    viewport_width: f64,
    viewport_height: f64,
    theme_bg: Color,
    theme_border: Color,
    theme_text: Color,
    theme_text_secondary: Color,
) -> CompletionPopupPaintResult {
    // Truncate the preview to a sensible number of lines so a giant
    // snippet doesn't dominate the editor. The single block-mode
    // entry is index 0 — the caller's hit-test treats a click
    // anywhere on the popup as "accept block mode".
    const MAX_LINES: usize = 6;
    let preview_lines: Vec<&str> = preview.lines().take(MAX_LINES).collect();
    let truncated = preview.lines().count() > MAX_LINES;

    let title_row_h: f64 = 22.0;
    let divider_h: f64 = 1.0;
    let line_h: f64 = line_height.max(16.0);
    let body_h = (preview_lines.len() as f64) * line_h + if truncated { line_h } else { 0.0 };
    let popup_height = title_row_h + divider_h + body_h + 6.0;

    // Position at start of the text column (right of gutter), on the
    // **same line** as the caret. We deliberately do NOT offset
    // below the caret — the user wants the popup to overlay where
    // the snippet will land, like ghost text.
    let pad_x = 8.0;
    let popup_x = gutter_w + 2.0;
    // Hard-cap so a long preview line doesn't push the popup off
    // the right edge.
    let popup_width = (viewport_width - popup_x - 8.0).max(200.0).min(560.0);
    let mut popup_y = line_y;
    if popup_y + popup_height > viewport_height - 4.0 {
        popup_y = (viewport_height - popup_height - 4.0).max(4.0);
    }

    // Slightly green-tinted background — mix the editor's bg with a
    // soft green so the surface still reads as "in editor" but the
    // user can tell at a glance this is a snippet, not a regular
    // completion.
    let tinted_bg = tint_with_green(theme_bg, 0.18);

    let popup_rect = Rect::new(
        popup_x,
        popup_y,
        popup_x + popup_width,
        popup_y + popup_height,
    );
    let rounded = RoundedRect::from_rect(popup_rect, 6.0);
    painter.fill(rounded, tinted_bg).draw();
    use masonry::kurbo::Stroke;
    painter
        .stroke(rounded, &Stroke::new(1.0), theme_border)
        .draw();

    // Title row.
    let (fctx_t, lctx_t) = ctx.text_contexts();
    let mut tb = lctx_t.ranged_builder(fctx_t, title, 1.0, true);
    tb.push_default(parley::StyleProperty::FontSize(12.0));
    tb.push_default(parley::StyleProperty::FontFamily(
        parley::FontFamily::Single(parley::FontFamilyName::Generic(
            parley::GenericFamily::SansSerif,
        )),
    ));
    tb.push_default(parley::StyleProperty::FontWeight(parley::FontWeight::BOLD));
    tb.push_default(parley::StyleProperty::Brush(BrushIndex(0)));
    let mut title_layout = tb.build(title);
    title_layout.break_all_lines(Some((popup_width - 2.0 * pad_x) as f32));
    let title_y = popup_y + (title_row_h - title_layout.height() as f64) / 2.0;
    render_text(
        painter,
        Affine::translate((popup_x + pad_x, title_y)),
        &title_layout,
        &[theme_text.into()],
        true,
    );

    // `n/m` source-count indicator, right-aligned in the
    // title row when there's more than one snippet source.
    // Tells the user "Up/Down browses to other suggestions"
    // — the spec's mimic of a list with a counter.
    if source_total > 1 {
        let counter = format!("{}/{}", source_idx + 1, source_total);
        let (fctx_c, lctx_c) = ctx.text_contexts();
        let mut cb = lctx_c.ranged_builder(fctx_c, &counter, 1.0, true);
        cb.push_default(parley::StyleProperty::FontSize(11.0));
        cb.push_default(parley::StyleProperty::FontFamily(
            parley::FontFamily::Single(parley::FontFamilyName::Generic(
                parley::GenericFamily::SansSerif,
            )),
        ));
        cb.push_default(parley::StyleProperty::Brush(BrushIndex(0)));
        let mut c_layout = cb.build(&counter);
        c_layout.break_all_lines(None);
        let c_w = c_layout.width() as f64;
        let c_h = c_layout.height() as f64;
        let c_x = popup_x + popup_width - pad_x - c_w;
        let c_y = popup_y + (title_row_h - c_h) / 2.0;
        render_text(
            painter,
            Affine::translate((c_x, c_y)),
            &c_layout,
            &[theme_text_secondary.into()],
            true,
        );
    }

    // Divider.
    let div_y = popup_y + title_row_h;
    let div_rect = Rect::new(
        popup_x + pad_x,
        div_y,
        popup_x + popup_width - pad_x,
        div_y + divider_h,
    );
    painter.fill(div_rect, theme_border).draw();

    // Preview body — render each line of the snippet's `new_text`
    // in monospaced text so it visually previews like the buffer it
    // will become.
    let body_top = div_y + divider_h + 3.0;
    for (slot, ln) in preview_lines.iter().enumerate() {
        let row_y = body_top + (slot as f64) * line_h;
        let (fctx_l, lctx_l) = ctx.text_contexts();
        let mut lb = lctx_l.ranged_builder(fctx_l, *ln, 1.0, true);
        lb.push_default(parley::StyleProperty::FontSize(12.0));
        lb.push_default(parley::StyleProperty::FontFamily(
            parley::FontFamily::Single(parley::FontFamilyName::Generic(
                parley::GenericFamily::Monospace,
            )),
        ));
        lb.push_default(parley::StyleProperty::Brush(BrushIndex(0)));
        let mut layout = lb.build(*ln);
        layout.break_all_lines(Some((popup_width - 2.0 * pad_x) as f32));
        render_text(
            painter,
            Affine::translate((popup_x + pad_x, row_y)),
            &layout,
            &[theme_text.into()],
            true,
        );
    }
    if truncated {
        let row_y = body_top + (preview_lines.len() as f64) * line_h;
        let ellipsis = "…";
        let (fctx_e, lctx_e) = ctx.text_contexts();
        let mut eb = lctx_e.ranged_builder(fctx_e, ellipsis, 1.0, true);
        eb.push_default(parley::StyleProperty::FontSize(12.0));
        eb.push_default(parley::StyleProperty::FontFamily(
            parley::FontFamily::Single(parley::FontFamilyName::Generic(
                parley::GenericFamily::Monospace,
            )),
        ));
        eb.push_default(parley::StyleProperty::Brush(BrushIndex(0)));
        let mut layout = eb.build(ellipsis);
        layout.break_all_lines(None);
        render_text(
            painter,
            Affine::translate((popup_x + pad_x, row_y)),
            &layout,
            &[theme_text_secondary.into()],
            true,
        );
    }

    // The whole popup is index 0 — a click anywhere accepts the
    // block-mode action (there's only one item).
    CompletionPopupPaintResult {
        item_bounds: vec![(0, popup_rect)],
    }
}

/// Mix the given color towards green by `t` (0.0 = unchanged,
/// 1.0 = pure green). Used to give the block-mode popup a hint of
/// snippet-green so the user reads it as "this is a snippet" at a
/// glance, while still keeping the editor's overall palette.
fn tint_with_green(bg: Color, t: f64) -> Color {
    let t = t.clamp(0.0, 1.0) as f32;
    let [r, g, b, a] = bg.components;
    // Pull towards a desaturated green (#88c088). Blending against
    // a fully saturated green made the popup garish in dogfood;
    // this softer target keeps the tint a hint, not a billboard.
    let target_r = 0.53_f32;
    let target_g = 0.75_f32;
    let target_b = 0.53_f32;
    Color::new([
        r + (target_r - r) * t,
        g + (target_g - g) * t,
        b + (target_b - b) * t,
        a,
    ])
}

/// Return the first non-empty line of a multi-line string, trimmed.
/// Used to keep non-selected detail previews compact.
fn first_line_of(s: &str) -> String {
    s.lines()
        .map(|l| l.trim())
        .find(|l| !l.is_empty())
        .unwrap_or("")
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::first_line_of;

    #[test]
    fn first_line_of_returns_first_non_empty_line() {
        assert_eq!(first_line_of("fn foo()\n  -> i32"), "fn foo()");
    }

    #[test]
    fn first_line_of_skips_leading_blank_lines() {
        assert_eq!(first_line_of("\n\n  ready\nlater"), "ready");
    }

    #[test]
    fn first_line_of_trims_whitespace() {
        assert_eq!(first_line_of("    spaced    "), "spaced");
    }

    #[test]
    fn first_line_of_returns_empty_for_blank_input() {
        assert_eq!(first_line_of(""), "");
        assert_eq!(first_line_of("   \n  \n"), "");
    }
}
