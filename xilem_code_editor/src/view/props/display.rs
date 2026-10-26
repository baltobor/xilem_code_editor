// Copyright 2026 the xilem_code_editor developers
// SPDX-License-Identifier: Apache-2.0

use code_editor_core::model::BracketMode;
use masonry_code_editor::{CodeEditorWidget, EditorFontFamily, EditorTheme};
use xilem::Pod;
use xilem::core::Mut;
use xilem::masonry::peniko::Color;
use xilem::masonry::properties::{CaretColor, ContentColor, SelectionColor};

/// Appearance and basic behavior.
#[derive(Clone, PartialEq)]
pub(crate) struct DisplayProps {
    pub font_size: f32,
    pub font_family: EditorFontFamily,
    pub variable_italics: bool,
    pub show_line_numbers: bool,
    pub word_wrap: bool,
    pub tab_size: usize,
    pub theme: EditorTheme,
    pub text_color: Option<Color>,
    pub goto_def_click_mode: u8,
    pub bracket_mode: BracketMode,
}

impl Default for DisplayProps {
    fn default() -> Self {
        Self {
            font_size: 13.0,
            font_family: EditorFontFamily::default(),
            variable_italics: false,
            show_line_numbers: true,
            word_wrap: false,
            tab_size: 4,
            theme: EditorTheme::default(),
            text_color: None,
            goto_def_click_mode: 0,
            bracket_mode: BracketMode::default(),
        }
    }
}

impl DisplayProps {
    pub(crate) fn build(&self, widget: &mut CodeEditorWidget) {
        widget.set_font_family_direct(self.font_family);
        widget.set_variable_italics_direct(self.variable_italics);
        widget.set_show_line_numbers_direct(self.show_line_numbers);
        widget.set_word_wrap_direct(self.word_wrap);
        widget.set_tab_size_direct(self.tab_size);
        widget.set_theme_direct(self.theme.clone());
        widget.set_goto_def_click_mode(self.goto_def_click_mode);
        widget.set_bracket_mode(self.bracket_mode);
    }

    /// Masonry properties the widget reads for caret, selection and text.
    pub(crate) fn properties(&self) -> (CaretColor, SelectionColor, Option<ContentColor>) {
        (
            CaretColor {
                color: self.theme.cursor,
            },
            SelectionColor {
                color: self.theme.selection,
            },
            self.text_color.map(|color| ContentColor { color }),
        )
    }

    pub(crate) fn rebuild(&self, prev: &Self, element: &mut Mut<'_, Pod<CodeEditorWidget>>) {
        if self.font_size != prev.font_size {
            CodeEditorWidget::set_font_size(element, self.font_size);
        }
        if self.font_family != prev.font_family {
            CodeEditorWidget::set_font_family(element, self.font_family);
        }
        if self.variable_italics != prev.variable_italics {
            CodeEditorWidget::set_variable_italics(element, self.variable_italics);
        }
        if self.show_line_numbers != prev.show_line_numbers {
            CodeEditorWidget::set_show_line_numbers(element, self.show_line_numbers);
        }
        if self.word_wrap != prev.word_wrap {
            CodeEditorWidget::set_word_wrap(element, self.word_wrap);
        }
        if self.tab_size != prev.tab_size {
            CodeEditorWidget::set_tab_size(element, self.tab_size);
        }
        if self.theme != prev.theme {
            CodeEditorWidget::set_theme(element, self.theme.clone());
            element.insert_prop(CaretColor {
                color: self.theme.cursor,
            });
            element.insert_prop(SelectionColor {
                color: self.theme.selection,
            });
        }
        if self.text_color != prev.text_color {
            match self.text_color {
                Some(color) => {
                    element.insert_prop(ContentColor { color });
                }
                None => {
                    element.remove_prop::<ContentColor>();
                }
            }
        }
        if self.goto_def_click_mode != prev.goto_def_click_mode {
            element
                .widget
                .set_goto_def_click_mode(self.goto_def_click_mode);
        }
        if self.bracket_mode != prev.bracket_mode {
            element.widget.set_bracket_mode(self.bracket_mode);
        }
    }
}
