// Copyright 2026 the xilem_code_editor developers
// SPDX-License-Identifier: Apache-2.0

//! Layout-facing abstractions the model needs without depending on a
//! text-layout engine: visual coordinates and the [`TextMetrics`] trait.

mod coords;
mod text_metrics;

pub use coords::{ColPosition, RVLine, VLine};
pub use text_metrics::TextMetrics;
