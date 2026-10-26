// Copyright 2026 the xilem_code_editor developers
// SPDX-License-Identifier: Apache-2.0

//! Timing defaults, taken from zed (`assets/settings/default.json` and
//! `crates/editor/src/editor.rs`), whose popup behavior is the reference.

use std::time::Duration;

/// Hover popover: delay from resting the pointer to showing it.
/// The provider is asked halfway through, so it has a head start.
pub const HOVER_DELAY: Duration = Duration::from_millis(300);
/// Hover popover: grace period after the pointer leaves before hiding.
/// Restarted while the pointer moves towards the popover.
pub const HOVER_HIDE_DELAY: Duration = Duration::from_millis(300);
/// Signature help, when triggered automatically.
pub const SIGNATURE_HELP_DELAY: Duration = Duration::from_millis(300);
/// Code actions for the caret position.
pub const CODE_ACTIONS_DEBOUNCE: Duration = Duration::from_millis(250);
/// Generic language-server request debounce.
pub const LSP_REQUEST_DEBOUNCE: Duration = Duration::from_millis(50);
/// Inlay hints after an edit.
pub const INLAY_HINTS_EDIT_DEBOUNCE: Duration = Duration::from_millis(700);
/// Inlay hints after scrolling.
pub const INLAY_HINTS_SCROLL_DEBOUNCE: Duration = Duration::from_millis(50);
/// Inline diagnostics refresh.
pub const INLINE_DIAGNOSTICS_DEBOUNCE: Duration = Duration::from_millis(150);
/// Highlighting other occurrences of the selection.
pub const SELECTION_HIGHLIGHT_DEBOUNCE: Duration = Duration::from_millis(100);
/// Ghost-text edit predictions (zed's copilot default).
pub const EDIT_PREDICTION_DEBOUNCE: Duration = Duration::from_millis(75);
/// Caret blink half-period.
pub const CURSOR_BLINK_INTERVAL: Duration = Duration::from_millis(500);
