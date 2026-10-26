//! Caret-anchored Quick Help balloon — shows LSP hover documentation.
//!
//! The balloon renders the raw Markdown from the LSP hover response as
//! plain wrapped paragraphs (code blocks use monospace, prose uses the
//! UI sans-serif).  It flips direction based on available space:
//!
//! - **Below**: when there is more room below the caret than above.
//! - **Above**: when there is more room above the caret.
//!
//! If the content is taller than the available space the balloon is
//! capped at that height and the caller supplies a scroll offset so
//! the user can page through it (scroll is managed by the widget).
//!
//! ## Return value
//!
//! Returns the bounding `Rect` of the painted balloon (widget-local
//! pixels).  The widget uses it for:
//! 1. Hit-testing clicks — a click inside the rect is absorbed so the
//!    balloon stays open; a click outside closes it.
//! 2. Deciding the scroll delta for mouse-wheel events over the balloon.

// TODO: I don't like the look of the popups. This design should be more beautiful!

use masonry::core::{BrushIndex, PaintCtx, render_text};
use masonry::imaging::Painter;
use masonry::kurbo::{Affine, Rect, RoundedRect};
use masonry::parley;
use masonry::peniko::Brush;
use masonry::peniko::Color;

use crate::properties::SyntaxPalette;
use code_editor_core::highlight::HighlightCategory;
use code_editor_core::provider::HighlightProvider;

/// Highlights fenced code blocks in a popup, usually with the editor's
/// own provider and language.
#[derive(Clone, Copy)]
pub struct CodeBlockHighlighter<'a> {
    pub provider: Option<&'a dyn HighlightProvider>,
    pub language: &'a str,
}

/// Maximum balloon width in pixels.
const MAX_WIDTH: f64 = 480.0;
/// Minimum balloon width.
const MIN_WIDTH: f64 = 200.0;
/// Horizontal and vertical inner padding.
const PAD: f64 = 10.0;
/// Gap between rendered paragraphs.
const PARA_GAP: f64 = 6.0;
/// Corner radius.
const RADIUS: f64 = 6.0;

// ── Markdown pre-processing ──────────────────────────────────────────────────

/// A single rendered block extracted from the hover Markdown.
enum Block {
    /// Regular prose paragraph.
    Prose(String),
    /// Fenced or indented code block.
    Code(String),
}

/// Split the raw LSP hover string into renderable blocks.
///
/// Handles:
/// - Fenced code blocks (```…```)
/// - Blank-line paragraph breaks
fn parse_blocks(text: &str) -> Vec<Block> {
    let mut blocks = Vec::new();
    let mut para_lines: Vec<&str> = Vec::new();
    let mut in_fence = false;
    let mut fence_lines: Vec<&str> = Vec::new();

    let flush_para = |lines: &mut Vec<&str>, out: &mut Vec<Block>| {
        let s = lines.join("\n").trim().to_string();
        if !s.is_empty() {
            out.push(Block::Prose(s));
        }
        lines.clear();
    };

    for line in text.lines() {
        if line.starts_with("```") {
            if in_fence {
                // close fence
                in_fence = false;
                let code = fence_lines.join("\n");
                if !code.trim().is_empty() {
                    blocks.push(Block::Code(code));
                }
                fence_lines.clear();
            } else {
                // open fence — flush pending prose first
                flush_para(&mut para_lines, &mut blocks);
                in_fence = true;
                // skip the opening ``` line itself (may carry a lang hint)
            }
            continue;
        }

        if in_fence {
            fence_lines.push(line);
            continue;
        }

        if line.trim().is_empty() {
            flush_para(&mut para_lines, &mut blocks);
        } else {
            para_lines.push(line);
        }
    }

    if in_fence && !fence_lines.is_empty() {
        blocks.push(Block::Code(fence_lines.join("\n")));
    }
    flush_para(&mut para_lines, &mut blocks);

    blocks
}

// ── Layout helper ────────────────────────────────────────────────────────────

struct RenderedLine {
    layout: parley::Layout<BrushIndex>,
    /// Brushes to pass to render_text. Single-element for prose; full
    /// syntax palette for highlighted code blocks.
    brushes: Vec<Brush>,
    is_code: bool,
    /// Vertical offset from the top of the content area (before scroll).
    y: f64,
    height: f64,
}

fn build_layouts(
    ctx: &mut PaintCtx<'_>,
    blocks: &[Block],
    content_width: f64,
    syntax: &SyntaxPalette,
    theme_text: Color,
    code: CodeBlockHighlighter<'_>,
) -> (Vec<RenderedLine>, f64) {
    let (fctx, lctx) = ctx.text_contexts();
    let mut lines: Vec<RenderedLine> = Vec::new();
    let mut cursor_y = 0.0;

    for (bi, block) in blocks.iter().enumerate() {
        let (text, is_code) = match block {
            Block::Prose(t) => (t.as_str(), false),
            Block::Code(t) => (t.as_str(), true),
        };

        let font_size: f32 = if is_code { 12.0 } else { 13.0 };
        let wrap_width = content_width as f32;

        let mut builder = lctx.ranged_builder(fctx, text, 1.0, true);
        builder.push_default(parley::StyleProperty::FontSize(font_size));
        if is_code {
            builder.push_default(parley::StyleProperty::FontFamily(
                parley::FontFamily::Single(parley::FontFamilyName::Generic(
                    parley::GenericFamily::Monospace,
                )),
            ));
        } else {
            builder.push_default(parley::StyleProperty::FontFamily(
                parley::FontFamily::Single(parley::FontFamilyName::Generic(
                    parley::GenericFamily::SansSerif,
                )),
            ));
        }
        builder.push_default(parley::StyleProperty::Brush(BrushIndex(0)));

        let brushes: Vec<Brush>;
        if is_code {
            let spans = code
                .provider
                .map(|p| p.highlight(text, code.language))
                .unwrap_or_default();
            for span in &spans {
                if span.category != HighlightCategory::Default {
                    let idx = syntax.brush_index(span.category);
                    builder.push(
                        parley::StyleProperty::Brush(BrushIndex(idx)),
                        span.byte_range.clone(),
                    );
                }
            }
            brushes = syntax.brushes();
        } else {
            brushes = vec![theme_text.into()];
        }

        let mut layout = builder.build(text);
        layout.break_all_lines(Some(wrap_width));

        let h = layout.height() as f64;
        lines.push(RenderedLine {
            layout,
            brushes,
            is_code,
            y: cursor_y,
            height: h,
        });
        cursor_y += h;
        if bi + 1 < blocks.len() {
            cursor_y += PARA_GAP;
        }
    }

    (lines, cursor_y)
}

// ── Public entry point ───────────────────────────────────────────────────────

/// Paint result — the widget uses this for hit-testing and scroll handling.
pub struct QuickHelpPaintResult {
    /// Bounding rect of the whole balloon (widget-local pixels).
    /// A click inside keeps the balloon open; a click outside closes it.
    pub balloon_rect: Rect,
    /// Total rendered content height (pixels). When this exceeds the
    /// balloon's inner height the caller knows scrolling is possible.
    pub content_height: f64,
}

/// Paint the Quick Help balloon anchored to the caret.
///
/// - `caret_x`, `caret_y`: widget-local caret top-left pixel.
/// - `line_height`: height of a single editor line.
/// - `viewport_width`, `viewport_height`: editor widget size.
/// - `scroll_offset`: vertical scroll into the content (0 = top).
/// - `theme_bg`, `theme_code_bg`, `theme_border`, `theme_text`: colours.
#[allow(clippy::too_many_arguments)]
pub fn paint_quick_help_balloon(
    ctx: &mut PaintCtx<'_>,
    painter: &mut Painter<'_>,
    text: &str,
    caret_x: f64,
    caret_y: f64,
    line_height: f64,
    viewport_width: f64,
    viewport_height: f64,
    scroll_offset: f64,
    theme_bg: Color,
    theme_code_bg: Color,
    theme_border: Color,
    theme_text: Color,
    syntax: &SyntaxPalette,
    code: CodeBlockHighlighter<'_>,
) -> QuickHelpPaintResult {
    let blocks = parse_blocks(text);
    if blocks.is_empty() {
        return QuickHelpPaintResult {
            balloon_rect: Rect::ZERO,
            content_height: 0.0,
        };
    }

    // Compute balloon width
    let balloon_width = (viewport_width - caret_x - PAD * 2.0 - 4.0)
        .max(MIN_WIDTH)
        .min(MAX_WIDTH);
    let content_width = balloon_width - PAD * 2.0;

    // Build text layouts at this width, with syntax highlighting for code blocks
    let (rendered, total_content_h) =
        build_layouts(ctx, &blocks, content_width, syntax, theme_text, code);
    let content_height = total_content_h;

    // Decide: pop down or up?
    let gap = 4.0;
    let space_below = viewport_height - (caret_y + line_height + gap);
    let space_above = caret_y - gap;

    // Cap balloon height to available space (with minimum so it's usable)
    let available = if space_below >= space_above {
        space_below
    } else {
        space_above
    };
    let balloon_inner_h = (content_height + PAD * 2.0).min(available.max(80.0));
    let balloon_h = balloon_inner_h;

    // X position — left-align with caret, clamp to viewport
    let balloon_x = caret_x.min(viewport_width - balloon_width - 4.0).max(4.0);

    // Y position
    let balloon_y = if space_below >= space_above {
        caret_y + line_height + gap
    } else {
        (caret_y - gap - balloon_h).max(4.0)
    };

    let balloon_rect = Rect::new(
        balloon_x,
        balloon_y,
        balloon_x + balloon_width,
        balloon_y + balloon_h,
    );

    // ── Paint background ────────────────────────────────────────────────────
    let rounded = RoundedRect::from_rect(balloon_rect, RADIUS);
    painter.fill(rounded, theme_bg).draw();
    use masonry::kurbo::Stroke;
    painter
        .stroke(rounded, &Stroke::new(1.0), theme_border)
        .draw();

    // ── Clip content to inner rect then paint lines ─────────────────────────
    let inner_x = balloon_x + PAD;
    let inner_y = balloon_y + PAD;
    let inner_h = balloon_h - PAD * 2.0;

    for rl in &rendered {
        let line_top = inner_y + rl.y - scroll_offset;
        let line_bottom = line_top + rl.height;

        // Skip lines entirely above the clip window
        if line_bottom < inner_y {
            continue;
        }
        // Stop once we're below the clip window
        if line_top > inner_y + inner_h {
            break;
        }

        // Draw code block background tint
        if rl.is_code {
            let code_rect = Rect::new(
                inner_x - 4.0,
                line_top.max(inner_y),
                inner_x + content_width + 4.0,
                line_bottom.min(inner_y + inner_h),
            );
            painter.fill(code_rect, theme_code_bg).draw();
        }

        render_text(
            painter,
            Affine::translate((inner_x, line_top)),
            &rl.layout,
            &rl.brushes,
            false,
        );
    }

    // ── Scroll indicator (thin bar on the right edge) ───────────────────────
    if content_height + PAD * 2.0 > balloon_h + 1.0 {
        let track_h = inner_h;
        let thumb_ratio = inner_h / (content_height + PAD * 2.0);
        let thumb_h = (thumb_ratio * track_h).max(16.0);
        let max_scroll = (content_height + PAD * 2.0 - balloon_h).max(1.0);
        let thumb_top = inner_y + (scroll_offset / max_scroll) * (track_h - thumb_h);
        let bar_x = balloon_x + balloon_width - 6.0;
        painter
            .fill(
                Rect::new(bar_x, thumb_top, bar_x + 3.0, thumb_top + thumb_h),
                masonry::peniko::Brush::Solid(theme_border),
            )
            .draw();
    }

    QuickHelpPaintResult {
        balloon_rect,
        content_height,
    }
}
