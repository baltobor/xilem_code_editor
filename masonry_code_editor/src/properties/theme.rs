//! EditorTheme — all colors used by the code editor.
//!
//! The theme is the single source of truth for syntax highlighting colors.
//! `SyntaxPalette` defines the mapping from `HighlightCategory` to brush indices,
//! eliminating the implicit coupling that existed before.

use masonry::peniko::Brush;
use masonry::peniko::Color;

use code_editor_core::highlight::HighlightCategory;

/// Single source of truth for syntax highlighting colors.
///
/// The order of entries defines the brush indices:
/// - Index 0 is always `Default` (default text color)
/// - Subsequent indices correspond to the order in `entries`
/// - `phantom_text_fg` is always the last entry
///
/// This design ensures the brush index mapping can't get out of sync.
#[derive(Debug, Clone, PartialEq)]
pub struct SyntaxPalette {
    /// Ordered list of (category, color) pairs. Order defines brush index.
    entries: Vec<(HighlightCategory, Color)>,
    /// Phantom text color (inlay hints) - always last in the brush array.
    pub phantom_text_fg: Color,
}

impl SyntaxPalette {
    /// Create a new syntax palette with the given entries.
    /// The first entry MUST be `HighlightCategory::Default`.
    pub fn new(entries: Vec<(HighlightCategory, Color)>, phantom_text_fg: Color) -> Self {
        debug_assert!(
            entries.first().map(|(c, _)| *c) == Some(HighlightCategory::Default),
            "First entry must be Default category"
        );
        Self {
            entries,
            phantom_text_fg,
        }
    }

    /// Get the brush index for a highlight category.
    /// Returns 0 (default) if category is not found.
    pub fn brush_index(&self, category: HighlightCategory) -> usize {
        self.entries
            .iter()
            .position(|(c, _)| *c == category)
            .unwrap_or(0)
    }

    /// Get the color for a highlight category.
    pub fn color(&self, category: HighlightCategory) -> Color {
        self.entries
            .iter()
            .find(|(c, _)| *c == category)
            .map(|(_, color)| *color)
            .unwrap_or_else(|| {
                self.entries
                    .first()
                    .map(|(_, c)| *c)
                    .unwrap_or(Color::BLACK)
            })
    }

    /// Build the brush array for rendering.
    /// Order matches `brush_index()` lookups.
    pub fn brushes(&self) -> Vec<Brush> {
        let mut brushes: Vec<Brush> = self.entries.iter().map(|(_, c)| (*c).into()).collect();
        brushes.push(self.phantom_text_fg.into());
        brushes
    }

    /// Get the brush index for phantom text (always last).
    pub fn phantom_brush_index(&self) -> usize {
        self.entries.len()
    }
}

/// All colors used by the code editor, provided externally.
///
/// This allows the editor to be themed without internal dependencies on
/// any particular palette.
#[derive(Debug, Clone, PartialEq)]
pub struct EditorTheme {
    // --- Editor chrome ---
    pub editor_bg: Color,
    pub gutter_bg: Color,
    pub active_line_bg: Color,
    pub line_number: Color,
    pub cursor: Color,
    pub selection: Color,
    /// Search highlight color (visible orange/amber for search results).
    pub search_highlight: Color,

    // --- Syntax highlighting (single source of truth) ---
    pub syntax: SyntaxPalette,

    // --- Overlay colors ---
    pub diagnostic_error: Color,
    pub diagnostic_warning: Color,
    pub diagnostic_info: Color,
    pub column_selection_bg: Color,

    // --- Guides ---
    pub column_guide: Color,

    // --- Scrollbars ---
    pub scrollbar_track: Color,
    pub scrollbar_thumb: Color,
}

impl EditorTheme {
    /// A light theme, VS Code "Light+"-inspired. `EditorTheme` has no
    /// built-in light/dark switching of its own — callers pick
    /// `EditorTheme::default()` (dark) or this based on their own
    /// app-level dark-mode flag (see `blog_editor::editor_pane`).
    pub fn light() -> Self {
        Self {
            editor_bg: Color::from_rgb8(255, 255, 255),
            gutter_bg: Color::from_rgb8(245, 245, 245),
            active_line_bg: Color::from_rgb8(240, 240, 240),
            line_number: Color::from_rgb8(150, 150, 150),
            cursor: Color::from_rgb8(0, 0, 0),
            selection: Color::from_rgb8(173, 214, 255),
            search_highlight: Color::from_rgba8(255, 165, 50, 140),
            syntax: SyntaxPalette::new(
                vec![
                    (HighlightCategory::Default, Color::from_rgb8(30, 30, 30)),
                    (HighlightCategory::Keyword, Color::from_rgb8(0, 0, 255)),
                    (HighlightCategory::String, Color::from_rgb8(163, 21, 21)),
                    (HighlightCategory::Comment, Color::from_rgb8(0, 128, 0)),
                    (HighlightCategory::Function, Color::from_rgb8(121, 94, 38)),
                    (HighlightCategory::Type, Color::from_rgb8(38, 127, 153)),
                    (HighlightCategory::Number, Color::from_rgb8(9, 134, 88)),
                    (HighlightCategory::Operator, Color::from_rgb8(30, 30, 30)),
                    (HighlightCategory::Punctuation, Color::from_rgb8(30, 30, 30)),
                    (HighlightCategory::Macro, Color::from_rgb8(175, 80, 0)),
                    (HighlightCategory::Attribute, Color::from_rgb8(121, 94, 38)),
                    (
                        HighlightCategory::Constructor,
                        Color::from_rgb8(38, 127, 153),
                    ),
                    (HighlightCategory::Property, Color::from_rgb8(0, 16, 128)),
                    (HighlightCategory::Variable, Color::from_rgb8(0, 16, 128)),
                    (HighlightCategory::Heading, Color::from_rgb8(121, 60, 0)),
                ],
                Color::from_rgb8(170, 170, 170), // phantom_text_fg
            ),
            diagnostic_error: Color::from_rgb8(205, 49, 49),
            diagnostic_warning: Color::from_rgb8(191, 140, 0),
            diagnostic_info: Color::from_rgb8(0, 90, 190),
            column_selection_bg: Color::from_rgb8(200, 220, 255),
            column_guide: Color::from_rgb8(220, 220, 220),
            scrollbar_track: Color::from_rgba8(0, 0, 0, 20),
            scrollbar_thumb: Color::from_rgba8(0, 0, 0, 80),
        }
    }

    /// Build the ordered brush palette for `render_text`.
    /// Delegates to `SyntaxPalette::brushes()`.
    pub fn syntax_brushes(&self) -> Vec<Brush> {
        self.syntax.brushes()
    }

    /// Get the brush index for a highlight category.
    pub fn brush_index(&self, category: HighlightCategory) -> usize {
        self.syntax.brush_index(category)
    }

    /// Get the brush index for phantom text.
    pub fn phantom_brush_index(&self) -> usize {
        self.syntax.phantom_brush_index()
    }

    /// Get the default text color.
    pub fn default_text(&self) -> Color {
        self.syntax.color(HighlightCategory::Default)
    }

    /// Get the comment color (for reference).
    pub fn comment_color(&self) -> Color {
        self.syntax.color(HighlightCategory::Comment)
    }
}

impl Default for EditorTheme {
    fn default() -> Self {
        Self {
            editor_bg: Color::from_rgb8(30, 30, 30),
            gutter_bg: Color::from_rgb8(37, 37, 38),
            active_line_bg: Color::from_rgb8(45, 45, 48),
            line_number: Color::from_rgb8(150, 150, 150),
            cursor: Color::from_rgb8(78, 154, 234),
            selection: Color::from_rgb8(55, 55, 64),
            search_highlight: Color::from_rgba8(255, 165, 50, 180), // Bright amber/orange
            syntax: SyntaxPalette::new(
                vec![
                    (HighlightCategory::Default, Color::from_rgb8(212, 212, 212)),
                    (HighlightCategory::Keyword, Color::from_rgb8(78, 154, 234)),
                    (HighlightCategory::String, Color::from_rgb8(80, 200, 120)),
                    (HighlightCategory::Comment, Color::from_rgb8(150, 150, 150)),
                    (HighlightCategory::Function, Color::from_rgb8(78, 154, 234)),
                    (HighlightCategory::Type, Color::from_rgb8(229, 192, 56)),
                    (HighlightCategory::Number, Color::from_rgb8(80, 200, 120)),
                    (HighlightCategory::Operator, Color::from_rgb8(212, 212, 212)),
                    (
                        HighlightCategory::Punctuation,
                        Color::from_rgb8(212, 212, 212),
                    ),
                    (HighlightCategory::Macro, Color::from_rgb8(240, 150, 80)),
                    (
                        HighlightCategory::Attribute,
                        Color::from_rgb8(200, 170, 100),
                    ),
                    (
                        HighlightCategory::Constructor,
                        Color::from_rgb8(180, 140, 220),
                    ),
                    (HighlightCategory::Property, Color::from_rgb8(180, 140, 220)),
                    (HighlightCategory::Variable, Color::from_rgb8(180, 140, 220)),
                    // Markdown / outline-style headings. Warm gold so
                    // a heading line is immediately legible on the
                    // dark editor body. See `HighlightCategory::Heading`.
                    (HighlightCategory::Heading, Color::from_rgb8(255, 188, 90)),
                ],
                Color::from_rgb8(120, 120, 120), // phantom_text_fg
            ),
            diagnostic_error: Color::from_rgb8(255, 85, 85),
            diagnostic_warning: Color::from_rgb8(229, 192, 56),
            diagnostic_info: Color::from_rgb8(78, 154, 234),
            column_selection_bg: Color::from_rgb8(55, 55, 80),
            column_guide: Color::from_rgb8(50, 48, 45),
            scrollbar_track: Color::from_rgba8(60, 60, 60, 80),
            scrollbar_thumb: Color::from_rgba8(100, 100, 100, 180),
        }
    }
}
