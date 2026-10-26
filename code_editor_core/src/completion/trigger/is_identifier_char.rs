// Copyright 2026 the xilem_code_editor developers
// SPDX-License-Identifier: Apache-2.0

//! `is_identifier_char` — the "is this char part of an
//! identifier?" predicate.

/// True for chars that can appear inside a Rust identifier
/// (alphanumeric or underscore). The auto-trigger path uses
/// this to decide whether the keystroke is the user typing
/// into / extending an identifier.
pub fn is_identifier_char(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}
