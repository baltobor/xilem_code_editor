// Copyright 2026 the xilem_code_editor developers
// SPDX-License-Identifier: Apache-2.0

//! Syntax highlight data: categories and per-line spans. Producing them
//! is the job of a [`HighlightProvider`](crate::provider::HighlightProvider).

mod category;
mod span;
mod split_into_lines;

pub use category::HighlightCategory;
pub use span::HighlightSpan;
pub use split_into_lines::split_into_lines;
