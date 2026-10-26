//! LineLayout — cached text layout for a single buffer line.
//!
//! Each buffer line gets its own `parley::Layout<BrushIndex>` built via
//! Parley's raw `LayoutContext::ranged_builder()`. We do NOT use PlainEditor —
//! the Rope document is the single source of truth for text content.
//!
//! The layout is cached and only recomputed when the line's content changes
//! (detected via the document revision counter).

use masonry::core::BrushIndex;
use masonry::parley::{self, FontContext, LayoutContext};

use crate::properties::EditorFontFamily;
use crate::properties::SyntaxPalette;
use crate::text_layout::PhantomTextLine;
use code_editor_core::highlight::{HighlightCategory, HighlightSpan};

/// Cached text layout for a single buffer line.
///
/// Contains the Parley layout plus metadata about visual dimensions.
/// Stored in the `Lines` manager and indexed by buffer line number.
pub struct LineLayout {
    /// The Parley layout for this line's text (with phantom text merged in).
    pub layout: parley::Layout<BrushIndex>,
    /// Total height of this line in pixels (may span multiple visual lines if wrapped).
    pub line_height: f64,
    /// Number of visual lines this buffer line occupies (1 if no wrap, 2+ if wrapped).
    pub visual_line_count: usize,
    /// Document revision when this layout was computed — used for cache invalidation.
    pub revision: u64,
    /// Wrap width the layout was broken at (`None`: no wrap). A different
    /// width (window resized) makes the cached layout stale.
    pub wrap_width: Option<f32>,
    /// Snapshot of the phantom text used when building this layout. Hit-test and
    /// caret rendering must read from this, not the current `Lines::phantoms`, or
    /// visual↔buffer column mapping can drift out of sync with the glyph positions.
    pub phantoms: PhantomTextLine,
}

/// Build a Parley Layout for a single line of text.
///
/// This is the core layout function — called once per visible line per frame
/// (or less, if the cache is still valid). It:
/// 1. Merges phantom text into the display string
/// 2. Applies syntax highlight spans as BrushIndex ranges
/// 3. Builds the Parley layout with monospace font
///
/// # Arguments
/// - `font_ctx`: Parley font context (from Masonry's text_contexts())
/// - `layout_ctx`: Parley layout context (from Masonry's text_contexts())
/// - `text`: The actual line text from the Rope (no trailing newline)
/// - `phantoms`: Virtual text entries to merge (diagnostics, hints, etc.)
/// - `highlights`: Syntax coloring spans for this line
/// - `syntax_palette`: The syntax color palette (provides brush index lookup)
/// - `font_size`: Font size in points
/// - `font_family`: Typeface family (serif monospace vs sans-serif coding face)
/// - `variable_italics`: When true, render `Constructor`-category identifiers
///   (enum variants, constructors, SCREAMING_CASE constants) in italic
/// - `max_width`: Maximum width for word wrapping (None = no wrap)
/// - `revision`: Current document revision for cache tagging
pub fn build_line_layout(
    font_ctx: &mut FontContext,
    layout_ctx: &mut LayoutContext<BrushIndex>,
    text: &str,
    phantoms: &PhantomTextLine,
    highlights: &[HighlightSpan],
    syntax_palette: &SyntaxPalette,
    font_size: f32,
    font_family: EditorFontFamily,
    variable_italics: bool,
    max_width: Option<f32>,
    revision: u64,
) -> LineLayout {
    // Merge phantom text into the display string.
    // The combined string is what Parley will lay out, but the highlight spans
    // still reference the original text positions — phantom_text handles the mapping.
    let display_text = phantoms.combine_with_text(text);

    // Build the Parley layout using the ranged builder API
    let mut builder = layout_ctx.ranged_builder(font_ctx, &display_text, 1.0, true);

    // Default styles: chosen typeface (OS-resolved fallback stack), specified size
    builder.push_default(parley::StyleProperty::FontSize(font_size));
    builder.push_default(parley::StyleProperty::from(font_family.families()));
    builder.push_default(parley::StyleProperty::LineHeight(
        parley::LineHeight::FontSizeRelative(1.2),
    ));
    // Default brush index 0 (default text color)
    builder.push_default(parley::StyleProperty::Brush(BrushIndex(0)));

    // Apply syntax highlight spans.
    // Each span maps a byte range to a BrushIndex via the syntax palette.
    let display_bytes = display_text.as_bytes();
    // Snap a byte index forward to the next UTF-8 char boundary so
    // we never hand parley a range whose endpoint sits inside a
    // multi-byte glyph (`→`, etc.). Mirrors the snap in
    // `highlight::split_into_lines`; defensive against any path
    // that produces a misaligned span.
    let snap = |idx: usize| -> usize {
        let mut i = idx.min(display_bytes.len());
        while i < display_bytes.len() && (display_bytes[i] & 0b1100_0000) == 0b1000_0000 {
            i += 1;
        }
        i
    };
    for span in highlights {
        // Map the original text byte range through phantom text offsets
        let visual_start = snap(phantoms.col_at(span.byte_range.start));
        let visual_end = snap(phantoms.col_at(span.byte_range.end));
        if visual_start < visual_end && visual_end <= display_text.len() {
            let brush_idx = syntax_palette.brush_index(span.category);
            builder.push(
                parley::StyleProperty::Brush(BrushIndex(brush_idx)),
                visual_start..visual_end,
            );
            // Italicize constructor-shaped identifiers when the readability
            // toggle is on — enum variants (`Some`, `Character`), tuple/unit
            // structs, and SCREAMING_CASE constants (`MAX_LEN`, `PI`), which
            // tree-sitter routes to `Constructor`. RustRover-style: struct
            // fields (`masonry_state`), methods (`set_clipboard`), plain
            // locals and types stay upright.
            if variable_italics && span.category == HighlightCategory::Constructor {
                builder.push(
                    parley::StyleProperty::FontStyle(parley::FontStyle::Italic),
                    visual_start..visual_end,
                );
            }
        }
    }

    // Apply phantom text styling (muted colors, underlines for diagnostics)
    let phantom_brush_idx = syntax_palette.phantom_brush_index();
    phantoms.apply_styles(&mut builder, text.len(), phantom_brush_idx);

    // Build and break the layout
    let mut layout = builder.build(&display_text);
    layout.break_all_lines(max_width);

    // Compute visual metrics
    let line_height = layout.height() as f64;
    let visual_line_count = layout.lines().count().max(1);

    LineLayout {
        layout,
        line_height,
        visual_line_count,
        revision,
        wrap_width: max_width,
        phantoms: phantoms.clone(),
    }
}
