// Copyright 2026 the xilem_code_editor developers
// SPDX-License-Identifier: Apache-2.0

use crate::edit::TextEdit;
use crate::model::CodeAction;

use super::{PositionRequest, ProviderFuture};

/// Supplies quick fixes / refactorings and the edits they perform.
pub trait CodeActionProvider: Send + Sync {
    fn code_actions(&self, request: PositionRequest) -> ProviderFuture<Vec<CodeAction>>;

    /// The edits for `action` (resolved lazily, as language servers do).
    fn resolve(&self, action: &CodeAction) -> ProviderFuture<Vec<TextEdit>>;
}
