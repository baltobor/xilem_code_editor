//! PhantomText — virtual text overlays that appear in the editor but don't exist in the buffer.
//!
//! Phantom text is used for:
//! - Inline diagnostic messages (errors, warnings after the line)
//! - Inlay type hints (e.g. `: i32` after a let binding)
//! - AI code suggestions (ghost text)
//! - Git blame information at end of line
//!
//! Adapted from Lapce's PhantomText system. Uses SmallVec to avoid heap
//! allocation for the common case of 0-6 phantom entries per line.

use masonry::core::BrushIndex;
use masonry::parley;
use masonry::peniko::Color;
use smallvec::SmallVec;

/// What produced this piece of phantom text.
///
/// The kind determines default styling (color, underline) when
/// no explicit color override is provided.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
pub enum PhantomTextKind {
    /// Compiler diagnostic (error, warning, info, hint)
    Diagnostic,
    /// Type annotation inlay hint (e.g. `: String`)
    InlayHint,
    /// AI-generated code suggestion (ghost text)
    AiSuggestion,
    /// Git blame info at end of line
    GitBlame,
}

/// A single piece of virtual text to display at a specific position on a line.
///
/// Phantom text is visible to the user but does NOT exist in the document buffer.
/// The layout engine merges it into the display string before building the
/// Parley layout, and tracks the offset mapping so cursor/selection still
/// work correctly in buffer coordinates.
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct PhantomText {
    /// What produced this phantom text.
    pub kind: PhantomTextKind,
    /// Column position in the buffer line (byte offset within the line).
    /// The phantom text is inserted visually at this position.
    pub col: usize,
    /// The virtual text to display.
    pub text: String,
    /// Foreground color override (None = use kind-specific default).
    pub fg: Option<Color>,
    /// Background color override.
    pub bg: Option<Color>,
    /// Underline color (typically for diagnostic squiggles).
    pub underline: Option<Color>,
}

/// Manages all phantom text entries for a single buffer line.
///
/// Uses SmallVec to avoid heap allocation for the common case of few entries.
/// Entries are sorted by column position for efficient merging.
#[derive(Debug, Clone)]
pub struct PhantomTextLine {
    entries: SmallVec<[PhantomText; 6]>,
}

impl PhantomTextLine {
    /// Create an empty phantom text line (no virtual text).
    pub fn empty() -> Self {
        Self {
            entries: SmallVec::new(),
        }
    }

    /// Create a phantom text line from a list of entries.
    /// Entries are sorted by column position.
    pub fn new(mut entries: SmallVec<[PhantomText; 6]>) -> Self {
        entries.sort_by_key(|e| e.col);
        Self { entries }
    }

    /// Returns true if there are no phantom text entries.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Compare only the layout-affecting fields (`col`, `text.len()`) of each
    /// entry. Used by debug-mode cache-coherence checks — see
    /// `Lines::ensure_layout`. Colors and kind don't affect glyph positions.
    #[cfg(debug_assertions)]
    pub fn positioning_eq(&self, other: &Self) -> bool {
        if self.entries.len() != other.entries.len() {
            return false;
        }
        self.entries
            .iter()
            .zip(other.entries.iter())
            .all(|(a, b)| a.col == b.col && a.text.len() == b.text.len())
    }

    /// Add a phantom text entry, maintaining sort order.
    pub fn push(&mut self, entry: PhantomText) {
        let pos = self.entries.partition_point(|e| e.col <= entry.col);
        self.entries.insert(pos, entry);
    }

    /// Returns the entries.
    pub fn entries(&self) -> &[PhantomText] {
        &self.entries
    }

    /// Merge phantom text into the real line text, producing the display string.
    ///
    /// The display string is what Parley lays out. For example:
    /// - Buffer text: `let x = foo();`
    /// - Phantom: InlayHint at col 5 with text `: i32`
    /// - Display: `let x: i32 = foo();`
    ///
    /// The mapping between buffer columns and display columns is tracked
    /// by `col_at()` and `buffer_col_at()`.
    pub fn combine_with_text(&self, line_text: &str) -> String {
        if self.entries.is_empty() {
            return line_text.to_string();
        }

        let mut result = String::with_capacity(line_text.len() + self.total_phantom_len());
        let mut last_col = 0;

        for entry in &self.entries {
            // Clamp column to line length
            let col = entry.col.min(line_text.len());
            if col > last_col {
                result.push_str(&line_text[last_col..col]);
            }
            result.push_str(&entry.text);
            last_col = col;
        }

        // Append remaining real text
        if last_col < line_text.len() {
            result.push_str(&line_text[last_col..]);
        }

        result
    }

    /// Map a buffer column (byte offset within line) to a visual column
    /// in the display string (accounting for inserted phantom text).
    ///
    /// Uses strict less-than comparison (`<`) so the cursor stays BEFORE
    /// phantom text when positioned at the insertion point. This matches
    /// Lapce/Floem's behavior where inlay hints appear after the cursor,
    /// not before it.
    pub fn col_at(&self, buffer_col: usize) -> usize {
        let mut offset = 0usize;
        for entry in &self.entries {
            // Use `<` not `<=` so cursor at insertion point stays before phantom text
            if entry.col < buffer_col {
                offset += entry.text.len();
            } else {
                break;
            }
        }
        buffer_col + offset
    }

    /// Map a buffer column to visual column for selection painting.
    ///
    /// Unlike `col_at()`, this uses `<=` comparison so that selections
    /// starting at a buffer column will visually start AFTER any phantom
    /// text at that same column. This prevents selection highlights from
    /// incorrectly covering phantom text (like inlay hints).
    pub fn col_at_for_selection(&self, buffer_col: usize) -> usize {
        let mut offset = 0usize;
        for entry in &self.entries {
            // Use `<=` so selection starts AFTER phantom text at the same column
            if entry.col <= buffer_col {
                offset += entry.text.len();
            } else {
                break;
            }
        }
        buffer_col + offset
    }

    /// Map a visual column back to a buffer column.
    /// The inverse of `col_at()`.
    pub fn buffer_col_at(&self, visual_col: usize) -> usize {
        let mut phantom_offset = 0usize;
        let mut last_buffer_col = 0;

        for entry in &self.entries {
            let visual_pos = entry.col + phantom_offset;
            if visual_col < visual_pos {
                // We're before this phantom entry
                return visual_col - phantom_offset;
            }
            let phantom_end = visual_pos + entry.text.len();
            if visual_col < phantom_end {
                // We're inside the phantom text — map to the insertion column
                return entry.col;
            }
            phantom_offset += entry.text.len();
            last_buffer_col = entry.col;
        }

        // Past all phantom entries
        let _ = last_buffer_col;
        visual_col.saturating_sub(phantom_offset)
    }

    /// Apply phantom text styles to a Parley ranged builder.
    ///
    /// Called during layout building to color the phantom text portions
    /// of the display string. The `real_text_len` parameter is the length
    /// of the original line text (before phantom insertion).
    /// `phantom_brush_idx` is the brush index for phantom text (from SyntaxPalette).
    pub fn apply_styles(
        &self,
        builder: &mut parley::RangedBuilder<'_, BrushIndex>,
        real_text_len: usize,
        phantom_brush_idx: usize,
    ) {
        if self.entries.is_empty() {
            return;
        }

        let mut offset = 0usize;
        for entry in &self.entries {
            let col = entry.col.min(real_text_len);
            let visual_start = col + offset;
            let visual_end = visual_start + entry.text.len();

            if entry.fg.is_some() {
                builder.push(
                    parley::StyleProperty::Brush(BrushIndex(phantom_brush_idx)),
                    visual_start..visual_end,
                );
            }

            offset += entry.text.len();
        }
    }

    /// Total length of all phantom text entries.
    fn total_phantom_len(&self) -> usize {
        self.entries.iter().map(|e| e.text.len()).sum()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn combine_empty_phantoms() {
        let line = PhantomTextLine::empty();
        assert_eq!(line.combine_with_text("hello world"), "hello world");
    }

    #[test]
    fn combine_single_phantom() {
        let mut line = PhantomTextLine::empty();
        line.push(PhantomText {
            kind: PhantomTextKind::InlayHint,
            col: 5,
            text: ": i32".to_string(),
            fg: None,
            bg: None,
            underline: None,
        });
        assert_eq!(line.combine_with_text("let x = 42;"), "let x: i32 = 42;");
    }

    #[test]
    fn col_mapping() {
        let mut line = PhantomTextLine::empty();
        line.push(PhantomText {
            kind: PhantomTextKind::InlayHint,
            col: 5,
            text: ": i32".to_string(), // 5 bytes
            fg: None,
            bg: None,
            underline: None,
        });
        // Before phantom: col 3 in buffer = col 3 in visual
        assert_eq!(line.col_at(3), 3);
        // At phantom insertion point: col 5 in buffer = col 5 in visual
        // (cursor stays BEFORE phantom text, not after)
        assert_eq!(line.col_at(5), 5);
        // After phantom: col 6 in buffer = col 11 in visual (6 + 5 phantom bytes)
        assert_eq!(line.col_at(6), 11);
        // Further after: col 7 in buffer = col 12 in visual
        assert_eq!(line.col_at(7), 12);
    }

    #[test]
    fn buffer_col_mapping() {
        let mut line = PhantomTextLine::empty();
        line.push(PhantomText {
            kind: PhantomTextKind::InlayHint,
            col: 5,
            text: ": i32".to_string(),
            fg: None,
            bg: None,
            underline: None,
        });
        // Visual col 3 = buffer col 3
        assert_eq!(line.buffer_col_at(3), 3);
        // Visual col 7 (inside phantom) = buffer col 5
        assert_eq!(line.buffer_col_at(7), 5);
        // Visual col 12 = buffer col 7
        assert_eq!(line.buffer_col_at(12), 7);
    }
}
