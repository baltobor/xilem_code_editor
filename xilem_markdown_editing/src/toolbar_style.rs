// Copyright 2026 the xilem_code_editor developers
// SPDX-License-Identifier: Apache-2.0

use xilem::Color;

/// Colors of the Markdown toolbar.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ToolbarStyle {
    pub icon_color: Color,
    /// Background of a toggled-on button (active view mode, word wrap).
    pub active_background: Color,
    /// Text size of the "H1"/"pre" labels; icons use `icon_size`.
    pub text_size: f32,
    pub icon_size: f32,
}
