// Copyright 2026 the xilem_code_editor developers
// SPDX-License-Identifier: Apache-2.0

//! Scroll a portal so a chosen block sits at the top of the viewport
//! (a table-of-contents jump). A `ScrollAnchor` wraps a block and, when
//! asked to reveal, reports where it is; the `ScrollHost` directly inside
//! the portal turns that into scroll requests.

mod anchor_view;
mod anchor_widget;
mod host_view;
mod host_widget;
mod single_child;

pub use anchor_view::{ScrollAnchorView, scroll_anchor};
pub use anchor_widget::{RevealAnchor, ScrollAnchor};
pub use host_view::{ScrollHostView, scroll_host};
pub use host_widget::ScrollHost;

#[cfg(test)]
mod tests;
