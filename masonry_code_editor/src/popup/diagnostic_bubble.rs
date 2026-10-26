//! Inline diagnostic bubble — Xcode-style error/warning popovers.
//!
//! Each error or warning line gets a bubble at the end of its text.
//! Click to expand; expanded bubbles show the full wrapped message
//! plus a list of quick-fix cards (when LSP supplies code actions).
//!
//! This module is the rendering pass only — the widget feeds it the
//! diagnostic list, the expansion state, and a closure that returns
//! the per-line code-action cache. The result is two lists of
//! `(line, rect)` and `(line, action_index, rect)` bounds the widget
//! uses for click hit-testing.

// TODO: I don't like the look of the popups. This design should be more beautiful!

use masonry::core::{BrushIndex, PaintCtx, render_text};
use masonry::imaging::Painter;
use masonry::kurbo::{Affine, Circle, Rect, RoundedRect};
use masonry::parley;
use masonry::peniko::Color;

use crate::text_layout::Lines;
use crate::text_layout::ScreenLines;
use code_editor_core::model::{Diagnostic, DiagnosticSeverity};

/// Bounds returned by `paint_inline_diagnostic_bubbles` for hit-testing.
pub struct DiagnosticBubblePaintResult {
    /// `(line, rect)` for each bubble's clickable area (toggles
    /// expansion).
    pub bubble_bounds: Vec<(usize, Rect)>,
    /// `(line, action_index, rect)` for each quick-fix row painted
    /// inside an expanded bubble. Empty when nothing is expanded or
    /// the expanded bubble has no fixes available yet.
    pub action_bounds: Vec<(usize, usize, Rect)>,
}

/// Paint Xcode-style inline diagnostic bubbles at the end of
/// error/warning lines.
///
/// Each bubble shows a severity icon and truncated message with a
/// chevron. Only one bubble per line is shown (highest severity wins).
/// When `expanded_line` matches a bubble's line, the full wrapped
/// message is rendered below the inline bubble, followed by a list of
/// quick-fix rows pulled from `code_actions_for_line(line)`.
///
/// The closure form for actions keeps the painter free of any direct
/// dependency on `AppModel` — the caller (widget) wires in a lookup
/// against the per-(file, line) cache.
pub fn paint_inline_diagnostic_bubbles<'a, F>(
    ctx: &mut PaintCtx<'_>,
    painter: &mut Painter<'_>,
    screen: &ScreenLines,
    lines: &Lines,
    diagnostics: &[Diagnostic],
    gutter_width: f64,
    scroll_x: f64,
    expanded_line: Option<usize>,
    selected_action_index: usize,
    code_actions_for_line: F,
) -> DiagnosticBubblePaintResult
where
    F: Fn(usize) -> &'a [code_editor_core::model::CodeAction],
{
    use std::collections::HashMap;

    let mut bounds = Vec::new();
    let mut action_bounds: Vec<(usize, usize, Rect)> = Vec::new();

    // Group diagnostics by line, keeping only the highest severity per line
    let mut by_line: HashMap<usize, &Diagnostic> = HashMap::new();
    for diag in diagnostics {
        let dominated = by_line.get(&diag.line).map_or(false, |existing| {
            // Error > Warning > Info > Hint
            match (existing.severity, diag.severity) {
                (DiagnosticSeverity::Error, _) => true,
                (DiagnosticSeverity::Warning, DiagnosticSeverity::Error) => false,
                (DiagnosticSeverity::Warning, _) => true,
                _ => false,
            }
        });
        if !dominated {
            by_line.insert(diag.line, diag);
        }
    }

    // Paint order matters: when a bubble is expanded, its popover
    // grows downward and may overlap bubbles on later lines. We want
    // the expanded popover to cover anything below it, so sort by
    // line ascending and place the expanded bubble (if any) last.
    // That way the *most* important bubble (the one the user just
    // clicked) always wins z-order.
    let mut diags_in_paint_order: Vec<&Diagnostic> = by_line.values().copied().collect();
    diags_in_paint_order.sort_by_key(|d| (Some(d.line) == expanded_line, d.line));

    for diag in diags_in_paint_order {
        // Find the screen line for this diagnostic
        let Some(sl) = screen.lines.iter().find(|sl| sl.line == diag.line) else {
            continue;
        };

        // Get line width from layout
        let line_width = lines
            .get_layout(sl.line)
            .map(|l| l.layout.width() as f64)
            .unwrap_or(0.0);

        // Xcode-style colors (pink for errors, yellow for warnings)
        let (bg_color, icon_color) = match diag.severity {
            DiagnosticSeverity::Error => (
                Color::from_rgba8(255, 200, 200, 230), // Light pink
                Color::from_rgb8(220, 50, 50),         // Red icon
            ),
            DiagnosticSeverity::Warning => (
                Color::from_rgba8(255, 245, 180, 230), // Light yellow
                Color::from_rgb8(200, 170, 50),        // Yellow icon
            ),
            _ => continue, // Skip info/hint for inline bubbles
        };

        // Position: after line text + gap
        let x = gutter_width + line_width + 16.0 - scroll_x;
        let y = sl.y;
        let line_height = lines
            .get_layout(sl.line)
            .map(|l| l.line_height)
            .unwrap_or(lines.line_height());

        // Check if this line is expanded
        let is_expanded = expanded_line == Some(diag.line);

        // Inline bubble shows ONLY first line of message (full message in expanded popover)
        let first_line = diag.message.lines().next().unwrap_or(&diag.message);
        let inline_msg = if first_line.len() > 50 {
            format!("{}...", &first_line[..47])
        } else {
            first_line.to_string()
        };

        // Show chevron if there's more content (multi-line or truncated)
        let has_more_content = diag.message.contains('\n') || first_line.len() > 50;
        let needs_chevron = has_more_content;
        let display_msg = inline_msg;

        // Build text layout for measuring
        let (fctx, lctx) = ctx.text_contexts();
        let mut builder = lctx.ranged_builder(fctx, &display_msg, 1.0, true);
        builder.push_default(parley::StyleProperty::FontSize(12.0));
        builder.push_default(parley::StyleProperty::FontFamily(
            parley::FontFamily::Single(parley::FontFamilyName::Generic(
                parley::GenericFamily::Monospace,
            )),
        ));
        builder.push_default(parley::StyleProperty::Brush(BrushIndex(0)));
        let mut text_layout = builder.build(&display_msg);
        text_layout.break_all_lines(None);

        let text_width = text_layout.width() as f64;
        let text_height = text_layout.height() as f64;
        let chevron_width = if needs_chevron { 14.0 } else { 0.0 };
        let bubble_width = chevron_width + 24.0 + text_width + 12.0; // chevron + icon + text + padding
        // Use actual text height, but ensure minimum of one line height
        let bubble_height = text_height.max(line_height - 4.0);

        // Draw bubble background
        let rect = Rect::new(x, y + 2.0, x + bubble_width, y + 2.0 + bubble_height);
        let rounded = RoundedRect::from_rect(rect, 4.0);
        painter.fill(rounded, bg_color).draw();

        // Draw chevron indicator at the START of bubble (before icon)
        let icon_offset = if needs_chevron {
            use masonry::kurbo::{BezPath, Stroke};

            let chevron_x = x + 8.0;
            let chevron_y = y + 2.0 + bubble_height / 2.0;
            let chevron_size = 4.0;

            let mut path = BezPath::new();
            if is_expanded {
                // Draw "v" chevron (pointing down = expanded)
                path.move_to((
                    chevron_x - chevron_size / 2.0,
                    chevron_y - chevron_size / 2.0,
                ));
                path.line_to((
                    chevron_x + chevron_size / 2.0,
                    chevron_y + chevron_size / 2.0,
                ));
                path.line_to((
                    chevron_x + chevron_size * 1.5,
                    chevron_y - chevron_size / 2.0,
                ));
            } else {
                // Draw ">" chevron (pointing right = collapsed)
                path.move_to((chevron_x, chevron_y - chevron_size));
                path.line_to((chevron_x + chevron_size, chevron_y));
                path.line_to((chevron_x, chevron_y + chevron_size));
            }
            painter.stroke(&path, &Stroke::new(1.5), icon_color).draw();
            chevron_width
        } else {
            0.0
        };

        // Draw error icon (filled circle) - after chevron
        let icon_x = x + icon_offset + 12.0;
        let icon_y = y + 2.0 + bubble_height / 2.0;
        painter
            .fill(Circle::new((icon_x, icon_y), 5.0), icon_color)
            .draw();

        // Draw message text - after icon
        let text_x = x + icon_offset + 24.0;
        let text_y = y + 2.0 + (bubble_height - text_height) / 2.0;
        render_text(
            painter,
            Affine::translate((text_x, text_y)),
            &text_layout,
            &[Color::from_rgb8(60, 40, 40).into()], // Dark text on light background
            true,
        );

        bounds.push((diag.line, rect));

        // Draw expanded popover below the inline bubble.
        //
        // The inline bubble above already shows the message
        // (truncated to its first line + "..." when it's longer
        // than 50 chars). Only re-draw the message inside the
        // popover when there's *more* content to show — i.e. the
        // message was truncated or has multiple lines. When the
        // inline bubble already shows the whole thing, the popover
        // skips the message redraw and goes straight to the
        // quick-fixes panel below. Without this guard the user
        // sees the same one-liner twice (inline + wrapped in the
        // popover) for every short error. See screenshot.
        if is_expanded {
            let popover_x = x;
            let popover_y = y + 2.0 + bubble_height + 2.0; // Below the inline bubble
            let popover_max_width: f64 = 400.0;

            let popover_shows_message = has_more_content;

            // Build wrapped text layout for full message
            let (fctx2, lctx2) = ctx.text_contexts();
            let mut builder2 = lctx2.ranged_builder(fctx2, &diag.message, 1.0, true);
            builder2.push_default(parley::StyleProperty::FontSize(12.0));
            builder2.push_default(parley::StyleProperty::FontFamily(
                parley::FontFamily::Single(parley::FontFamilyName::Generic(
                    parley::GenericFamily::Monospace,
                )),
            ));
            builder2.push_default(parley::StyleProperty::Brush(BrushIndex(0)));
            let mut popover_layout = builder2.build(&diag.message);
            popover_layout.break_all_lines(Some(popover_max_width as f32 - 24.0)); // Wrap text

            let popover_text_height: f64 = if popover_shows_message {
                popover_layout.height() as f64
            } else {
                0.0
            };
            let popover_text_width: f64 = if popover_shows_message {
                popover_layout.width() as f64
            } else {
                0.0
            };
            // Keep at least a 220px width so the quick-fixes panel
            // below has room even when the message panel collapsed.
            let popover_width: f64 = popover_max_width.min(popover_text_width + 24.0).max(220.0);
            let popover_height = if popover_shows_message {
                popover_text_height + 16.0
            } else {
                0.0
            };

            // Draw popover background (matching severity color, like inline bubble).
            // Use fully opaque fills so bubbles on lines below don't
            // bleed through the expanded popover.
            let (popover_bg, popover_border, text_color) = match diag.severity {
                DiagnosticSeverity::Error => (
                    Color::from_rgb8(255, 220, 220), // Light pink, opaque
                    Color::from_rgb8(200, 80, 80),   // Red border
                    Color::from_rgb8(80, 30, 30),    // Dark red text
                ),
                DiagnosticSeverity::Warning => (
                    Color::from_rgb8(255, 252, 220), // Bright light yellow
                    Color::from_rgb8(200, 180, 80),  // Golden border
                    Color::from_rgb8(60, 50, 30),    // Dark gray text
                ),
                _ => (
                    Color::from_rgb8(240, 240, 240),
                    Color::from_rgb8(150, 150, 150),
                    Color::from_rgb8(50, 50, 50),
                ),
            };

            if popover_shows_message {
                let popover_rect = Rect::new(
                    popover_x,
                    popover_y,
                    popover_x + popover_width,
                    popover_y + popover_height,
                );
                let popover_rounded = RoundedRect::from_rect(popover_rect, 6.0);
                painter.fill(popover_rounded, popover_bg).draw();

                // Draw border
                use masonry::kurbo::Stroke;
                painter
                    .stroke(popover_rounded, &Stroke::new(1.0), popover_border)
                    .draw();

                // Draw full message text
                render_text(
                    painter,
                    Affine::translate((popover_x + 12.0, popover_y + 8.0)),
                    &popover_layout,
                    &[text_color.into()],
                    true,
                );
            }

            // Quick-fix rows below the message. Each row is a
            // clickable rect with the action title; the widget hit-
            // tests `action_bounds` and emits ApplyDiagnosticCodeAction
            // when one is clicked. We always paint a header strip
            // (even if no actions are available yet) so the user can
            // tell the bubble is doing something while LSP fetches.
            let actions = code_actions_for_line(diag.line);
            let row_height: f64 = 22.0;
            let row_padding_x: f64 = 12.0;
            let header_y = popover_y + popover_height;
            let actions_label = if actions.is_empty() {
                "No quick fixes available".to_string()
            } else {
                "Quick fixes".to_string()
            };
            // Header strip: a thin label bar above the rows, slightly
            // darker than the message background.
            let header_bg = match diag.severity {
                DiagnosticSeverity::Error => Color::from_rgba8(245, 200, 200, 245),
                DiagnosticSeverity::Warning => Color::from_rgba8(245, 235, 180, 250),
                _ => Color::from_rgba8(225, 225, 225, 245),
            };
            let header_rect = Rect::new(
                popover_x,
                header_y,
                popover_x + popover_width,
                header_y + row_height,
            );
            painter.fill(header_rect, header_bg).draw();
            let (fctx_h, lctx_h) = ctx.text_contexts();
            let mut hb = lctx_h.ranged_builder(fctx_h, &actions_label, 1.0, true);
            hb.push_default(parley::StyleProperty::FontSize(11.0));
            hb.push_default(parley::StyleProperty::FontFamily(
                parley::FontFamily::Single(parley::FontFamilyName::Generic(
                    parley::GenericFamily::Monospace,
                )),
            ));
            hb.push_default(parley::StyleProperty::FontWeight(parley::FontWeight::BOLD));
            hb.push_default(parley::StyleProperty::Brush(BrushIndex(0)));
            let mut header_layout = hb.build(&actions_label);
            header_layout.break_all_lines(None);
            let header_text_y = header_y + (row_height - header_layout.height() as f64) / 2.0;
            render_text(
                painter,
                Affine::translate((popover_x + row_padding_x, header_text_y)),
                &header_layout,
                &[text_color.into()],
                true,
            );

            // Each action row is rendered as a button-like card so the
            // user can tell it's clickable: a contrasting fill, a
            // visible 1px border, and a leading "▸" glyph that reads
            // as "press to run". Hover state isn't supported by the
            // editor widget yet — adding it would mean tracking the
            // mouse position per paint pass; deferred.
            let row_button_bg = match diag.severity {
                DiagnosticSeverity::Error => Color::from_rgba8(255, 240, 240, 250),
                DiagnosticSeverity::Warning => Color::from_rgba8(255, 252, 230, 252),
                _ => Color::from_rgba8(252, 252, 252, 250),
            };
            let row_border = match diag.severity {
                DiagnosticSeverity::Error => Color::from_rgb8(200, 80, 80),
                DiagnosticSeverity::Warning => Color::from_rgb8(200, 180, 80),
                _ => Color::from_rgb8(150, 150, 150),
            };
            let action_height = row_height + 2.0; // tiny breathing room
            let row_inset = 6.0; // inset so buttons read as separate cards
            for (idx, action) in actions.iter().enumerate() {
                let is_selected = idx == selected_action_index;
                let row_y = header_y + row_height + 4.0 + (idx as f64) * (action_height + 4.0);
                let row_rect = Rect::new(
                    popover_x + row_inset,
                    row_y,
                    popover_x + popover_width - row_inset,
                    row_y + action_height,
                );
                let rounded = RoundedRect::from_rect(row_rect, 4.0);
                painter.fill(rounded, row_button_bg).draw();
                use masonry::kurbo::Stroke;
                // Selected card gets a thicker border so it reads as
                // "this is what Tab / Return will apply".
                let stroke_width = if is_selected { 2.5 } else { 1.0 };
                painter
                    .stroke(rounded, &Stroke::new(stroke_width), row_border)
                    .draw();

                // Click-target marker. Selected card paints a fully
                // filled donut; unselected cards keep the open ring.
                let marker_cx = row_rect.x0 + 12.0;
                let marker_cy = row_rect.y0 + action_height / 2.0;
                let marker_outer = 5.0;
                let marker_inner = 2.0;
                painter
                    .fill(
                        Circle::new((marker_cx, marker_cy), marker_outer),
                        row_border,
                    )
                    .draw();
                if !is_selected {
                    painter
                        .fill(
                            Circle::new((marker_cx, marker_cy), marker_inner),
                            row_button_bg,
                        )
                        .draw();
                }

                // Action title, indented to clear the marker.
                let label_text = action.title.clone();
                let (fctx_a, lctx_a) = ctx.text_contexts();
                let mut ab = lctx_a.ranged_builder(fctx_a, &label_text, 1.0, true);
                ab.push_default(parley::StyleProperty::FontSize(12.0));
                ab.push_default(parley::StyleProperty::FontFamily(
                    parley::FontFamily::Single(parley::FontFamilyName::Generic(
                        parley::GenericFamily::Monospace,
                    )),
                ));
                ab.push_default(parley::StyleProperty::Brush(BrushIndex(0)));
                let mut row_layout = ab.build(&label_text);
                row_layout.break_all_lines(Some(popover_width as f32 - 32.0));
                let row_text_y = row_y + (action_height - row_layout.height() as f64) / 2.0;
                let text_x_in_row = row_rect.x0 + 22.0; // past the marker
                render_text(
                    painter,
                    Affine::translate((text_x_in_row, row_text_y)),
                    &row_layout,
                    &[text_color.into()],
                    true,
                );
                action_bounds.push((diag.line, idx, row_rect));
            }
        }
    }

    DiagnosticBubblePaintResult {
        bubble_bounds: bounds,
        action_bounds,
    }
}
