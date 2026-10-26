//! State for the inline diagnostic bubbles (Xcode-style popovers) on
//! `CodeEditorWidget`: gutter markers, the full diagnostic list, the
//! per-line quick-fix cache, and which bubble (if any) is expanded.
//!
//! Extracted from eight loose fields (`diagnostic_markers`/
//! `inline_diagnostics`/`diagnostic_bubble_bounds`/
//! `diagnostic_action_bounds`/`bubble_code_actions_view`/
//! `expanded_diagnostic_line`/`bubble_actions_requested_for_line`/
//! `selected_action_index`) as part of the CodeEditorWidget SOLID
//! rework. The expand/collapse/cycle logic used to be duplicated
//! inline at each of three call sites (pointer toggle, Tab/Enter
//! cycling, Escape collapse) — it now lives once, on the type that
//! owns the invariants:
//! - collapsing always resets the same three fields together
//!   (expanded line, requested-for-line marker, selected index)
//! - expanding always resets the selected index to 0
//! - the selected-action index only ever moves within the bounds of
//!   the expanded line's current quick-fix count

use masonry::kurbo::{Point, Rect};

use code_editor_core::model::CodeAction;
use code_editor_core::model::{Diagnostic, DiagnosticSeverity};

#[derive(Debug, Clone, Default)]
pub struct DiagnosticBubbleState {
    /// Diagnostic line markers (line, severity) for gutter painting.
    /// Line numbers are 0-based.
    markers: Vec<(usize, DiagnosticSeverity)>,
    /// Full diagnostic data for inline popover rendering.
    inline_diagnostics: Vec<Diagnostic>,
    /// Rendered popover bounds for click detection: (line, bubble_rect).
    bubble_bounds: Vec<(usize, Rect)>,
    /// Hit-test rectangles for quick-fix rows painted inside an
    /// expanded diagnostic bubble. Each entry: (line_0based,
    /// action_index, row_rect). Recomputed every paint pass.
    action_bounds: Vec<(usize, usize, Rect)>,
    /// Per-line cache of code actions to render inside the expanded
    /// bubble. Pushed in by the view layer from
    /// `AppModel::bubble_code_actions` via a `widget_mut` setter.
    code_actions: std::collections::HashMap<usize, Vec<CodeAction>>,
    /// Currently expanded diagnostic line (None = all collapsed).
    expanded_line: Option<usize>,
    /// Lines we've already emitted a "request bubble code actions"
    /// signal for. Avoids re-firing the LSP request on every paint
    /// while a bubble stays expanded; cleared when the bubble
    /// collapses.
    actions_requested_for_line: Option<usize>,
    /// Index of the highlighted quick-fix card within the currently
    /// expanded bubble. Reset to 0 when the bubble expands.
    selected_action_index: usize,
}

impl DiagnosticBubbleState {
    pub fn markers(&self) -> &[(usize, DiagnosticSeverity)] {
        &self.markers
    }

    pub fn inline_diagnostics(&self) -> &[Diagnostic] {
        &self.inline_diagnostics
    }

    pub fn bubble_bounds(&self) -> &[(usize, Rect)] {
        &self.bubble_bounds
    }

    pub fn action_bounds(&self) -> &[(usize, usize, Rect)] {
        &self.action_bounds
    }

    pub fn code_actions_view(&self) -> &std::collections::HashMap<usize, Vec<CodeAction>> {
        &self.code_actions
    }

    pub fn expanded_line(&self) -> Option<usize> {
        self.expanded_line
    }

    pub fn is_expanded_at(&self, line: usize) -> bool {
        self.expanded_line == Some(line)
    }

    pub fn selected_action_index(&self) -> usize {
        self.selected_action_index
    }

    /// Number of quick-fix actions available for the given line, 0 if
    /// none cached yet.
    fn action_count_for(&self, line: usize) -> usize {
        self.code_actions.get(&line).map(|v| v.len()).unwrap_or(0)
    }

    /// `Some(count)` when a bubble is expanded and its quick-fix cache
    /// has at least one entry — the gate Tab/Enter use to decide
    /// between bubble-navigation and their normal completion/newline
    /// behavior.
    pub fn pending_action_count(&self) -> Option<usize> {
        self.expanded_line.and_then(|line| {
            let n = self.action_count_for(line);
            (n > 0).then_some(n)
        })
    }

    /// `(line, selected_action_index)` for the expanded bubble, if any
    /// — used to apply the currently-highlighted quick-fix.
    pub fn expanded_selection(&self) -> Option<(usize, usize)> {
        self.expanded_line
            .map(|line| (line, self.selected_action_index))
    }

    /// Push a fresh diagnostic list from the View layer: replaces the
    /// full diagnostic set and re-derives the gutter markers (one per
    /// line, highest severity wins).
    pub fn set_diagnostics(&mut self, diagnostics: Vec<Diagnostic>) {
        let mut markers: std::collections::HashMap<usize, DiagnosticSeverity> =
            std::collections::HashMap::new();
        for diag in &diagnostics {
            let entry = markers.entry(diag.line).or_insert(diag.severity);
            if diag.severity == DiagnosticSeverity::Error {
                *entry = DiagnosticSeverity::Error;
            }
        }
        self.markers = markers.into_iter().collect();
        self.inline_diagnostics = diagnostics;
    }

    /// Replace the per-line quick-fix cache wholesale.
    pub fn set_code_actions(&mut self, cache: std::collections::HashMap<usize, Vec<CodeAction>>) {
        self.code_actions = cache;
    }

    /// Record the bubble + quick-fix-row geometry painted this frame.
    pub fn set_painted_bounds(
        &mut self,
        bubble_bounds: Vec<(usize, Rect)>,
        action_bounds: Vec<(usize, usize, Rect)>,
    ) {
        self.bubble_bounds = bubble_bounds;
        self.action_bounds = action_bounds;
    }

    /// Hit-test a quick-fix row at `pos`. Returns `(line, action_index)`.
    pub fn action_at(&self, pos: Point) -> Option<(usize, usize)> {
        self.action_bounds
            .iter()
            .find(|(_, _, r)| r.contains(pos))
            .map(|(l, i, _)| (*l, *i))
    }

    /// Hit-test a bubble at `pos`. Returns the line it belongs to.
    pub fn bubble_at(&self, pos: Point) -> Option<usize> {
        self.bubble_bounds
            .iter()
            .find(|(_, r)| r.contains(pos))
            .map(|(l, _)| *l)
    }

    /// Expand the bubble for `line`, resetting the selected quick-fix
    /// index. Does not fire the code-actions request — the caller owns
    /// `ctx.submit_action` and should follow up with
    /// `mark_actions_requested`.
    pub fn expand(&mut self, line: usize) {
        self.expanded_line = Some(line);
        self.selected_action_index = 0;
    }

    pub fn mark_actions_requested(&mut self, line: usize) {
        self.actions_requested_for_line = Some(line);
    }

    /// Collapse whatever bubble is expanded, clearing the requested-
    /// for-line marker and selected index together.
    pub fn collapse(&mut self) {
        self.expanded_line = None;
        self.actions_requested_for_line = None;
        self.selected_action_index = 0;
    }

    /// Move the selected quick-fix index within the expanded bubble.
    /// Returns `true` when the selection actually moved (so the
    /// caller knows whether to repaint) — `false` when no bubble is
    /// expanded or it has no quick-fixes yet.
    pub fn select_step(&mut self, backward: bool) -> bool {
        let Some(line) = self.expanded_line else {
            return false;
        };
        let count = self.action_count_for(line);
        if count == 0 {
            return false;
        }
        let cur = self.selected_action_index.min(count - 1);
        self.selected_action_index = if backward {
            if cur == 0 { count - 1 } else { cur - 1 }
        } else if cur + 1 >= count {
            0
        } else {
            cur + 1
        };
        true
    }
}
