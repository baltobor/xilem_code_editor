// Copyright 2026 the xilem_code_editor developers
// SPDX-License-Identifier: Apache-2.0

use crate::model::SignatureHelp;

use super::{PositionRequest, ProviderFuture};

/// Supplies the signature of the call around the caret.
pub trait SignatureHelpProvider: Send + Sync {
    fn signature_help(&self, request: PositionRequest) -> ProviderFuture<Option<SignatureHelp>>;

    /// Characters that (re-)open signature help, e.g. `(` and `,`.
    fn trigger_characters(&self) -> &[char] {
        &['(', ',']
    }
}
