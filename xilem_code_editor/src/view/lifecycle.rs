// Copyright 2026 the xilem_code_editor developers
// SPDX-License-Identifier: Apache-2.0

use masonry_code_editor::{CodeEditorAction, CodeEditorWidget};
use xilem::core::{MessageCtx, MessageResult, Mut, View, ViewMarker};
use xilem::{Pod, ViewCtx};

use super::code_editor::CodeEditor;
use crate::command::{apply_command, edits_text};

/// Highest command id already run.
pub struct CodeEditorViewState {
    last_command_id: u64,
}

impl<State, Action> ViewMarker for CodeEditor<State, Action> {}

impl<State: 'static, Action: 'static> View<State, Action, ViewCtx> for CodeEditor<State, Action> {
    type Element = Pod<CodeEditorWidget>;
    type ViewState = CodeEditorViewState;

    fn build(&self, ctx: &mut ViewCtx, _: &mut State) -> (Self::Element, Self::ViewState) {
        let mut widget = CodeEditorWidget::new(&self.contents, self.display.font_size);
        self.display.build(&mut widget);
        self.highlighting.build(&mut widget);
        self.gutter.build(&mut widget);
        self.gutter_controls.build(&mut widget);
        self.language_data.build(&mut widget);
        self.completion.build(&mut widget);

        // Commands present at build: only navigation makes sense on a fresh
        // widget; everything else is just marked as done.
        for command in &self.commands {
            if let crate::EditorCommand::GotoLine {
                line,
                col,
                highlight_len,
                alignment,
            } = &command.command
            {
                widget.set_goto_line_direct(*line, *col, *highlight_len, *alignment);
            }
        }
        let last_command_id = self.commands.iter().map(|c| c.id).max().unwrap_or(0);

        let mut pod = ctx.with_action_widget(|ctx| ctx.create_pod(widget));
        let (caret, selection, content) = self.display.properties();
        pod.new_widget.properties.insert(caret);
        pod.new_widget.properties.insert(selection);
        if let Some(content) = content {
            pod.new_widget.properties.insert(content);
        }
        (pod, CodeEditorViewState { last_command_id })
    }

    fn rebuild(
        &self,
        prev: &Self,
        state: &mut Self::ViewState,
        _ctx: &mut ViewCtx,
        mut element: Mut<'_, Self::Element>,
        _: &mut State,
    ) {
        let new_commands: Vec<_> = self
            .commands
            .iter()
            .filter(|c| c.id > state.last_command_id)
            .collect();

        // A text edit in this rebuild supersedes the (pre-edit) contents.
        let edits_pending = new_commands.iter().any(|c| edits_text(&c.command));
        if !edits_pending && element.widget.text_string() != self.contents {
            CodeEditorWidget::set_text(&mut element, &self.contents);
        }

        self.display.rebuild(&prev.display, &mut element);
        self.highlighting.rebuild(&prev.highlighting, &mut element);
        self.gutter.rebuild(&prev.gutter, &mut element);
        self.language_data
            .rebuild(&prev.language_data, &mut element);
        self.completion.rebuild(&prev.completion, &mut element);
        self.quick_help.rebuild(&prev.quick_help, &mut element);
        self.hover.rebuild(&prev.hover, &mut element);

        for command in new_commands {
            apply_command(&mut element, &command.command);
            state.last_command_id = state.last_command_id.max(command.id);
        }
    }

    fn teardown(&self, _: &mut Self::ViewState, _: &mut ViewCtx, _: Mut<'_, Self::Element>) {}

    fn message(
        &self,
        _: &mut Self::ViewState,
        message: &mut MessageCtx,
        _: Mut<'_, Self::Element>,
        app_state: &mut State,
    ) -> MessageResult<Action> {
        debug_assert!(
            message.remaining_path().is_empty(),
            "id path should be empty in CodeEditor::message"
        );
        let Some(action) = message.take_message::<CodeEditorAction>() else {
            tracing::warn!("CodeEditor received an unexpected message");
            return MessageResult::Stale;
        };
        match *action {
            CodeEditorAction::ContentChanged(text) => {
                MessageResult::Action((self.on_changed)(app_state, text))
            }
            event => match &self.on_event {
                Some(handler) => {
                    handler(app_state, event).map_or(MessageResult::Nop, MessageResult::Action)
                }
                None => MessageResult::Nop,
            },
        }
    }
}
