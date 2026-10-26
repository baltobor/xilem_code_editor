// Copyright 2026 the xilem_code_editor developers
// SPDX-License-Identifier: Apache-2.0

//! Completion model: items, provider responses, client-side filtering,
//! and when to auto-trigger.

mod filter;
mod item;
mod kind;
mod response;
pub mod trigger;

pub use filter::filter_completions;
pub use item::CompletionItem;
pub use kind::CompletionKind;
pub use response::CompletionResponse;
