// Copyright 2026 the xilem_code_editor developers
// SPDX-License-Identifier: Apache-2.0

use code_editor_core::model::BracketMode;
use masonry_code_editor::{EditorFontFamily, EditorTheme};
use xilem::masonry::peniko::Color;

use crate::CodeEditor;

impl<State, Action> CodeEditor<State, Action> {
    pub fn font_size(mut self, size: f32) -> Self {
        self.display.font_size = size;
        self
    }

    pub fn font_family(mut self, family: EditorFontFamily) -> Self {
        self.display.font_family = family;
        self
    }

    /// Render variables in italics.
    pub fn variable_italics(mut self, italics: bool) -> Self {
        self.display.variable_italics = italics;
        self
    }

    pub fn show_line_numbers(mut self, show: bool) -> Self {
        self.display.show_line_numbers = show;
        self
    }

    /// Soft-wrap long lines at the viewport width.
    pub fn word_wrap(mut self, wrap: bool) -> Self {
        self.display.word_wrap = wrap;
        self
    }

    pub fn tab_size(mut self, size: usize) -> Self {
        self.display.tab_size = size;
        self
    }

    pub fn theme(mut self, theme: EditorTheme) -> Self {
        self.display.theme = theme;
        self
    }

    pub fn text_color(mut self, color: Color) -> Self {
        self.display.text_color = Some(color);
        self
    }

    /// Which modifier+click triggers go-to-definition (see
    /// `EditorConfig::check_goto_click`).
    pub fn goto_def_click_mode(mut self, mode: u8) -> Self {
        self.display.goto_def_click_mode = mode;
        self
    }

    pub fn bracket_mode(mut self, mode: BracketMode) -> Self {
        self.display.bracket_mode = mode;
        self
    }
}
