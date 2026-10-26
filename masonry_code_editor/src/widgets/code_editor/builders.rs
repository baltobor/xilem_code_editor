//! Constructor + public setters for `CodeEditorWidget`.
//!
//! `new` allocates the rope-backed document and zeroes every state
//! field. The remaining methods are direct setters used by the
//! `editor_pane_view` / `view::CodeEditor` builders to flow theme,
//! language, line-number visibility, trigger-chars, bracket mode,
//! and goto-line requests into a freshly-built widget before the
//! first frame.

use crate::properties::EditorTheme;
use crate::text_layout::Lines;
use crate::text_layout::ViewportState;
use code_editor_core::config::EditorConfig;
use code_editor_core::cursor::Cursor;
use code_editor_core::document::Document;
use code_editor_core::document::RopeDocument;
use code_editor_core::highlight::HighlightSpan;
use code_editor_core::timing::CursorAnimator;

use super::CodeEditorWidget;

// MARK: BUILDERS

impl CodeEditorWidget {
    /// Create a new code editor widget with the given text.
    pub fn new(text: &str, font_size: f32) -> Self {
        let document = Box::new(RopeDocument::from_str(text));
        let line_count = document.line_count();

        Self {
            cursor: Cursor::new(0),
            lines: Lines::new(line_count, font_size),
            screen_lines: None,
            document,
            theme: None,
            brushes: vec![],
            config: EditorConfig::new(),
            viewport: ViewportState::new(),
            animator: CursorAnimator::new(),
            highlighter: None,
            highlight_passes: Vec::new(),
            language: String::new(),
            breakpoints: Vec::new(),
            execution_line: None,
            diagnostics: crate::state::DiagnosticBubbleState::default(),
            section_markers: Vec::new(),
            section_markers_dirty: false,
            gutter_registry: Self::create_default_gutter_registry(line_count),
            // Internal scroll management (derived from Portal)
            scroll: crate::scroll::ScrollState::default(),
            completion: crate::state::CompletionState::default(),
            bracket_mode: code_editor_core::model::BracketMode::default(),
            is_search_highlight: false,
            pending_scroll_alignment: code_editor_core::model::GotoAlignment::Edge,
            snippet_session: None,
            pending_workspace_edit_text: None,
            quick_help: crate::state::QuickHelpState::default(),
            hover: crate::state::HoverState::default(),
            vcs: crate::state::VcsGutterState::default(),
        }
    }

    /// Set the theme directly on the widget (before it's wrapped in a Pod).
    /// Used during `build()` so the widget has correct colors from the first paint.
    pub fn set_theme_direct(&mut self, theme: EditorTheme) {
        self.brushes = theme.syntax_brushes();
        self.theme = Some(theme);
    }

    /// Set the goto definition click mode directly on the widget.
    pub fn set_goto_def_click_mode(&mut self, mode: u8) {
        self.config.set_goto_def_click_mode(mode);
    }

    /// Set the editor typeface family directly (before the widget is wrapped
    /// in a Pod), so the first paint already uses the chosen font.
    pub fn set_font_family_direct(&mut self, family: crate::properties::EditorFontFamily) {
        self.lines.set_font_family(family);
    }

    /// Set the variable-italics toggle directly (before the widget is wrapped
    /// in a Pod).
    pub fn set_variable_italics_direct(&mut self, italics: bool) {
        self.lines.set_variable_italics(italics);
    }

    /// Set the VCS gutter diff directly (before the widget is wrapped in a Pod).
    pub fn set_vcs_diff_direct(&mut self, diff: Option<code_editor_core::vcs::VcsGutterDiff>) {
        self.vcs.set_diff(diff);
    }

    /// Enable/disable the VCS gutter feature directly.
    pub fn set_vcs_gutter_enabled_direct(&mut self, enabled: bool) {
        self.vcs.set_enabled(enabled);
    }

    /// Set show_line_numbers directly on the widget (before it's wrapped in a Pod).
    pub fn set_show_line_numbers_direct(&mut self, show: bool) {
        self.config.show_line_numbers = show;
    }

    /// Set the completion trigger characters from LSP capabilities.
    pub fn set_trigger_chars(&mut self, chars: Vec<String>) {
        self.completion.set_trigger_chars(chars);
    }

    /// Set the bracket auto-closing mode.
    pub fn set_bracket_mode(&mut self, mode: code_editor_core::model::BracketMode) {
        self.bracket_mode = mode;
    }

    /// Set soft wrap directly on the widget (before it's wrapped in a Pod).
    pub fn set_word_wrap_direct(&mut self, wrap: bool) {
        self.config.word_wrap = wrap;
    }

    /// Breakpoint lines (1-based) before the widget is in a Pod.
    pub fn set_breakpoints_direct(&mut self, lines: Vec<usize>) {
        self.apply_breakpoints(lines);
    }

    /// Debugger execution line (1-based) before the widget is in a Pod.
    pub fn set_execution_line_direct(&mut self, line: Option<usize>) {
        self.apply_execution_line(line);
    }

    /// Diagnostics and inlay hints before the widget is in a Pod.
    pub fn set_overlays_direct(
        &mut self,
        diagnostics: &[code_editor_core::model::Diagnostic],
        hints: &[code_editor_core::model::InlayHint],
    ) {
        self.apply_overlays(diagnostics, hints);
    }

    /// Highlighting source and post-passes before the widget is in a Pod.
    pub fn set_highlighting_direct(
        &mut self,
        highlighter: Option<std::sync::Arc<dyn code_editor_core::provider::HighlightProvider>>,
        passes: Vec<std::sync::Arc<dyn code_editor_core::provider::HighlightPostPass>>,
    ) {
        self.highlighter = highlighter;
        self.highlight_passes = passes;
        self.rehighlight();
    }

    /// Tab size (spaces) before the widget is in a Pod.
    pub fn set_tab_size_direct(&mut self, size: usize) {
        self.config.tab_size = size;
    }

    /// Add an app-specific gutter control (e.g. a "run" button) before
    /// the widget is in a Pod. It sees the current document immediately.
    pub fn register_gutter_control_direct(
        &mut self,
        control: Box<dyn crate::gutter::GutterControl>,
    ) {
        self.gutter_registry.register(control);
        self.gutter_registry
            .document_changed(self.document.as_ref());
    }

    /// Set the language directly on the widget (before it's wrapped in a Pod).
    pub fn set_language_direct(&mut self, lang: impl Into<String>) {
        self.language = lang.into();
        self.rehighlight();
    }

    /// Set custom highlights directly on the widget (bypasses tree-sitter).
    /// Used for terminal output, compiler messages with ANSI colors, etc.
    pub fn set_highlights_direct(&mut self, highlights: Vec<Vec<HighlightSpan>>) {
        self.lines.set_highlights(highlights);
    }

    /// Set the cursor position to a line/column directly during widget construction.
    /// Used when the widget is first built with a pending_goto_line.
    /// Line and column are 1-based. If `highlight_len` is provided, selects that many characters.
    /// `alignment` controls how the destination is placed in the viewport.
    /// Marker: Structure Click Appearance: Click shows the function at the
    /// top of the text editor — `Top` is honored here.
    pub fn set_goto_line_direct(
        &mut self,
        line: usize,
        col: usize,
        highlight_len: Option<usize>,
        alignment: code_editor_core::model::GotoAlignment,
    ) {
        let line_count = self.document.line_count();
        let line = line.max(1).min(line_count);
        let line_idx = line - 1;

        // Get the byte offset for the start of the line
        let line_start = self.document.offset_of_line(line_idx);

        // Calculate the offset for the column
        let (byte_offset, line_str) =
            if let Some(line_content) = self.document.line_content(line_idx) {
                let col_idx = col.saturating_sub(1);
                let content_str = line_content.to_string();
                let mut offset = line_start;
                for (i, c) in content_str.chars().enumerate() {
                    if i >= col_idx {
                        break;
                    }
                    offset += c.len_utf8();
                }
                (
                    offset.min(line_start + line_content.len_bytes()),
                    content_str,
                )
            } else {
                (line_start, String::new())
            };

        // Set the cursor position or selection
        if let Some(len) = highlight_len {
            // Calculate end offset by advancing `len` characters
            let col_idx = col.saturating_sub(1);
            let mut end_offset = byte_offset;
            for (i, c) in line_str.chars().skip(col_idx).enumerate() {
                if i >= len {
                    break;
                }
                end_offset += c.len_utf8();
            }
            self.cursor.selection.set_single(byte_offset, end_offset);
        } else {
            self.cursor.selection.set_caret(byte_offset);
        }
        self.cursor.horiz = None;
        // External jump — store the requested alignment so the next scroll
        // honors it. Marker: Structure Click Appearance: Click shows the
        // function at the top of the text editor.
        self.pending_scroll_alignment = alignment;

        // Request scroll to cursor on first frame
        self.viewport.request_scroll_to_cursor();
    }
}
