// Copyright 2026 the xilem_code_editor developers
// SPDX-License-Identifier: Apache-2.0

use crate::model::InlayHint;

use super::{DocumentSnapshot, ProviderFuture};

/// Supplies inlay hints for the whole document.
pub trait InlayHintProvider: Send + Sync {
    fn inlay_hints(&self, document: DocumentSnapshot) -> ProviderFuture<Vec<InlayHint>>;
}
