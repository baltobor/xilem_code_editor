// Copyright 2026 the xilem_code_editor developers
// SPDX-License-Identifier: Apache-2.0

use crate::completion::{CompletionItem, CompletionResponse};
use crate::edit::TextEdit;

use super::{PositionRequest, ProviderFuture};

/// Why completions are requested.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompletionTrigger {
    /// Typing an identifier character or a trigger character.
    Typing(char),
    /// Explicit request (e.g. Ctrl+Space).
    Explicit,
}

/// Supplies completion items (language server, word list, wiki links, …).
pub trait CompletionProvider: Send + Sync {
    fn completions(
        &self,
        request: PositionRequest,
        trigger: CompletionTrigger,
    ) -> ProviderFuture<CompletionResponse>;

    /// Characters that open the completion popup besides identifier
    /// characters, e.g. `.` and `::`'s `:`, or `[` for wiki links.
    fn trigger_characters(&self) -> &[char] {
        &[]
    }

    /// Additional edits applied after accepting `item` (e.g. an import).
    fn additional_edits(&self, _item: &CompletionItem) -> ProviderFuture<Vec<TextEdit>> {
        Box::pin(async { Vec::new() })
    }
}
