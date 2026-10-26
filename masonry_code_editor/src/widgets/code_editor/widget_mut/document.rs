//! WidgetMut API — document text + display config.
//!
//! `set_text` does a wholesale replacement (drops the undo log, used
//! when an external source like file-open hands us a fresh string).
//! Display setters cover brushes / line-numbers / font-size /
//! tab-size / language / highlights.

use masonry::peniko::Brush;

use code_editor_core::cursor::Cursor;
use std::sync::Arc;

use crate::widgets::code_editor::CodeEditorWidget;
use code_editor_core::document::RopeDocument;
use code_editor_core::highlight::HighlightSpan;
use code_editor_core::provider::{HighlightPostPass, HighlightProvider};

impl CodeEditorWidget {
    /// Set the text content, replacing the entire document.
    ///
    /// This is the wholesale-replacement path used when an
    /// out-of-band edit (LSP completion accept, quick-fix workspace
    /// edit, file load) hands us a freshly-built `tab.content` that
    /// no longer matches the rope we own. We rebuild the rope and
    /// **drop the entire undo history** — the recorded transactions
    /// reference byte ranges in the previous rope, and replaying
    /// them against the new rope panics. Losing undo on those out-
    /// of-band paths is the lesser evil; a long-term fix would route
    /// every external edit through the widget's edit-group machinery
    /// (`perform_insert` / similar) so the new edit is recorded
    /// instead of obliterating history.
    // MARK: Document — set_text (wholesale replacement, drops undo)

    pub fn set_text(this: &mut masonry::core::WidgetMut<'_, Self>, text: &str) {
        let current = this.widget.document.to_string();
        if current == text {
            // Wrapper has caught up to the widget — no work needed.
            // Clear any stashed post-edit snapshot since we're
            // confirmed in sync.
            this.widget.pending_workspace_edit_text = None;
            return;
        }

        // Stale-rebuild guard: when an LSP-driven mutation
        // (`perform_workspace_edits` / `perform_text_edit`) just
        // updated the widget's document, the wrapper's `tab.content`
        // may still reflect the pre-edit text on the very next
        // rebuild. Without this guard, we'd wholesale-replace the
        // widget's document with the stale `tab.content` AND wipe
        // the undo group the workspace-edit just recorded — the
        // user's Cmd+Z would have nothing to reverse.
        //
        // The snapshot recognises this gap: if the widget's current
        // text matches the stashed post-edit text, the wrapper is
        // simply lagging. Skip the rollback + history wipe and let
        // the next `ContentChanged` round trip catch the wrapper up;
        // the equality branch above will then clear the snapshot.
        if let Some(ref expected) = this.widget.pending_workspace_edit_text {
            if &current == expected {
                return;
            }
        }

        // Genuine wholesale replacement (file open, project switch,
        // hot-reload, etc.). Drop any in-flight undo history before
        // swapping — the new `RopeDocument` below starts with its
        // own empty undo stack, but if a future refactor reuses the
        // existing document and just `edit()`s it instead, the old
        // transactions would still be present and `redo` / `undo`
        // would replay against the wrong text.
        this.widget.pending_workspace_edit_text = None;
        this.widget.document.clear_history();
        // Replace the entire document
        this.widget.document = Box::new(RopeDocument::from_str(text));
        let line_count = this.widget.document.line_count();
        this.widget.lines.resize(line_count);
        this.widget.lines.invalidate_all();
        this.widget.viewport.last_synced_revision = this.widget.document.revision();
        this.widget.viewport.max_content_width = 0.0;
        // Reset cursor to start if it's beyond the new text
        let doc_len = this.widget.document.len_bytes();
        if this.widget.cursor.offset() > doc_len {
            this.widget.cursor = Cursor::new(0);
        }
        // Re-highlight with the new text
        this.widget.rehighlight();
        this.ctx.request_layout();
    }

    /// Set the brush palette used for rendering.
    // MARK: Display config — brushes, line numbers, font, tabs, language, highlights

    pub fn set_brushes(this: &mut masonry::core::WidgetMut<'_, Self>, brushes: Vec<Brush>) {
        this.widget.brushes = brushes;
        // Brush changes don't affect layout geometry — only colors.
        // No need to invalidate layouts since BrushIndex values are the same.
        this.ctx.request_paint_only();
    }

    /// Toggle the line numbers gutter.
    pub fn set_show_line_numbers(this: &mut masonry::core::WidgetMut<'_, Self>, show: bool) {
        if this.widget.config.show_line_numbers != show {
            this.widget.config.show_line_numbers = show;
            this.ctx.request_layout();
        }
    }

    /// Turn soft wrap on or off; relays out every line.
    pub fn set_word_wrap(this: &mut masonry::core::WidgetMut<'_, Self>, wrap: bool) {
        if this.widget.config.word_wrap != wrap {
            this.widget.config.word_wrap = wrap;
            this.widget.lines.invalidate_all();
            this.ctx.request_layout();
        }
    }

    /// Set the font size.
    pub fn set_font_size(this: &mut masonry::core::WidgetMut<'_, Self>, size: f32) {
        this.widget.lines.set_font_size(size);
        this.ctx.request_layout();
    }

    /// Set the editor typeface family.
    pub fn set_font_family(
        this: &mut masonry::core::WidgetMut<'_, Self>,
        family: crate::properties::EditorFontFamily,
    ) {
        this.widget.lines.set_font_family(family);
        this.ctx.request_layout();
    }

    /// Set the variable-italics toggle.
    pub fn set_variable_italics(this: &mut masonry::core::WidgetMut<'_, Self>, italics: bool) {
        this.widget.lines.set_variable_italics(italics);
        this.ctx.request_layout();
    }

    /// Set the tab size (number of spaces).
    pub fn set_tab_size(this: &mut masonry::core::WidgetMut<'_, Self>, size: usize) {
        this.widget.config.tab_size = size;
    }

    /// Set the language for syntax highlighting.
    pub fn set_language(this: &mut masonry::core::WidgetMut<'_, Self>, lang: &str) {
        if this.widget.language != lang {
            this.widget.language = lang.to_string();
            this.widget.rehighlight();
            this.ctx.request_layout();
        }
    }

    /// Set the syntax highlighting source (`None`: no highlighting).
    pub fn set_highlighter(
        this: &mut masonry::core::WidgetMut<'_, Self>,
        highlighter: Option<Arc<dyn HighlightProvider>>,
    ) {
        this.widget.highlighter = highlighter;
        this.widget.rehighlight();
        this.ctx.request_layout();
    }

    /// Set the post-passes applied after the highlighter.
    pub fn set_highlight_passes(
        this: &mut masonry::core::WidgetMut<'_, Self>,
        passes: Vec<Arc<dyn HighlightPostPass>>,
    ) {
        this.widget.highlight_passes = passes;
        this.widget.rehighlight();
        this.ctx.request_layout();
    }

    /// Set custom highlights (bypasses tree-sitter).
    /// Used for terminal output, compiler messages with ANSI colors, etc.
    pub fn set_highlights(
        this: &mut masonry::core::WidgetMut<'_, Self>,
        highlights: Vec<Vec<HighlightSpan>>,
    ) {
        this.widget.lines.set_highlights(highlights);
        this.ctx.request_layout();
    }
}
