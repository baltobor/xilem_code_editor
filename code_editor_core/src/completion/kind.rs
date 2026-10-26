// Copyright 2026 the xilem_code_editor developers
// SPDX-License-Identifier: Apache-2.0

/// What a completion item refers to; drives the glyph shown in the popup.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum CompletionKind {
    Function,
    Method,
    Field,
    Variable,
    Constant,
    Struct,
    Enum,
    Module,
    Keyword,
    Snippet,
    #[default]
    Other,
}
