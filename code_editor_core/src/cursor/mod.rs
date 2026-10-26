// Copyright 2026 the xilem_code_editor developers
// SPDX-License-Identifier: Apache-2.0

//! Cursor: wraps a [`Selection`](crate::selection::Selection) with
//! high-level movement and editing operations.
//!
//! - `cursor.rs`               — the `Cursor` struct and accessors.
//! - `movement.rs`             — character/line/word movement (+ select variants).
//! - `editing.rs`              — insert / delete / word delete.
//! - `word_boundaries.rs`      — word boundary helpers.
//! - `brackets.rs`             — bracket matching.
//! - `bracket_continuation.rs` — what follows `}` when `{<Return>` is expanded.
//! - `word_helpers.rs`         — word/indent helpers used by input handling.

mod brackets;
mod cursor;
mod editing;
mod movement;
mod word_boundaries;

pub mod bracket_continuation;
pub mod word_helpers;

pub use brackets::{find_matching_bracket, find_nearby_bracket, is_bracket};
pub use cursor::Cursor;
pub use word_boundaries::{next_word_boundary, prev_word_boundary};
