//! Keyboard, mouse, and IME event handling for the code editor.
//!
//! Extracted from the widget so widget.rs stays focused on
//! orchestration. The widget calls `handle_key_event`,
//! `handle_pointer_event`, and `handle_paste` from its
//! `on_text_event` and `on_pointer_event` methods, and reads back an
//! `InputResult` describing what happened (text edited, cursor
//! moved, popup dismissed, LSP request to fire, etc.).
//!
//! ## File layout (after the SOLID refactor)
//!
//! - this `mod.rs`     — `InputResult` flag struct, the two pure
//!                        priority key handlers (`try_handle_snippet_key`,
//!                        `try_handle_bubble_key`), `handle_paste`,
//!                        and the `#[cfg(test)] mod tests` priority-
//!                        order regression net.
//! - `key.rs`          — `handle_key_event` (the unified keyboard
//!                        router) plus tiny bracket-pair helpers used
//!                        by its auto-close logic.
//! - `pointer.rs`      — `handle_pointer_event` (mouse / scroll /
//!                        drag).
//! - `word_helpers.rs` — word-boundary + indentation helpers shared
//!                        by `key.rs` (Tab indent) and `pointer.rs`
//!                        (double-click word-select).

mod key;
mod key_handlers;
mod paste;
mod pointer;

pub use key::handle_key_event;
pub use key_handlers::{try_handle_bubble_key, try_handle_snippet_key};
pub use paste::handle_paste;
pub use pointer::handle_pointer_event;

// MARK: InputResult — flags returned to the widget after handling

/// Result of processing an input event.
pub struct InputResult {
    /// Whether the document text was edited (needs layout + content notification).
    pub edited: bool,
    /// Whether the cursor moved (needs cursor position notification).
    pub cursor_moved: bool,
    /// Whether the event was handled (should call ctx.set_handled()).
    pub handled: bool,
    /// Whether an undo was requested.
    pub undo: bool,
    /// Whether a redo was requested.
    pub redo: bool,
    /// Whether "Go to Definition" was requested (F12 / Cmd+B).
    pub goto_definition: bool,
    /// Whether "Hover Preview" was requested (Alt+F12 / Cmd+Shift+B).
    pub hover_request: bool,
    /// Whether font size increase was requested (Cmd/Ctrl + Plus).
    pub increase_font_size: bool,
    /// Whether font size decrease was requested (Cmd/Ctrl + Minus).
    pub decrease_font_size: bool,
    /// Whether completion was requested (Ctrl+Space).
    pub request_completion: bool,
    /// Whether code actions were requested (Ctrl+.).
    pub request_code_actions: bool,
    /// Whether signature help was requested (Ctrl+Shift+Space).
    pub request_signature_help: bool,
    /// Whether help documentation was requested (F1).
    pub request_help: bool,
    /// Whether popups should be dismissed (Escape).
    pub dismiss_popups: bool,
    /// Esc-on-visible-completion-popup variant of
    /// `dismiss_popups`. Different routing path so the model
    /// can set the suppression flag (so the popup doesn't
    /// auto-reopen on the next letter of the current word).
    pub dismiss_completion_by_esc: bool,
    /// Bracket ghost character to set (e.g., ')' after typing '(').
    pub set_bracket_ghost: Option<char>,
    /// Whether to clear bracket ghost (on non-bracket input).
    pub clear_bracket_ghost: bool,
    /// Whether to accept the current bracket ghost inline (Tab pressed).
    pub accept_bracket_ghost: bool,
    /// Whether to accept bracket ghost with newline in between (Enter pressed).
    pub accept_bracket_ghost_newline: bool,
    /// Whether to accept completion ghost text (Tab pressed with completion visible).
    pub accept_completion_ghost: bool,
    /// Character that was typed (for trigger character detection).
    pub typed_char: Option<char>,
    /// Whether to select the previous completion item (Up with completion visible).
    pub completion_select_prev: bool,
    /// Whether to select the next completion item (Down with completion visible).
    pub completion_select_next: bool,
    /// Whether to accept the current completion item (Enter with completion visible).
    pub accept_completion: bool,
    /// Whether to move the diagnostic-bubble action selection up.
    pub bubble_select_prev: bool,
    /// Whether to move the diagnostic-bubble action selection down.
    pub bubble_select_next: bool,
    /// Whether to apply the currently-selected bubble quick fix
    /// (Tab / Return while a bubble is expanded).
    pub apply_selected_bubble_action: bool,
    /// Whether to collapse the expanded diagnostic bubble (Esc).
    pub collapse_bubble: bool,
    /// Whether to advance to the next snippet placeholder (Tab while a
    /// snippet session is active).
    pub snippet_next: bool,
    /// Whether to step back to the previous snippet placeholder
    /// (Shift+Tab while a snippet session is active).
    pub snippet_prev: bool,
    /// Whether to commit the snippet session (Esc, or Return at the
    /// final stop).
    pub snippet_commit: bool,
}

impl Default for InputResult {
    fn default() -> Self {
        Self {
            edited: false,
            cursor_moved: false,
            handled: false,
            undo: false,
            redo: false,
            goto_definition: false,
            hover_request: false,
            increase_font_size: false,
            decrease_font_size: false,
            request_completion: false,
            request_code_actions: false,
            request_signature_help: false,
            request_help: false,
            dismiss_popups: false,
            dismiss_completion_by_esc: false,
            set_bracket_ghost: None,
            clear_bracket_ghost: false,
            accept_bracket_ghost: false,
            accept_bracket_ghost_newline: false,
            accept_completion_ghost: false,
            typed_char: None,
            completion_select_prev: false,
            completion_select_next: false,
            accept_completion: false,
            bubble_select_prev: false,
            bubble_select_next: false,
            apply_selected_bubble_action: false,
            collapse_bubble: false,
            snippet_next: false,
            snippet_prev: false,
            snippet_commit: false,
        }
    }
}

impl InputResult {
    fn nop() -> Self {
        Self::default()
    }

    fn cursor_only() -> Self {
        Self {
            cursor_moved: true,
            handled: true,
            ..Self::default()
        }
    }

    fn edit() -> Self {
        Self {
            edited: true,
            cursor_moved: true,
            handled: true,
            ..Self::default()
        }
    }

    fn handled() -> Self {
        Self {
            handled: true,
            ..Self::default()
        }
    }
}

/// Get the matching closing bracket for an opening bracket.
///
/// Curly braces only — by user policy. Auto-closing parentheses /
/// brackets / quotes is more disruptive than helpful in practice
/// (every completion accept that ends with `()` re-triggers a popup,
/// and `'lifetimes` get autoclosed as char literals). Restrict
/// auto-close to `{}` where the multi-line nature of the matching
/// pair actually saves the user keystrokes.

/// Find the start of the word containing the given byte offset.
// MARK: tests
//
// Priority-order regression net for the keyboard router. Each test
// pins one branch of the chain so that a future refactor that
// accidentally re-orders snippet vs bubble vs completion handling
// trips a test instead of silently breaking the editor.
//
// Tests focus on the two pure helpers (`try_handle_snippet_key`,
// `try_handle_bubble_key`) and on a hand-rolled mini-router that
// composes them in priority order. The full `handle_key_event`
// requires an `EventCtx` (set_clipboard) so it can't run in a
// pure test — but the priority chain *itself* lives in those two
// helpers, which is what we cover here.
#[cfg(test)]
mod tests {
    use super::*;
    use masonry::core::keyboard::{Key, NamedKey};

    /// A snippet session is open. Tab should advance the snippet,
    /// not get swallowed by anything else.
    #[test]
    fn tab_with_active_snippet_advances_to_next_stop() {
        let r = try_handle_snippet_key(
            &Key::Named(NamedKey::Tab),
            /* shift */ false,
            /* action_mod */ false,
            /* snippet_active */ true,
        )
        .expect("snippet handler claims Tab");
        assert!(r.snippet_next);
        assert!(r.handled);
        assert!(!r.snippet_prev);
        assert!(!r.snippet_commit);
    }

    #[test]
    fn shift_tab_with_active_snippet_walks_back_to_previous_stop() {
        let r = try_handle_snippet_key(&Key::Named(NamedKey::Tab), true, false, true)
            .expect("snippet handler claims Shift+Tab");
        assert!(r.snippet_prev);
        assert!(!r.snippet_next);
    }

    #[test]
    fn enter_and_esc_with_active_snippet_commit_the_session() {
        let enter = try_handle_snippet_key(&Key::Named(NamedKey::Enter), false, false, true)
            .expect("snippet handler claims Enter");
        assert!(enter.snippet_commit);

        let esc = try_handle_snippet_key(&Key::Named(NamedKey::Escape), false, false, true)
            .expect("snippet handler claims Esc");
        assert!(esc.snippet_commit);
    }

    /// Snippet helper must NOT claim arrow keys, regular letters, etc.
    /// — those need to fall through to the next priority level.
    #[test]
    fn snippet_handler_does_not_claim_arrow_keys() {
        assert!(
            try_handle_snippet_key(&Key::Named(NamedKey::ArrowDown), false, false, true,).is_none()
        );
        assert!(
            try_handle_snippet_key(&Key::Named(NamedKey::ArrowUp), false, false, true,).is_none()
        );
    }

    /// With no snippet session active, the helper must yield `None`
    /// for every key — even the ones it would otherwise claim.
    #[test]
    fn snippet_handler_yields_when_no_session_is_active() {
        assert!(
            try_handle_snippet_key(
                &Key::Named(NamedKey::Tab),
                false,
                false,
                /* snippet_active */ false,
            )
            .is_none()
        );
    }

    /// An expanded bubble with quick fixes captures arrow keys.
    #[test]
    fn arrow_down_with_bubble_expanded_moves_marker() {
        let r = try_handle_bubble_key(
            &Key::Named(NamedKey::ArrowDown),
            false,
            false,
            /* bubble_action_count */ Some(3),
        )
        .expect("bubble handler claims ArrowDown");
        assert!(r.bubble_select_next);
        assert!(!r.bubble_select_prev);
    }

    #[test]
    fn tab_and_enter_with_bubble_apply_the_selected_action() {
        let tab = try_handle_bubble_key(&Key::Named(NamedKey::Tab), false, false, Some(2))
            .expect("bubble claims Tab");
        assert!(tab.apply_selected_bubble_action);

        let enter = try_handle_bubble_key(&Key::Named(NamedKey::Enter), false, false, Some(2))
            .expect("bubble claims Enter");
        assert!(enter.apply_selected_bubble_action);
    }

    #[test]
    fn esc_with_bubble_expanded_collapses_keeping_caret() {
        let r = try_handle_bubble_key(&Key::Named(NamedKey::Escape), false, false, Some(1))
            .expect("bubble claims Esc");
        assert!(r.collapse_bubble);
    }

    /// A bubble that is expanded but has zero quick-fix entries
    /// should *not* steal keys — the user might still want to
    /// dismiss via the generic Esc handler.
    #[test]
    fn bubble_handler_yields_when_no_quick_fix_actions() {
        assert!(
            try_handle_bubble_key(&Key::Named(NamedKey::Escape), false, false, Some(0),).is_none()
        );
    }

    #[test]
    fn bubble_handler_yields_when_no_bubble_expanded() {
        assert!(
            try_handle_bubble_key(&Key::Named(NamedKey::ArrowDown), false, false, None,).is_none()
        );
    }

    /// **The critical priority test.** When a snippet session is
    /// active *and* a bubble is expanded, Tab must drive the snippet
    /// — not apply a quick fix. This pins the `snippet → bubble`
    /// order in `handle_key_event`.
    #[test]
    fn tab_with_snippet_and_bubble_both_active_drives_the_snippet() {
        // Compose the priority chain by hand, mirroring
        // handle_key_event's order.
        let snippet = try_handle_snippet_key(
            &Key::Named(NamedKey::Tab),
            false,
            false,
            /* snippet_active */ true,
        );
        let bubble = try_handle_bubble_key(
            &Key::Named(NamedKey::Tab),
            false,
            false,
            /* bubble_action_count */ Some(3),
        );

        // Both want Tab — but snippet runs first and wins.
        assert!(snippet.is_some(), "snippet should claim Tab");
        let snippet = snippet.unwrap();
        assert!(snippet.snippet_next);
        assert!(!snippet.apply_selected_bubble_action);

        // Demonstrate that the bubble *would* claim it if it ran
        // first, so the test catches a future re-ordering of the
        // chain.
        assert!(bubble.is_some(), "bubble would also claim Tab");
        let bubble = bubble.unwrap();
        assert!(bubble.apply_selected_bubble_action);
    }

    /// Arrow-down with a snippet active *and* a bubble open should
    /// fall through to the bubble — the snippet handler doesn't
    /// claim arrow keys, so priority resolves cleanly.
    #[test]
    fn arrow_down_with_snippet_active_falls_through_to_bubble() {
        let snippet = try_handle_snippet_key(&Key::Named(NamedKey::ArrowDown), false, false, true);
        assert!(snippet.is_none(), "snippet should NOT claim ArrowDown");

        // …so the next handler in line takes it.
        let bubble = try_handle_bubble_key(&Key::Named(NamedKey::ArrowDown), false, false, Some(2));
        assert!(bubble.is_some());
        assert!(bubble.unwrap().bubble_select_next);
    }
}
