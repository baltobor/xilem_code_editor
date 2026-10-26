// Copyright 2026 the xilem_code_editor developers
// SPDX-License-Identifier: Apache-2.0

//! Time-driven behavior as pure state machines. Every function takes
//! `now`, so the widget drives them from animation frames and tests
//! drive them with synthetic instants. Defaults follow zed.

mod completion_policy;
mod cursor_blink;
mod debouncer;
pub mod defaults;
mod hover_timer;
mod popup_timing;
mod request_generation;

pub use completion_policy::{CompletionStep, next_completion_step};
pub use cursor_blink::{AnimationResult, CursorAnimator};
pub use debouncer::Debouncer;
pub use hover_timer::{HoverEffect, HoverTimer};
pub use popup_timing::PopupTiming;
pub use request_generation::RequestGeneration;
