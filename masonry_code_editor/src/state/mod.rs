// Copyright 2026 the xilem_code_editor developers
// SPDX-License-Identifier: Apache-2.0

//! Per-feature widget state for popups and gutter interactions.

mod completion_state;
mod diagnostic_bubble_state;
mod hover_state;
mod quick_help_state;
mod vcs_gutter_state;

pub use completion_state::*;
pub use diagnostic_bubble_state::*;
pub use hover_state::HoverState;
pub use quick_help_state::*;
pub use vcs_gutter_state::*;
