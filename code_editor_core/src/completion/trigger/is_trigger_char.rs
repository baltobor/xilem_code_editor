// Copyright 2026 the xilem_code_editor developers
// SPDX-License-Identifier: Apache-2.0

//! `is_trigger_char` — does the typed char match a server-
//! declared trigger character (after the noisy-bracket
//! filter)?

use super::is_noisy_trigger_char::is_noisy_trigger_char;

/// True when `ch` is one of the server's declared trigger
/// characters and isn't on the noisy-bracket deny-list.
pub fn is_trigger_char(ch: char, declared: &[String]) -> bool {
    if is_noisy_trigger_char(ch) {
        return false;
    }
    let ch_str = ch.to_string();
    declared.iter().any(|tc| tc == &ch_str)
}
