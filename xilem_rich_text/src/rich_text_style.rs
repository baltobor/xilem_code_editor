// Copyright 2026 the xilem_code_editor developers
// SPDX-License-Identifier: Apache-2.0

//! Typography for rich text: font, size, line height and colors.

use xilem::Color;
use xilem::masonry::parley::FontFamilyName;

#[derive(Debug, Clone, PartialEq)]
pub struct RichTextStyle {
    /// Body font stack; parley uses the first installed one.
    pub font_stack: &'static [FontFamilyName<'static>],
    pub font_size: f32,
    /// Line height as a multiple of the font size.
    pub line_height: f32,
    pub bold_base: bool,
    pub text_color: Color,
    pub link_color: Color,
    pub code_color: Color,
}
