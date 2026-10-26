// Copyright 2026 the xilem_code_editor developers
// SPDX-License-Identifier: Apache-2.0

//! Minimum identifier-prefix length before auto-trigger fires.
//!
//! Single responsibility: own the constant. Lives in its own
//! file so a future preferences hook can swap it without
//! touching the trigger logic.

/// IntelliJ-style: 3 characters before showing suggestions.
/// One-letter keystrokes never pop the world; even two-letter
/// keystrokes stay quiet so passing text like `if` / `fn` /
/// `pub` doesn't trigger a request that's about to be wasted.
/// Three letters is when the user has visibly committed to
/// typing an identifier. Ctrl+Space remains the always-
/// available manual escape at any prefix length / context.
pub const AUTO_TRIGGER_MIN_PREFIX: usize = 3;
