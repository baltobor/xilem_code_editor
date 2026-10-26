//! Priority key-handler chain — pure, testable.
//!
//! Each `try_handle_*` returns `Some(result)` when its mode owns the
//! keystroke or `None` when the keystroke should fall through to the
//! next handler in the chain. Order matches the call order in
//! `handle_key_event` so the same priority chain lives in one place.

use masonry::core::keyboard::{Key, NamedKey};

use super::InputResult;

// MARK: Priority key handlers (pure, testable)
//
// Each `try_handle_*` is a pure function that returns `Some(result)`
// when its mode owns the keystroke, or `None` when the keystroke
// should fall through to the next handler in the priority chain.
//
// **Priority order — must match the call order in `handle_key_event`:**
//
//   1. `try_handle_snippet_key`   — when `snippet_active`, owns Tab,
//      Shift+Tab, Return, Esc. Anything else falls through (typing
//      inside a placeholder is normal editing).
//   2. `try_handle_bubble_key`    — when an expanded diagnostic
//      bubble has ≥1 quick fix, owns ↑/↓, Tab, Return, Esc.
//   3. (still inside `handle_key_event`) — completion popup nav,
//      generic Esc, and all normal text-editing keys.
//
// The handlers are pure and take only the small key-context they need
// so they can be unit-tested without an `EventCtx` or a `Cursor`.
// The priority-order tests at the bottom of this file are the
// regression net for this chain.

/// Snippet-mode key router. Returns `Some(InputResult)` when the key
/// is one of `Tab`, `Shift+Tab`, `Return`, or `Esc` and a snippet
/// session is active. Returns `None` for any other key (the caller
/// then falls through to the next handler in the chain).
pub fn try_handle_snippet_key(
    key: &Key,
    shift: bool,
    action_mod: bool,
    snippet_active: bool,
) -> Option<InputResult> {
    if !snippet_active {
        return None;
    }
    match key {
        Key::Named(NamedKey::Tab) if !action_mod && !shift => Some(InputResult {
            snippet_next: true,
            handled: true,
            ..InputResult::default()
        }),
        Key::Named(NamedKey::Tab) if !action_mod && shift => Some(InputResult {
            snippet_prev: true,
            handled: true,
            ..InputResult::default()
        }),
        Key::Named(NamedKey::Enter) if !action_mod && !shift => Some(InputResult {
            snippet_commit: true,
            handled: true,
            ..InputResult::default()
        }),
        Key::Named(NamedKey::Escape) => Some(InputResult {
            snippet_commit: true,
            handled: true,
            ..InputResult::default()
        }),
        _ => None,
    }
}

/// Diagnostic-bubble-mode key router. Returns `Some(InputResult)`
/// when an expanded bubble has at least one quick fix and the key is
/// one of `↑`, `↓`, `Tab`, `Return`, or `Esc`. Returns `None`
/// otherwise.
///
/// **Note on priority overlap with snippets:** when both
/// `snippet_active` and `bubble_action_count` are true, the snippet
/// handler runs *first* and wins for `Tab`/`Return`/`Esc`. The bubble
/// only sees those keys when no snippet session is open.
pub fn try_handle_bubble_key(
    key: &Key,
    shift: bool,
    action_mod: bool,
    bubble_action_count: Option<usize>,
) -> Option<InputResult> {
    let action_count = bubble_action_count?;
    if action_count == 0 {
        return None;
    }
    match key {
        Key::Named(NamedKey::ArrowUp) if !action_mod && !shift => Some(InputResult {
            bubble_select_prev: true,
            handled: true,
            ..InputResult::default()
        }),
        Key::Named(NamedKey::ArrowDown) if !action_mod && !shift => Some(InputResult {
            bubble_select_next: true,
            handled: true,
            ..InputResult::default()
        }),
        Key::Named(NamedKey::Tab) | Key::Named(NamedKey::Enter) => Some(InputResult {
            apply_selected_bubble_action: true,
            handled: true,
            ..InputResult::default()
        }),
        Key::Named(NamedKey::Escape) => Some(InputResult {
            collapse_bubble: true,
            handled: true,
            ..InputResult::default()
        }),
        _ => None,
    }
}
