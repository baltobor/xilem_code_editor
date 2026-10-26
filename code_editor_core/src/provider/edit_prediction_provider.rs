// Copyright 2026 the xilem_code_editor developers
// SPDX-License-Identifier: Apache-2.0

use std::time::Duration;

use crate::model::EditPrediction;
use crate::timing::defaults;

use super::{PositionRequest, ProviderFuture};

/// Supplies ghost-text predictions of the next edit.
pub trait EditPredictionProvider: Send + Sync {
    fn predict(&self, request: PositionRequest) -> ProviderFuture<Option<EditPrediction>>;

    /// Quiet time after the last keystroke before `predict` is called.
    fn debounce(&self) -> Duration {
        defaults::EDIT_PREDICTION_DEBOUNCE
    }
}
