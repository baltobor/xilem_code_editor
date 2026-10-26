//! Lines — manages all line layouts and provides coordinate conversions.
//!
//! This is the brain of the viewport virtualization system. It:
//! - Caches one `LineLayout` per buffer line (computed on demand)
//! - Converts between buffer offsets, buffer lines, and visual lines
//! - Tracks the total document height for scrollbar sizing
//! - Only computes layouts for lines that are actually visible
//!
//! The design follows Lapce/Floem's Lines struct, adapted for Parley/Masonry.

use masonry::core::BrushIndex;
use masonry::parley::editing::Cursor as ParleyCursor;
use masonry::parley::layout::Affinity;
use masonry::parley::{FontContext, LayoutContext};

use crate::properties::SyntaxPalette;
use crate::text_layout::LineLayout;
use crate::text_layout::PhantomTextLine;
use code_editor_core::document::Document;
use code_editor_core::highlight::HighlightSpan;
use code_editor_core::layout::{RVLine, VLine};

/// Manages line layouts and coordinate conversions for the editor.
///
/// The key performance insight: we only compute Parley layouts for lines
/// that are currently visible in the viewport. A 100,000-line file might
/// only have 50 visible lines — so we only do work for those 50.
pub struct Lines {
    /// Per-line layout cache. Indexed by buffer line number.
    /// `None` means the layout hasn't been computed yet (or was invalidated).
    layouts: Vec<Option<LineLayout>>,

    /// Estimated line height in pixels (used when actual layout isn't computed).
    /// Set from the first computed layout, or from font_size * 1.2 as default.
    line_height: f64,

    /// Font size for building layouts.
    font_size: f32,

    /// Typeface family for building layouts.
    font_family: crate::properties::EditorFontFamily,

    /// Whether variable-style identifiers render italic.
    variable_italics: bool,

    /// Per-line syntax highlight spans.
    /// Indexed by buffer line number. Empty vec = no highlights for that line.
    highlights: Vec<Vec<HighlightSpan>>,

    /// Per-line phantom text (diagnostics, hints, etc.).
    /// Indexed by buffer line number.
    phantoms: Vec<PhantomTextLine>,
}

impl Lines {
    /// Create a new Lines manager for a document with the given line count.
    pub fn new(line_count: usize, font_size: f32) -> Self {
        let mut layouts = Vec::with_capacity(line_count);
        layouts.resize_with(line_count, || None);
        let mut highlights = Vec::with_capacity(line_count);
        highlights.resize_with(line_count, Vec::new);
        let mut phantoms = Vec::with_capacity(line_count);
        phantoms.resize_with(line_count, PhantomTextLine::empty);
        Self {
            layouts,
            line_height: font_size as f64 * 1.2,
            font_size,
            font_family: crate::properties::EditorFontFamily::default(),
            variable_italics: false,
            highlights,
            phantoms,
        }
    }

    /// Returns the estimated height of a single line.
    pub fn line_height(&self) -> f64 {
        self.line_height
    }

    /// Returns the font size used for layouts.
    pub fn font_size(&self) -> f32 {
        self.font_size
    }

    /// Returns the estimated width of a single character (for monospace fonts).
    /// Used for column guide positioning.
    pub fn char_width(&self) -> f64 {
        self.font_size as f64 * 0.6
    }

    /// Update font size — invalidates all cached layouts.
    pub fn set_font_size(&mut self, size: f32) {
        self.font_size = size;
        self.line_height = size as f64 * 1.2;
        self.invalidate_all();
    }

    /// Returns the typeface family used for layouts.
    pub fn font_family(&self) -> crate::properties::EditorFontFamily {
        self.font_family
    }

    /// Update the typeface family — invalidates all cached layouts.
    pub fn set_font_family(&mut self, family: crate::properties::EditorFontFamily) {
        if self.font_family != family {
            self.font_family = family;
            self.invalidate_all();
        }
    }

    /// Update the variable-italics toggle — invalidates all cached layouts.
    pub fn set_variable_italics(&mut self, italics: bool) {
        if self.variable_italics != italics {
            self.variable_italics = italics;
            self.invalidate_all();
        }
    }

    /// Resize the layout cache to match a new document line count.
    /// Called after edits that change the number of lines.
    pub fn resize(&mut self, new_line_count: usize) {
        self.layouts.resize_with(new_line_count, || None);
        self.highlights.resize_with(new_line_count, Vec::new);
        self.phantoms
            .resize_with(new_line_count, PhantomTextLine::empty);
    }

    /// Set syntax highlight spans for all lines.
    /// The outer Vec is indexed by buffer line number.
    pub fn set_highlights(&mut self, highlights: Vec<Vec<HighlightSpan>>) {
        // Rebuild only the lines whose colors changed. Invalidating every
        // layout would also drop the widths of lines scrolled out of view,
        // shrinking the content width and snapping horizontal scroll back.
        let line_count = self.highlights.len().max(highlights.len());
        for line in 0..line_count {
            if self.highlights.get(line) != highlights.get(line) {
                self.invalidate(line);
            }
        }
        self.highlights = highlights;
    }

    /// Check if a position (line, byte_col) is inside a string or comment.
    ///
    /// Used to suppress completion triggers inside strings/comments where
    /// code completions don't make sense. Returns true if cursor is in a
    /// non-code context.
    pub fn is_in_string_or_comment(&self, line: usize, byte_col: usize) -> bool {
        use code_editor_core::highlight::HighlightCategory;

        if line >= self.highlights.len() {
            return false;
        }

        for span in &self.highlights[line] {
            if span.byte_range.contains(&byte_col) {
                match span.category {
                    HighlightCategory::String | HighlightCategory::Comment => return true,
                    _ => {}
                }
            }
        }
        false
    }

    /// Set phantom text for a specific line.
    pub fn set_phantoms(&mut self, line: usize, phantoms: PhantomTextLine) {
        if line < self.phantoms.len() {
            self.phantoms[line] = phantoms;
            self.invalidate(line);
        }
    }

    /// Clear all phantom text.
    pub fn clear_phantoms(&mut self) {
        for p in &mut self.phantoms {
            *p = PhantomTextLine::empty();
        }
        self.invalidate_all();
    }

    /// Ensure a layout exists for the given buffer line. Computes it if missing.
    ///
    /// This is the lazy evaluation entry point — called only for visible lines
    /// during `ScreenLines::compute()`.
    pub fn ensure_layout(
        &mut self,
        line: usize,
        document: &dyn Document,
        font_ctx: &mut FontContext,
        layout_ctx: &mut LayoutContext<BrushIndex>,
        syntax_palette: &SyntaxPalette,
        max_width: Option<f32>,
    ) {
        if line >= self.layouts.len() {
            return;
        }

        let current_rev = document.revision();

        // Check if cached layout is still valid
        if let Some(ref cached) = self.layouts[line] {
            if cached.revision == current_rev && cached.wrap_width == max_width {
                // Defensive: if phantoms changed without going through
                // set_phantoms()/clear_phantoms(), the cached glyph positions
                // no longer match the current phantom mapping — that's the
                // intermittent click-misplaces-cursor bug class.
                #[cfg(debug_assertions)]
                {
                    let current_phantoms = self
                        .phantoms
                        .get(line)
                        .cloned()
                        .unwrap_or_else(PhantomTextLine::empty);
                    if !cached.phantoms.positioning_eq(&current_phantoms) {
                        tracing::error!(
                            line,
                            revision = current_rev,
                            "Lines::ensure_layout: stale phantom layout — some code path mutated phantoms without invalidating"
                        );
                    }
                }
                return; // Cache hit — nothing to do
            }
        }

        // Cache miss — build the layout
        let line_text = document
            .line_content(line)
            .map(|s| s.to_string())
            .unwrap_or_default();

        let empty_highlights: Vec<HighlightSpan> = Vec::new();
        let highlights = if line < self.highlights.len() {
            &self.highlights[line]
        } else {
            &empty_highlights
        };

        let phantoms = if line < self.phantoms.len() {
            &self.phantoms[line]
        } else {
            &PhantomTextLine::empty()
        };

        let layout = crate::text_layout::build_line_layout(
            font_ctx,
            layout_ctx,
            &line_text,
            phantoms,
            highlights,
            syntax_palette,
            self.font_size,
            self.font_family,
            self.variable_italics,
            max_width,
            current_rev,
        );

        // Update the estimated line height from actual computed layout.
        // For monospace text, all lines should have similar height,
        // so we use the first computed line as our estimate.
        if layout.visual_line_count == 1 {
            self.line_height = layout.line_height;
        }

        self.layouts[line] = Some(layout);
    }

    /// Get a reference to the cached layout for a line (if it exists).
    pub fn get_layout(&self, line: usize) -> Option<&LineLayout> {
        self.layouts.get(line)?.as_ref()
    }

    /// Get phantom text for a line (returns empty if out of bounds).
    pub fn get_phantoms(&self, line: usize) -> PhantomTextLine {
        self.phantoms
            .get(line)
            .cloned()
            .unwrap_or_else(PhantomTextLine::empty)
    }

    /// Invalidate the cached layout for a single line.
    pub fn invalidate(&mut self, line: usize) {
        if line < self.layouts.len() {
            self.layouts[line] = None;
        }
    }

    /// Invalidate all cached layouts (font size change, theme change, etc.).
    pub fn invalidate_all(&mut self) {
        for slot in &mut self.layouts {
            *slot = None;
        }
    }

    /// Estimate the total document height in pixels.
    ///
    /// For lines with computed layouts, use the actual height.
    /// For lines without layouts, use the estimated line_height.
    /// This gives a reasonable scrollbar size even before all lines are laid out.
    pub fn total_height(&self) -> f64 {
        let mut height = 0.0;
        for slot in &self.layouts {
            match slot {
                Some(layout) => height += layout.line_height,
                None => height += self.line_height,
            }
        }
        height
    }

    /// Compute the y position of a given buffer line.
    ///
    /// Sums the heights of all lines before the given line.
    pub fn y_of_line(&self, line: usize) -> f64 {
        let mut y = 0.0;
        for i in 0..line.min(self.layouts.len()) {
            y += match &self.layouts[i] {
                Some(layout) => layout.line_height,
                None => self.line_height,
            };
        }
        y
    }

    /// Find which buffer line is at a given y position.
    pub fn line_at_y(&self, y: f64) -> usize {
        let mut current_y = 0.0;
        for (i, slot) in self.layouts.iter().enumerate() {
            let h = match slot {
                Some(layout) => layout.line_height,
                None => self.line_height,
            };
            if current_y + h > y {
                return i;
            }
            current_y += h;
        }
        self.layouts.len().saturating_sub(1)
    }

    /// Convert a visual line (VLine) to a relative visual line (RVLine).
    pub fn vline_to_rvline(&self, vline: VLine) -> RVLine {
        let mut remaining = vline.0;
        for (line_idx, slot) in self.layouts.iter().enumerate() {
            let visual_count = match slot {
                Some(layout) => layout.visual_line_count,
                None => 1,
            };
            if remaining < visual_count {
                return RVLine::with_index(line_idx, remaining);
            }
            remaining -= visual_count;
        }
        // Past the end — return last line
        let last = self.layouts.len().saturating_sub(1);
        RVLine::new(last)
    }

    /// Convert a relative visual line (RVLine) to an absolute visual line (VLine).
    pub fn rvline_to_vline(&self, rvline: RVLine) -> VLine {
        let mut vline = 0;
        for (line_idx, slot) in self.layouts.iter().enumerate() {
            if line_idx == rvline.line {
                return VLine(vline + rvline.line_index);
            }
            let visual_count = match slot {
                Some(layout) => layout.visual_line_count,
                None => 1,
            };
            vline += visual_count;
        }
        VLine(vline)
    }

    /// Hit-test a line at a given x pixel position.
    ///
    /// Returns the buffer byte offset within the line where the x position falls.
    /// Used for click-to-cursor and column selection.
    ///
    /// This properly accounts for phantom text by converting the visual column
    /// back to a buffer column.
    pub fn hit_test_line(&self, line: usize, x: f64) -> Option<usize> {
        let layout = self.get_layout(line)?;
        // Same affinity caveat as `x_to_offset` — see that comment.
        let cursor = ParleyCursor::from_point(&layout.layout, x as f32, 0.0);
        let visual_col = cursor.index();

        // Use the phantom snapshot captured when this layout was built — see
        // offset_to_x for why.
        Some(layout.phantoms.buffer_col_at(visual_col))
    }

    /// Convert a byte offset in the document to an x pixel position.
    ///
    /// Used for:
    /// - Rendering the cursor caret
    /// - Preserving horizontal position during vertical movement
    ///
    /// This properly accounts for phantom text (inlay hints, diagnostics) which
    /// are rendered in the layout but don't exist in the buffer.
    pub fn offset_to_x(&self, doc: &dyn Document, offset: usize) -> f64 {
        let line = doc.line_of_offset(offset);
        let line_start = doc.offset_of_line(line);
        let buffer_col = offset - line_start;

        if let Some(layout) = self.get_layout(line) {
            // Use the phantom snapshot captured when this layout was built.
            // Reading from the (possibly newer) self.phantoms here would
            // misalign visual columns with the glyph positions Parley stored.
            let visual_col = layout.phantoms.col_at(buffer_col);

            // Use Parley's cursor geometry to find x position
            let cursor =
                ParleyCursor::from_byte_index(&layout.layout, visual_col, Affinity::Downstream);
            let geom = cursor.geometry(&layout.layout, 1.5);
            geom.x0 as f64
        } else {
            // No layout computed — estimate from character width
            buffer_col as f64 * (self.font_size as f64 * 0.6)
        }
    }

    /// Convert a byte offset to x position for selection painting.
    ///
    /// Unlike `offset_to_x()`, this uses `col_at_for_selection()` which
    /// ensures selections start AFTER phantom text at the same column.
    /// This prevents selection highlights from covering phantom text.
    pub fn offset_to_x_for_selection(&self, doc: &dyn Document, offset: usize) -> f64 {
        let line = doc.line_of_offset(offset);
        let line_start = doc.offset_of_line(line);
        let buffer_col = offset - line_start;

        if let Some(layout) = self.get_layout(line) {
            let visual_col = layout.phantoms.col_at_for_selection(buffer_col);

            let cursor =
                ParleyCursor::from_byte_index(&layout.layout, visual_col, Affinity::Downstream);
            let geom = cursor.geometry(&layout.layout, 1.5);
            geom.x0 as f64
        } else {
            buffer_col as f64 * (self.font_size as f64 * 0.6)
        }
    }

    /// Convert an x pixel position on a given line to a byte offset.
    ///
    /// The inverse of `offset_to_x()`. Used for:
    /// - Vertical cursor movement (up/down arrows)
    /// - Column selection
    ///
    /// This properly accounts for phantom text by converting the visual column
    /// back to a buffer column.
    pub fn x_to_offset(&self, doc: &dyn Document, line: usize, x: f64) -> usize {
        let line_start = doc.offset_of_line(line);

        if let Some(layout) = self.get_layout(line) {
            // Affinity asymmetry: Parley's `from_point` picks Upstream or
            // Downstream based on which half of the cluster the click hit,
            // while `offset_to_x` always renders with `Affinity::Downstream`.
            // Within a line both resolve to the same x, so we don't normalize
            // here — but at soft-wrap boundaries the same byte index can sit
            // at end-of-line-N (Upstream) or start-of-line-N+1 (Downstream),
            // and click vs. caret may pick different lines. If wrap-boundary
            // clicks ever misplace the caret, normalize affinity here.
            let cursor = ParleyCursor::from_point(&layout.layout, x as f32, 0.0);
            let visual_col = cursor.index();

            // Use the phantom snapshot captured when this layout was built — see
            // offset_to_x for why.
            let buffer_col = layout.phantoms.buffer_col_at(visual_col);

            line_start + buffer_col
        } else {
            // No layout — estimate
            let char_width = self.font_size as f64 * 0.6;
            let estimated_chars = (x / char_width).round() as usize;
            if let Some(content) = doc.line_content(line) {
                let max_chars = content.len_chars();
                let chars = estimated_chars.min(max_chars);
                let text = doc.text();
                let line_start_char = text.byte_to_char(line_start);
                text.char_to_byte(line_start_char + chars)
            } else {
                line_start
            }
        }
    }

    /// Compute the total number of visual lines in the document.
    pub fn total_visual_lines(&self) -> usize {
        self.layouts
            .iter()
            .map(|slot| match slot {
                Some(layout) => layout.visual_line_count,
                None => 1,
            })
            .sum()
    }
}

/// Compute the gutter width based on line count.
///
/// Dynamic width: enough digits for the highest line number, plus padding.
/// This is a free function so it can be called without borrowing Lines.
pub fn compute_gutter_width(line_count: usize) -> f64 {
    let digits = if line_count >= 10000 {
        5
    } else if line_count >= 1000 {
        4
    } else if line_count >= 100 {
        3
    } else {
        2
    };
    // ~8px per digit at 13px font size, plus left + right padding
    (digits as f64) * 8.0 + 32.0
}
