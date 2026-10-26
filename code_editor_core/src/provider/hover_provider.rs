// Copyright 2026 the xilem_code_editor developers
// SPDX-License-Identifier: Apache-2.0

use crate::model::Hover;

use super::{PositionRequest, ProviderFuture};

/// Supplies the hover popover's content.
pub trait HoverProvider: Send + Sync {
    fn hover(&self, request: PositionRequest) -> ProviderFuture<Option<Hover>>;
}
