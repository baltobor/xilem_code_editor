//! State for the completion feature on `CodeEditorWidget`: the popup
//! list, block-mode single-preview, bracket/completion ghost text,
//! and the "popup navigated" / "visible" flags that gate keyboard
//! routing.
//!
//! Extracted from nine loose fields (`bracket_ghost`/
//! `completion_ghost_text`/`completion_visible`/
//! `popup_user_navigated`/`completion_popup_items`/
//! `completion_popup_selected`/`completion_item_bounds`/
//! `completion_block_action`/`completion_trigger_chars`) as part of
//! the CodeEditorWidget SOLID rework. Two invariants that used to be
//! duplicated at each call site now live once, on the type that owns
//! them:
//! - bracket ghost takes priority over completion ghost when both
//!   happen to be set (`ghost_to_paint`)
//! - clearing "all ghost text" always clears both fields together
//!   (`clear_ghost`)

use masonry::kurbo::{Point, Rect};

use code_editor_core::completion::CompletionItem;

#[derive(Debug, Clone, Default)]
pub struct CompletionState {
    /// Single-char gray ghost shown after the cursor for an
    /// auto-closing bracket (Xcode-style bracket mode).
    bracket_ghost: Option<char>,
    /// Multi-char faded preview of the selected completion.
    completion_ghost_text: Option<String>,
    /// Whether the completion popup (list or block-mode) is visible.
    visible: bool,
    /// Mirror of `AppModel.popup_user_navigated`: true once the user
    /// has moved the popup selection with Up/Down, arming Return to
    /// accept instead of inserting a newline.
    popup_user_navigated: bool,
    /// Completion list items for the caret-anchored popup.
    items: Vec<CompletionItem>,
    /// Index of the highlighted item within `items`.
    selected: usize,
    /// Hit-test rectangles for the last-painted popup rows (list or
    /// block-mode). Each entry: (item_index, row_rect).
    item_bounds: Vec<(usize, Rect)>,
    /// Block-mode payload: a single full-preview code-action
    /// `(title, preview, source_idx, source_total)`. Renders in place
    /// of the list popup when set; the AppModel guarantees the two
    /// never both apply.
    block_action: Option<(String, String, usize, usize)>,
    /// Completion trigger characters advertised by the active LSP
    /// server's capabilities.
    trigger_chars: Vec<String>,
}

impl CompletionState {
    pub fn bracket_ghost(&self) -> Option<char> {
        self.bracket_ghost
    }

    pub fn completion_ghost_active(&self) -> bool {
        self.completion_ghost_text.is_some()
    }

    /// Ghost text to paint after the cursor this frame: bracket ghost
    /// takes priority over the completion preview when both are set.
    pub fn ghost_to_paint(&self) -> Option<String> {
        if let Some(bracket) = self.bracket_ghost {
            Some(bracket.to_string())
        } else {
            self.completion_ghost_text.clone()
        }
    }

    pub fn is_visible(&self) -> bool {
        self.visible
    }

    pub fn popup_user_navigated(&self) -> bool {
        self.popup_user_navigated
    }

    pub fn items(&self) -> &[CompletionItem] {
        &self.items
    }

    pub fn selected(&self) -> usize {
        self.selected
    }

    pub fn block_action(&self) -> Option<&(String, String, usize, usize)> {
        self.block_action.as_ref()
    }

    pub fn trigger_chars(&self) -> &[String] {
        &self.trigger_chars
    }

    /// Whether the block-mode single-preview popup should paint this
    /// frame: visible, no list items, and a block payload is set. The
    /// AppModel-side strict-mode rule guarantees the list popup
    /// (`should_paint_list_popup`) is suppressed whenever this holds.
    pub fn should_paint_block_popup(&self) -> bool {
        self.visible && self.items.is_empty() && self.block_action.is_some()
    }

    /// Whether the caret-anchored list popup should paint this frame.
    pub fn should_paint_list_popup(&self) -> bool {
        self.visible && !self.items.is_empty()
    }

    pub fn take_bracket_ghost(&mut self) -> Option<char> {
        self.bracket_ghost.take()
    }

    /// Raw bracket-ghost set. Returns whether the value actually
    /// changed, so callers that only repaint on change can gate on
    /// it; callers that always repaint (e.g. a fresh ghost trigger)
    /// can ignore the return value.
    pub fn set_bracket_ghost(&mut self, ghost: Option<char>) -> bool {
        let changed = self.bracket_ghost != ghost;
        self.bracket_ghost = ghost;
        changed
    }

    /// Raw completion-ghost-text set. Same changed-return convention
    /// as `set_bracket_ghost`.
    pub fn set_completion_ghost_text(&mut self, text: Option<String>) -> bool {
        let changed = self.completion_ghost_text != text;
        self.completion_ghost_text = text;
        changed
    }

    /// Clear both ghost fields together. Returns whether either was
    /// actually set.
    pub fn clear_ghost(&mut self) -> bool {
        let changed = self.bracket_ghost.is_some() || self.completion_ghost_text.is_some();
        self.bracket_ghost = None;
        self.completion_ghost_text = None;
        changed
    }

    pub fn set_visible(&mut self, visible: bool) {
        self.visible = visible;
    }

    /// Returns whether the flag actually changed.
    pub fn set_popup_user_navigated(&mut self, navigated: bool) -> bool {
        let changed = self.popup_user_navigated != navigated;
        self.popup_user_navigated = navigated;
        changed
    }

    /// Push the full popup payload (items + selection + visibility)
    /// in one call, matching the single-channel setter the View layer
    /// uses.
    pub fn set_popup(&mut self, items: Vec<CompletionItem>, selected: usize, visible: bool) {
        self.items = items;
        self.selected = selected;
        self.visible = visible;
    }

    /// Returns whether the payload actually changed.
    pub fn set_block_action(&mut self, action: Option<(String, String, usize, usize)>) -> bool {
        let changed = self.block_action != action;
        self.block_action = action;
        changed
    }

    pub fn set_trigger_chars(&mut self, chars: Vec<String>) {
        self.trigger_chars = chars;
    }

    pub fn clear_item_bounds(&mut self) {
        self.item_bounds.clear();
    }

    pub fn set_item_bounds(&mut self, bounds: Vec<(usize, Rect)>) {
        self.item_bounds = bounds;
    }

    /// Hit-test a popup row at `pos`. Returns the item index.
    pub fn item_at(&self, pos: Point) -> Option<usize> {
        self.item_bounds
            .iter()
            .find(|(_, r)| r.contains(pos))
            .map(|(i, _)| *i)
    }
}
