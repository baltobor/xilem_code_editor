// Copyright 2026 the xilem_code_editor developers
// SPDX-License-Identifier: Apache-2.0

use masonry_code_editor::CodeEditorAction;

use super::props::{
    CompletionProps, DisplayProps, GutterControlsProps, GutterProps, HighlightingProps, HoverProps,
    LanguageDataProps, QuickHelpProps,
};
use crate::command::Command;

pub(crate) type ChangedCallback<State, Action> =
    Box<dyn Fn(&mut State, String) -> Action + Send + Sync + 'static>;
pub(crate) type EventHandler<State, Action> =
    Box<dyn Fn(&mut State, CodeEditorAction) -> Option<Action> + Send + Sync + 'static>;

/// A code editor. Create it with [`code_editor`].
///
/// ```ignore
/// // A Markdown editor:
/// code_editor(state.text.clone(), |state: &mut State, text| state.text = text)
///     .language("md")
///     .highlighter(highlighter.clone())
///     .word_wrap(true)
/// ```
pub struct CodeEditor<State, Action = ()> {
    pub(crate) contents: String,
    pub(crate) on_changed: ChangedCallback<State, Action>,
    pub(crate) on_event: Option<EventHandler<State, Action>>,
    pub(crate) display: DisplayProps,
    pub(crate) highlighting: HighlightingProps,
    pub(crate) gutter: GutterProps,
    pub(crate) gutter_controls: GutterControlsProps,
    pub(crate) language_data: LanguageDataProps,
    pub(crate) completion: CompletionProps,
    pub(crate) quick_help: QuickHelpProps,
    pub(crate) hover: HoverProps,
    pub(crate) commands: Vec<Command>,
}

/// An editor showing `contents`; `on_changed` receives the full text
/// after every edit.
pub fn code_editor<State, Action>(
    contents: impl Into<String>,
    on_changed: impl Fn(&mut State, String) -> Action + Send + Sync + 'static,
) -> CodeEditor<State, Action> {
    CodeEditor {
        contents: contents.into(),
        on_changed: Box::new(on_changed),
        on_event: None,
        display: DisplayProps::default(),
        highlighting: HighlightingProps::default(),
        gutter: GutterProps::default(),
        gutter_controls: GutterControlsProps::default(),
        language_data: LanguageDataProps::default(),
        completion: CompletionProps::default(),
        quick_help: QuickHelpProps::default(),
        hover: HoverProps::default(),
        commands: Vec::new(),
    }
}
