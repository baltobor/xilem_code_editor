// Copyright 2026 the xilem_code_editor developers
// SPDX-License-Identifier: Apache-2.0

use std::sync::Arc;

use crate::edit::TextPosition;

/// Snapshot of the document a provider request refers to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DocumentSnapshot {
    pub text: Arc<str>,
    /// Document revision; results for an older revision are discarded.
    pub revision: u64,
    /// Language id as given to the editor (usually the file extension).
    pub language: Arc<str>,
}

/// A request for a position in the document.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PositionRequest {
    pub document: DocumentSnapshot,
    /// Byte offset of the position.
    pub offset: usize,
    pub position: TextPosition,
}
