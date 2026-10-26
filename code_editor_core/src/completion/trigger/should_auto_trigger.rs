// Copyright 2026 the xilem_code_editor developers
// SPDX-License-Identifier: Apache-2.0

//! `should_auto_trigger` — the public auto-trigger decision.
//!
//! Single responsibility: combine the helper signals to decide
//! whether typing `ch` at `caret_byte` should fire a completion
//! request.

use super::identifier_run_length::identifier_run_length_ending_at;
use super::is_identifier_char::is_identifier_char;
use super::min_prefix::AUTO_TRIGGER_MIN_PREFIX;
use crate::document::Document;

/// Decide whether to auto-trigger completion after the user
/// typed `ch`.
///
/// `caret_byte` must be the caret offset *after* `ch` was
/// inserted. `popup_visible` is true when the completion popup
/// is currently on screen.
///
/// Rules (Xcode-style):
///   - typed char must be identifier-like;
///   - if the popup is already up, refresh on every keystroke
///     so the filter follows the prefix in real time;
///   - if the popup is hidden, only fire when the typed char
///     completes an identifier prefix of `AUTO_TRIGGER_MIN_PREFIX`
///     identifier chars (so `f` is silent, `fo` opens it).
///     Ctrl+Space remains the always-available manual escape.
pub fn should_auto_trigger(
    ch: char,
    caret_byte: usize,
    popup_visible: bool,
    doc: &dyn Document,
) -> bool {
    if !is_identifier_char(ch) {
        return false;
    }
    if popup_visible {
        return true;
    }
    identifier_run_length_ending_at(caret_byte, doc) >= AUTO_TRIGGER_MIN_PREFIX
}
