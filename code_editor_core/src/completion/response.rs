// Copyright 2026 the xilem_code_editor developers
// SPDX-License-Identifier: Apache-2.0

use super::CompletionItem;

/// What a completion provider answers.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct CompletionResponse {
    pub items: Vec<CompletionItem>,
    /// The list is partial: typing more must re-request instead of
    /// filtering client-side (LSP `isIncomplete`).
    pub is_incomplete: bool,
}
