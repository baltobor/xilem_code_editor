//! WidgetMut API — non-document state setters.
//!
//! Theme, breakpoints, execution-line marker, and the wide swath of
//! LSP-derived UI state pushed in by the View layer on rebuild
//! (overlays, diagnostic bubbles, completion popup items + ghost,
//! bracket ghost, etc.).

use std::collections::HashMap;

use crate::properties::EditorTheme;
use crate::text_layout::{PhantomText, PhantomTextKind, PhantomTextLine};
use crate::widgets::code_editor::CodeEditorWidget;
use code_editor_core::model::Diagnostic;

use code_editor_core::model::InlayHint;

impl CodeEditorWidget {
    // MARK: Theme + breakpoints + execution-line

    pub fn set_theme(this: &mut masonry::core::WidgetMut<'_, Self>, theme: EditorTheme) {
        this.widget.brushes = theme.syntax_brushes();
        this.widget.theme = Some(theme);
        this.widget.lines.invalidate_all();
        // Must request_layout (not just paint) because invalidate_all cleared
        // all cached line layouts — layout() needs to recompute them.
        this.ctx.request_layout();
    }

    /// Set breakpoint lines (1-based line numbers).
    pub fn set_breakpoints(this: &mut masonry::core::WidgetMut<'_, Self>, lines: Vec<usize>) {
        if this.widget.apply_breakpoints(lines) {
            this.ctx.request_paint_only();
        }
    }

    /// Set the current debugger-paused line (1-based), or `None`.
    pub fn set_execution_line(this: &mut masonry::core::WidgetMut<'_, Self>, line: Option<usize>) {
        if this.widget.apply_execution_line(line) {
            this.ctx.request_paint_only();
        }
    }

    /// Set all phantom text overlays (diagnostics + inlay hints) in one pass.
    ///
    /// Diagnostics are displayed via gutter markers, inline bubbles, and popovers (not ghost text).
    /// Inlay hints (type annotations) are displayed as inline ghost text.
    // MARK: LSP UI state — overlays, bubbles, completion popup, ghosts

    pub fn set_overlays(
        this: &mut masonry::core::WidgetMut<'_, Self>,
        diagnostics: &[Diagnostic],
        hints: &[InlayHint],
    ) {
        this.widget.apply_overlays(diagnostics, hints);
        this.ctx.request_layout();
    }

    /// Set the per-line cache of code actions to render inside an
    /// expanded diagnostic bubble. Replaces whatever was there.
    /// Keyed by 0-based line.
    pub fn set_bubble_code_actions(
        this: &mut masonry::core::WidgetMut<'_, Self>,
        cache: std::collections::HashMap<usize, Vec<code_editor_core::model::CodeAction>>,
    ) {
        this.widget.diagnostics.set_code_actions(cache);
        this.ctx.request_paint_only();
    }

    /// Push completion popup state from the host (AppModel) into the
    /// widget. The widget paints the popup itself anchored to the
    /// caret; this setter is the single channel for items / selected
    /// index / visibility.
    pub fn set_completion_popup(
        this: &mut masonry::core::WidgetMut<'_, Self>,
        items: Vec<code_editor_core::completion::CompletionItem>,
        selected: usize,
        visible: bool,
    ) {
        this.widget.completion.set_popup(items, selected, visible);
        this.ctx.request_paint_only();
    }

    /// Push Quick Help balloon state into the widget.
    /// `text` is the raw Markdown from the LSP hover response.
    /// `scroll` is the current scroll offset in pixels.
    pub fn set_quick_help(
        this: &mut masonry::core::WidgetMut<'_, Self>,
        text: Option<String>,
        scroll: f64,
    ) {
        this.widget.quick_help.set(text, scroll);
        this.ctx.request_paint_only();
    }

    /// Set the live VCS gutter diff (HEAD vs buffer).
    pub fn set_vcs_diff(
        this: &mut masonry::core::WidgetMut<'_, Self>,
        diff: Option<code_editor_core::vcs::VcsGutterDiff>,
    ) {
        this.widget.vcs.set_diff(diff);
        this.ctx.request_render();
    }

    /// Enable/disable the VCS gutter feature.
    pub fn set_vcs_gutter_enabled(this: &mut masonry::core::WidgetMut<'_, Self>, enabled: bool) {
        this.widget.vcs.set_enabled(enabled);
        this.ctx.request_render();
    }

    /// Set the bracket ghost character (closing bracket shown after cursor).
    pub fn set_bracket_ghost(this: &mut masonry::core::WidgetMut<'_, Self>, ghost: Option<char>) {
        if this.widget.completion.set_bracket_ghost(ghost) {
            this.ctx.request_paint_only();
        }
    }

    /// Set the completion ghost text (faded preview of selected completion).
    pub fn set_completion_ghost_text(
        this: &mut masonry::core::WidgetMut<'_, Self>,
        text: Option<String>,
    ) {
        if this.widget.completion.set_completion_ghost_text(text) {
            this.ctx.request_paint_only();
        }
    }

    /// Clear all ghost text (bracket and completion).
    pub fn clear_ghost_text(this: &mut masonry::core::WidgetMut<'_, Self>) {
        if this.widget.completion.clear_ghost() {
            this.ctx.request_paint_only();
        }
    }

    /// Set whether the completion popup is visible.
    /// When visible, Up/Down navigate the list and Enter accepts the selection
    /// (gated by `popup_user_navigated`).
    pub fn set_completion_visible(this: &mut masonry::core::WidgetMut<'_, Self>, visible: bool) {
        this.widget.completion.set_visible(visible);
    }

    /// Mirror of `AppModel.popup_user_navigated`. The view
    /// layer pumps this in every rebuild so the input router
    /// can arm/disarm Return without re-querying the AppModel
    /// for every keypress.
    pub fn set_popup_user_navigated(
        this: &mut masonry::core::WidgetMut<'_, Self>,
        navigated: bool,
    ) {
        // The gutter `⏎` glyph paints based on this flag,
        // so a flag flip needs a repaint.
        if this.widget.completion.set_popup_user_navigated(navigated) {
            this.ctx.request_paint_only();
        }
    }

    /// Push the block-mode payload (single full-preview code-action)
    /// into the widget. `Some((title, preview, idx, total))`
    /// enables block-mode rendering; `None` disables it. The
    /// painter renders an `n/m` header in the title row when
    /// `total > 1` so the user knows Up/Down navigates among
    /// alternative snippet sources.
    pub fn set_completion_block_action(
        this: &mut masonry::core::WidgetMut<'_, Self>,
        action: Option<(String, String, usize, usize)>,
    ) {
        if this.widget.completion.set_block_action(action) {
            this.ctx.request_paint_only();
        }
    }
}

// MARK: Shared by the WidgetMut setters and the build-time `*_direct` builders.
impl CodeEditorWidget {
    /// Returns whether anything changed.
    pub(crate) fn apply_breakpoints(&mut self, lines: Vec<usize>) -> bool {
        let changed = self.breakpoints != lines;
        self.breakpoints = lines;
        changed
    }

    /// Returns whether anything changed.
    pub(crate) fn apply_execution_line(&mut self, line: Option<usize>) -> bool {
        let changed = self.execution_line != line;
        self.execution_line = line;
        changed
    }

    /// Store diagnostics (gutter markers + inline bubbles; never phantom
    /// text) and lay out inlay hints as phantom text.
    pub(crate) fn apply_overlays(&mut self, diagnostics: &[Diagnostic], hints: &[InlayHint]) {
        self.lines.clear_phantoms();

        // Store full diagnostic data + re-derive gutter markers (one per
        // line, highest severity wins). Diagnostics are NOT displayed as
        // inline phantom text - only gutter markers + inline bubbles.
        self.diagnostics.set_diagnostics(diagnostics.to_vec());

        // Group phantoms by line - only inlay hints, no diagnostic ghost text
        let mut by_line: HashMap<usize, PhantomTextLine> = HashMap::new();

        for hint in hints {
            let entry = by_line
                .entry(hint.line)
                .or_insert_with(PhantomTextLine::empty);
            // Smart spacing based on hint kind:
            // - Type hints (`: Type`) get leading space for symmetry
            // - Parameter hints (`param:`) don't need leading space
            let text = match hint.kind {
                code_editor_core::model::InlayHintKind::Type => format!(" {}", hint.text),
                code_editor_core::model::InlayHintKind::Parameter => hint.text.clone(),
                code_editor_core::model::InlayHintKind::Other => {
                    // For unknown kinds, check if it starts with colon (type-like)
                    if hint.text.starts_with(':') {
                        format!(" {}", hint.text)
                    } else {
                        hint.text.clone()
                    }
                }
            };
            entry.push(PhantomText {
                kind: PhantomTextKind::InlayHint,
                col: hint.col,
                text,
                fg: Some(masonry::peniko::Color::from_rgb8(130, 130, 130)),
                bg: Some(masonry::peniko::Color::from_rgba8(180, 180, 180, 45)), // subtle background
                underline: None,
            });
        }

        for (line, phantoms) in by_line {
            self.lines.set_phantoms(line, phantoms);
        }
    }
}
