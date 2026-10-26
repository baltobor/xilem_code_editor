// Copyright 2026 the xilem_code_editor developers
// SPDX-License-Identifier: Apache-2.0

use std::time::Duration;

use super::defaults;

/// Tunable popup timing; `Default` reproduces zed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PopupTiming {
    pub hover_delay: Duration,
    pub hover_hide_delay: Duration,
    /// Keep the hover open while the pointer heads towards it (zed's
    /// `hover_popover_sticky`). Without it, leaving hides immediately.
    pub hover_sticky: bool,
    pub signature_help_delay: Duration,
    pub code_actions_debounce: Duration,
}

impl Default for PopupTiming {
    fn default() -> Self {
        Self {
            hover_delay: defaults::HOVER_DELAY,
            hover_hide_delay: defaults::HOVER_HIDE_DELAY,
            hover_sticky: true,
            signature_help_delay: defaults::SIGNATURE_HELP_DELAY,
            code_actions_debounce: defaults::CODE_ACTIONS_DEBOUNCE,
        }
    }
}
