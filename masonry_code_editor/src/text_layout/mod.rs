// Copyright 2026 the xilem_code_editor developers
// SPDX-License-Identifier: Apache-2.0

//! Text layout: one parley layout per buffer line, viewport
//! virtualization, visible-line snapshots, phantom (inlay) text, and
//! offset <-> position mapping.

mod coordinate_mapper;
mod layout_cache;
mod line_layout;
mod lines;
mod phantom_text;
mod screen_lines;
mod viewport;
mod visual_rows;

pub use coordinate_mapper::*;
pub use layout_cache::LayoutCache;
pub use line_layout::{LineLayout, build_line_layout};
pub use lines::*;
pub use phantom_text::{PhantomText, PhantomTextKind, PhantomTextLine};
pub use screen_lines::*;
pub use viewport::ViewportState;
pub use visual_rows::CaretGeometry;
