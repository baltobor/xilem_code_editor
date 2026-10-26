// Copyright 2026 the xilem_code_editor developers
// SPDX-License-Identifier: Apache-2.0

use std::future::Future;
use std::pin::Pin;

/// The future a provider returns. Dropping it cancels the request — the
/// editor drops superseded requests the way zed drops its tasks.
pub type ProviderFuture<T> = Pin<Box<dyn Future<Output = T> + Send + 'static>>;
