//! `CodeEditorWidget::on_text_event_impl` — the entire keyboard /
//! IME / paste event router.
//!
//! Extracted from `widget/mod.rs` so the orchestrator there stays
//! browseable. The Masonry trait `Widget::on_text_event` in
//! `mod.rs` delegates to this single inherent method.
//!
//! What lives here:
//!  - the priority chain through `editor::input::handle_key_event`,
//!  - the snippet-session lifecycle (start, navigate, commit),
//!  - completion-popup acceptance + dismissal,
//!  - bubble (diagnostic-popover) keyboard nav,
//!  - bracket auto-close ghost commit,
//!  - paste handling,
//!  - all the AppModel-bound action submissions (LSP requests,
//!    cursor-moved notifications, undo / redo / cut / copy, etc.).

use masonry::core::keyboard::KeyState;
use masonry::core::{EventCtx, PropertiesMut, TextEvent};

use super::CodeEditorWidget;
use crate::action::CodeEditorAction;
use crate::input;
use code_editor_core::cursor::Cursor;

impl CodeEditorWidget {
    pub(super) fn on_text_event_impl(
        &mut self,
        ctx: &mut EventCtx<'_>,
        _props: &mut PropertiesMut<'_>,
        event: &TextEvent,
    ) {
        // Handle pending scroll from external navigation (goto_line)
        // (Primary handling is in on_anim_frame; this is a fallback)
        if self.viewport.needs_scroll_to_cursor {
            if let Some(rect) = self.cursor_rect() {
                if self.scroll_to_show_rect(rect) {
                    self.viewport.needs_scroll_to_cursor = false;
                    ctx.request_render();
                }
                // If scroll failed, keep flag set for retry in on_anim_frame
            }
        }

        self.hover_text_event(ctx, event);

        // Reset blink animation on any text event
        self.animator.reset();
        ctx.request_anim_frame();

        match event {
            TextEvent::Keyboard(key_event) => {
                if key_event.state != KeyState::Down {
                    return;
                }

                let shift = key_event.modifiers.shift();
                let action_mod = if cfg!(target_os = "macos") {
                    key_event.modifiers.meta()
                } else {
                    key_event.modifiers.ctrl()
                };
                let alt = key_event.modifiers.alt();

                // Snapshot cursor and begin transaction before the edit
                let cursor_snapshot = self.cursor.selection.regions().to_vec();
                self.document.begin_edit_group(&cursor_snapshot);

                // Capture pre-edit doc length and primary selection so
                // the snippet session (if any) can map the edit to the
                // active tab stop. The primary selection's range is
                // what `cursor.insert` will replace; bytes-after will
                // tell us the delta.
                let pre_doc_len = self.document.len_bytes();
                let pre_primary = self
                    .cursor
                    .selection
                    .regions()
                    .first()
                    .map(|r| r.min()..r.max());

                // Bubble nav is active when the bubble is expanded
                // AND its action cache has at least one entry —
                // before that, Tab/Enter still need to behave as
                // completion accept / line break.
                let bubble_action_count = self.diagnostics.pending_action_count();

                let result = input::handle_key_event(
                    ctx,
                    &mut self.cursor,
                    self.document.as_mut(),
                    &self.lines,
                    &key_event.key,
                    shift,
                    action_mod,
                    alt,
                    self.config.tab_size,
                    self.completion.bracket_ghost(),
                    self.completion.completion_ghost_active(),
                    self.completion.is_visible(),
                    self.completion.popup_user_navigated(),
                    bubble_action_count,
                    self.snippet_session.is_some(),
                );

                if result.handled {
                    ctx.set_handled();
                }

                // Snippet session: navigate / commit before bracket
                // ghost or any other action handlers run. The session
                // is the source of truth for the active stop's range
                // and we update the cursor selection from it.
                if result.snippet_next || result.snippet_prev {
                    let target = if let Some(s) = self.snippet_session.as_mut() {
                        if result.snippet_next {
                            s.tab_next()
                        } else {
                            s.tab_prev()
                        }
                    } else {
                        None
                    };
                    if let Some(range) = target {
                        // Clamp to current document length — defensive
                        // against any stale range that survived an
                        // edit we didn't track.
                        let max = self.document.len_bytes();
                        let s = range.start.min(max);
                        let e = range.end.min(max);
                        self.cursor.selection.set_single(s, e);
                        // If we just landed on `$0`, the navigation is
                        // done — commit so further Tabs go to indent
                        // again.
                        if self
                            .snippet_session
                            .as_ref()
                            .map(|s| s.at_final())
                            .unwrap_or(false)
                        {
                            self.snippet_session = None;
                        }
                        ctx.request_paint_only();
                    } else {
                        // No further stop — commit and let the caret
                        // stay where it is.
                        self.snippet_session = None;
                    }
                }
                if result.snippet_commit {
                    // Esc / Return at the final stop: clear the session
                    // and leave the caret where it is. If we're not
                    // already at $0, jump there first so Return acts
                    // like "I'm done — go to the snippet's natural
                    // commit point".
                    if let Some(s) = self.snippet_session.as_mut() {
                        if !s.at_final() {
                            // Walk forward to the $0 stop.
                            let mut target = None;
                            while !s.at_final() {
                                target = s.tab_next();
                                if target.is_none() {
                                    break;
                                }
                            }
                            if let Some(range) = target {
                                let max = self.document.len_bytes();
                                let pos = range.start.min(max);
                                self.cursor.selection.set_caret(pos);
                            }
                        }
                    }
                    self.snippet_session = None;
                    ctx.request_paint_only();
                }

                // Handle bracket ghost state changes
                if result.accept_bracket_ghost {
                    // Tab pressed: Insert the ghost character inline
                    if let Some(ghost) = self.completion.take_bracket_ghost() {
                        self.cursor
                            .insert(self.document.as_mut(), &ghost.to_string());
                        let cursor_after = self.cursor.selection.regions().to_vec();
                        self.document.end_edit_group(&cursor_after);
                        let line_count = self.document.line_count();
                        self.lines.resize(line_count);
                        self.lines.invalidate_all();
                        self.rehighlight();
                        let text = self.document.to_string();
                        self.viewport.last_synced_revision = self.document.revision();
                        ctx.submit_action::<CodeEditorAction>(CodeEditorAction::ContentChanged(
                            text,
                        ));
                        ctx.request_layout();
                    }
                } else if result.accept_bracket_ghost_newline {
                    // Enter pressed: Insert newline, indent, closing bracket, then position cursor
                    if let Some(ghost) = self.completion.take_bracket_ghost() {
                        let doc = self.document.as_mut();
                        let offset = self.cursor.offset();
                        let line = doc.line_of_offset(offset);

                        // Get current line's content + indentation. We
                        // need both: the indentation to size the new
                        // lines, and the FULL line up to the cursor to
                        // decide whether the expanded block should end
                        // with `};` (assignment / let-binding context)
                        // or just `}` (function body, struct, control
                        // flow).
                        let (indent, line_before_brace) =
                            if let Some(line_content) = doc.line_content(line) {
                                let content_str = line_content.to_string();
                                let indent = content_str
                                    .chars()
                                    .take_while(|c| *c == ' ' || *c == '\t')
                                    .collect::<String>();
                                // The cursor is RIGHT AFTER the `{` that
                                // triggered the ghost. Compute the column
                                // index of the cursor on this line and
                                // slice the line up to (but not including)
                                // the `{`.
                                let line_start = doc.offset_of_line(line);
                                let col_in_line = offset.saturating_sub(line_start);
                                // The `{` itself sits at col_in_line - 1.
                                let before_brace = if col_in_line > 0 {
                                    let cut = col_in_line - 1;
                                    let cut = cut.min(content_str.len());
                                    content_str.get(..cut).unwrap_or("").to_string()
                                } else {
                                    String::new()
                                };
                                (indent, before_brace)
                            } else {
                                (String::new(), String::new())
                            };

                        // Add extra indent for content inside brackets
                        let inner_indent = format!("{}    ", indent);

                        // Smart trailing-semicolon: when the `{` was
                        // closing an expression-as-statement
                        // (`let foo = match l {` or `let foo = {`),
                        // append `;` after the closing `}`. See
                        // `code_editor_core::cursor::bracket_continuation` for
                        // the heuristic and its tests.
                        let trailing = if ghost == '}'
                            && code_editor_core::cursor::bracket_continuation::needs_trailing_semicolon(
                                &line_before_brace,
                            ) {
                            ";"
                        } else {
                            ""
                        };

                        // Build: newline + inner indent + newline + original indent + closing bracket [+ trailing semi]
                        let insert_text =
                            format!("\n{}\n{}{}{}", inner_indent, indent, ghost, trailing);
                        self.cursor.insert(doc, &insert_text);

                        // Move cursor back to the inner line (after inner_indent)
                        let inner_line_offset = offset + 1 + inner_indent.len();
                        self.cursor.selection.set_caret(inner_line_offset);

                        let cursor_after = self.cursor.selection.regions().to_vec();
                        self.document.end_edit_group(&cursor_after);
                        let line_count = self.document.line_count();
                        self.lines.resize(line_count);
                        self.lines.invalidate_all();
                        self.rehighlight();
                        let text = self.document.to_string();
                        self.viewport.last_synced_revision = self.document.revision();
                        ctx.submit_action::<CodeEditorAction>(CodeEditorAction::ContentChanged(
                            text,
                        ));
                        // Cursor is now on the empty inner line of
                        // the expanded block. Ask the LSP for any
                        // high-confidence code actions at this
                        // position; rust-analyzer responds with
                        // "Fill match arms" / "Wrap in Ok" / etc.
                        // when applicable. The response is routed
                        // into `completion_code_action_items` and
                        // surfaces in the completion popup as
                        // additional rows.
                        let (action_line, action_col) = self.cursor_position();
                        ctx.submit_action::<CodeEditorAction>(
                            CodeEditorAction::RequestCodeActionsForPopup {
                                line: action_line,
                                col: action_col,
                            },
                        );
                        ctx.request_layout();
                    }
                } else if result.accept_completion_ghost {
                    // Tab pressed with completion ghost visible: emit action for model to handle
                    // The model's accept_completion() will insert the expanded snippet
                    ctx.submit_action::<CodeEditorAction>(CodeEditorAction::AcceptCompletion);
                    // Clear completion ghost since it's being accepted
                    self.completion.set_completion_ghost_text(None);
                } else if let Some(ghost) = result.set_bracket_ghost {
                    use code_editor_core::model::BracketMode;
                    match self.bracket_mode {
                        BracketMode::Ghost => {
                            // Xcode-style: show ghost, Tab/Enter to accept
                            self.completion.set_bracket_ghost(Some(ghost));
                            ctx.request_paint_only();
                        }
                        BracketMode::AutoClose => {
                            // VSCode-style: insert both immediately, cursor between
                            self.cursor
                                .insert(self.document.as_mut(), &ghost.to_string());
                            self.cursor.move_left(self.document.as_ref());
                            let cursor_after = self.cursor.selection.regions().to_vec();
                            self.document.end_edit_group(&cursor_after);
                            let line_count = self.document.line_count();
                            self.lines.resize(line_count);
                            self.lines.invalidate_all();
                            self.rehighlight();
                            let text = self.document.to_string();
                            self.viewport.last_synced_revision = self.document.revision();
                            ctx.submit_action::<CodeEditorAction>(
                                CodeEditorAction::ContentChanged(text),
                            );
                            ctx.request_layout();
                        }
                    }
                } else if result.clear_bracket_ghost {
                    self.completion.set_bracket_ghost(None);
                }

                if result.edited {
                    // Close the transaction with the new cursor state
                    let cursor_after = self.cursor.selection.regions().to_vec();
                    self.document.end_edit_group(&cursor_after);

                    // Clear search highlight on edit
                    self.is_search_highlight = false;

                    // Snippet session: classify the edit. We need
                    // the actual byte range that was replaced and
                    // the byte length that replaced it so the
                    // session can shift later stops correctly.
                    //
                    // Two shapes to handle:
                    //   (a) The pre-edit primary selection was
                    //       non-empty — a typed character / paste
                    //       that replaced the selection. The
                    //       replaced range is `pre_primary` and
                    //       `new_len = pre_old_len + delta`.
                    //   (b) The pre-edit primary was empty (just
                    //       a caret) — could be insert-at-caret,
                    //       Backspace, or forward Delete. These
                    //       shapes are distinguished by the
                    //       post-edit primary caret position
                    //       relative to the pre-edit caret and
                    //       the doc-length delta:
                    //         delta > 0 → insert at caret;
                    //                     range = caret..caret,
                    //                     new_len = delta.
                    //         delta < 0 && post == pre → forward
                    //                     Delete; range =
                    //                     caret..(caret + |delta|),
                    //                     new_len = 0.
                    //         delta < 0 && post < pre → Backspace
                    //                     / word-delete; range =
                    //                     post..pre, new_len = 0.
                    //         delta == 0 → no-op, skip.
                    //
                    // The earlier impl only handled (a) and the
                    // insert-at-caret subcase of (b); Backspace /
                    // Delete inside a placeholder slipped through
                    // as (zero-width edit, zero new_len), leaving
                    // the session's tracked ranges 1+ bytes off
                    // and the next Tab landing on the wrong slice
                    // of text.
                    if let (Some(session), Some(pre_range)) =
                        (self.snippet_session.as_mut(), pre_primary.as_ref())
                    {
                        let post_doc_len = self.document.len_bytes();
                        let delta_signed: isize = post_doc_len as isize - pre_doc_len as isize;
                        let pre_old_len = pre_range.end - pre_range.start;
                        let post_caret = self
                            .cursor
                            .selection
                            .regions()
                            .first()
                            .map(|r| r.min())
                            .unwrap_or(pre_range.start);

                        let (edit_start, edit_end_old, new_len) = if pre_old_len > 0 {
                            // (a) Selection-replace.
                            let new_len = (pre_old_len as isize + delta_signed).max(0) as usize;
                            (pre_range.start, pre_range.end, new_len)
                        } else if delta_signed > 0 {
                            // (b1) Insert at caret.
                            (pre_range.start, pre_range.start, delta_signed as usize)
                        } else if delta_signed < 0 {
                            let removed = (-delta_signed) as usize;
                            if post_caret < pre_range.start {
                                // (b2) Backspace / word-delete.
                                (post_caret, pre_range.start, 0)
                            } else {
                                // (b3) Forward Delete.
                                (pre_range.start, pre_range.start + removed, 0)
                            }
                        } else {
                            // No-op — skip session update.
                            (pre_range.start, pre_range.start, 0)
                        };

                        // No-op edits don't need session updates.
                        let edit_changed = edit_start != edit_end_old || new_len != 0;
                        if edit_changed {
                            match session.before_edit(&(edit_start..edit_end_old)) {
                                code_editor_core::snippet::EditDisposition::Allow => {
                                    session.after_edit(edit_start, edit_end_old, new_len);
                                }
                                code_editor_core::snippet::EditDisposition::Commit => {
                                    self.snippet_session = None;
                                }
                            }
                        }
                    }

                    // Document changed — resize line cache, invalidate, notify
                    let line_count = self.document.line_count();
                    self.lines.resize(line_count);
                    self.lines.invalidate_all();
                    self.rehighlight();

                    let text = self.document.to_string();
                    self.viewport.last_synced_revision = self.document.revision();
                    ctx.submit_action::<CodeEditorAction>(CodeEditorAction::ContentChanged(text));
                    // Scroll to keep cursor visible after the edit
                    if let Some(rect) = self.cursor_rect() {
                        ctx.request_scroll_to(rect);
                    }
                    ctx.request_layout();
                } else {
                    // No edit happened — discard the empty transaction
                    self.document.end_edit_group(&cursor_snapshot);

                    if result.cursor_moved {
                        // Clear search highlight on cursor movement
                        self.is_search_highlight = false;
                        ctx.request_render();
                    }
                }

                // Handle undo/redo signals
                if result.undo {
                    if let Some(regions) = self.document.undo() {
                        self.cursor = Cursor::from_selection(
                            code_editor_core::selection::Selection::from_regions(regions),
                        );
                        let line_count = self.document.line_count();
                        self.lines.resize(line_count);
                        self.lines.invalidate_all();
                        // Re-parse: without this the tree-sitter parse
                        // tree still reflects the post-edit byte ranges,
                        // so spans point at the wrong characters in the
                        // rolled-back buffer.
                        self.rehighlight();
                        let text = self.document.to_string();
                        self.viewport.last_synced_revision = self.document.revision();
                        ctx.submit_action::<CodeEditorAction>(CodeEditorAction::ContentChanged(
                            text,
                        ));
                        ctx.request_layout();
                    }
                }

                if result.redo {
                    if let Some(regions) = self.document.redo() {
                        self.cursor = Cursor::from_selection(
                            code_editor_core::selection::Selection::from_regions(regions),
                        );
                        let line_count = self.document.line_count();
                        self.lines.resize(line_count);
                        self.lines.invalidate_all();
                        self.rehighlight();
                        let text = self.document.to_string();
                        self.viewport.last_synced_revision = self.document.revision();
                        ctx.submit_action::<CodeEditorAction>(CodeEditorAction::ContentChanged(
                            text,
                        ));
                        ctx.request_layout();
                    }
                }

                // Handle Go to Definition / Hover keyboard shortcuts
                if result.goto_definition {
                    let (line, col) = self.cursor_position();
                    ctx.submit_action::<CodeEditorAction>(CodeEditorAction::GotoDefinition {
                        line,
                        col,
                    });
                }
                if result.hover_request {
                    let (line, col) = self.cursor_position();
                    ctx.submit_action::<CodeEditorAction>(CodeEditorAction::HoverRequest {
                        line,
                        col,
                    });
                }

                // Handle font size shortcuts
                if result.increase_font_size {
                    ctx.submit_action::<CodeEditorAction>(CodeEditorAction::IncreaseFontSize);
                }
                if result.decrease_font_size {
                    ctx.submit_action::<CodeEditorAction>(CodeEditorAction::DecreaseFontSize);
                }

                // Statement terminators / structural punctuation
                // dismiss the completion popup. Two flavours, same
                // intent: the previously-suggested item is no longer
                // a useful continuation of what the user is typing.
                //
                // Closers + sentinels (`;`, `,`, `}`, `)`, `]`):
                //   typing `;` ends a statement, so the method that
                //   was suggested on the value-before-`;` is stale;
                //   if we leave the popup up, the next Enter would
                //   silently accept it (turning `let x = 1.0;<Enter>`
                //   into `let x = 1.0;type_id()`).
                //
                // Openers (`{`, `(`, `[`):
                //   typing `{` after `match l ` puts the caret in a
                //   fresh empty block. The popup may still be showing
                //   stale items from typing `match l` — `loop`,
                //   `let`, etc. — and a non-zero `completion_selected`
                //   means the next snippet-aware insert would splice
                //   `loop {…}` or `let …` into the empty `{}` the
                //   user just opened. Dismissing here keeps fresh
                //   blocks empty until the user types again.
                if let Some(ch) = result.typed_char {
                    if matches!(ch, ';' | ',' | '}' | ')' | ']' | '{' | '(' | '[') {
                        ctx.submit_action::<CodeEditorAction>(CodeEditorAction::DismissPopups);
                    }
                }

                // Handle completion/code actions/signature help requests.
                //
                // The decision boils down to OR-ing three signals:
                //
                //   - the input handler explicitly asked for it
                //     (Ctrl+Space, snippet commit, etc.),
                //   - the typed char is one of the LSP-declared
                //     trigger chars (filtered for the bracket-y
                //     ones),
                //   - the typed char is identifier-like AND we're
                //     either already showing the popup or
                //     extending an existing identifier.
                //
                // The signal helpers are pure free functions in
                // `completion_trigger`; see that module for the
                // exact rules and the unit tests pinning them.

                let trigger_char_match = result.typed_char.is_some_and(|ch| {
                    code_editor_core::completion::trigger::is_trigger_char(
                        ch,
                        self.completion.trigger_chars(),
                    ) || code_editor_core::completion::trigger::completes_trigger_sequence(
                        self.cursor.offset(),
                        self.document.as_ref(),
                        self.completion.trigger_chars(),
                    )
                });

                let identifier_auto_trigger = result.typed_char.map_or(false, |ch| {
                    code_editor_core::completion::trigger::should_auto_trigger(
                        ch,
                        self.cursor.offset(),
                        self.completion.is_visible(),
                        self.document.as_ref(),
                    )
                });

                // If the typed char is a word boundary, signal
                // the model to clear any post-Esc completion
                // suppression. Done unconditionally; the model's
                // notify is cheap (a bool toggle) and there's no
                // harm in clearing an already-clear flag.
                if let Some(ch) = result.typed_char {
                    if !ch.is_alphanumeric() && ch != '_' {
                        ctx.submit_action::<CodeEditorAction>(CodeEditorAction::WordBoundaryTyped);
                    }
                }

                let should_trigger_completion =
                    result.request_completion || trigger_char_match || identifier_auto_trigger;

                // Debug: trace completion trigger
                if result.typed_char.is_some() || result.request_completion {
                    tracing::debug!(
                        typed_char = ?result.typed_char,
                        explicit_request = result.request_completion,
                        trigger_chars = ?self.completion.trigger_chars(),
                        trigger_char_match,
                        should_trigger = should_trigger_completion,
                        "Completion trigger check"
                    );
                }

                if should_trigger_completion {
                    let (line, col) = self.cursor_position();
                    tracing::debug!(line, col, "Submitting RequestCompletion action");
                    // Distinguish explicit Ctrl+Space from auto-
                    // trigger so the model can honour the
                    // post-Esc suppression flag for auto-triggers
                    // while letting Ctrl+Space override it.
                    if result.request_completion {
                        ctx.submit_action::<CodeEditorAction>(
                            CodeEditorAction::RequestCompletionExplicit { line, col },
                        );
                    } else {
                        ctx.submit_action::<CodeEditorAction>(
                            CodeEditorAction::RequestCompletion { line, col },
                        );
                    }
                }
                if result.request_code_actions {
                    let (line, col) = self.cursor_position();
                    ctx.submit_action::<CodeEditorAction>(CodeEditorAction::RequestCodeActions {
                        line,
                        col,
                    });
                }
                if result.request_signature_help {
                    let (line, col) = self.cursor_position();
                    ctx.submit_action::<CodeEditorAction>(CodeEditorAction::RequestSignatureHelp {
                        line,
                        col,
                    });
                }
                if result.request_help {
                    ctx.submit_action::<CodeEditorAction>(CodeEditorAction::RequestHelp);
                }
                if result.dismiss_popups {
                    ctx.submit_action::<CodeEditorAction>(CodeEditorAction::DismissPopups);
                }
                if result.dismiss_completion_by_esc {
                    ctx.submit_action::<CodeEditorAction>(CodeEditorAction::DismissCompletionByEsc);
                }

                // Handle completion navigation and acceptance
                if result.completion_select_prev {
                    ctx.submit_action::<CodeEditorAction>(CodeEditorAction::CompletionSelectPrev);
                }
                if result.completion_select_next {
                    ctx.submit_action::<CodeEditorAction>(CodeEditorAction::CompletionSelectNext);
                }
                if result.accept_completion {
                    ctx.submit_action::<CodeEditorAction>(CodeEditorAction::AcceptCompletion);
                }

                // Handle bubble (quick-fix) navigation and acceptance.
                // Selection / collapse are widget-internal — we move
                // `selected_action_index` and re-render. Apply has to
                // round-trip through the view so AppModel can run the
                // workspace edit.
                if result.bubble_select_prev || result.bubble_select_next {
                    if self.diagnostics.select_step(result.bubble_select_prev) {
                        ctx.request_render();
                    }
                }
                if result.apply_selected_bubble_action {
                    if let Some((line, action_index)) = self.diagnostics.expanded_selection() {
                        ctx.submit_action::<CodeEditorAction>(
                            CodeEditorAction::ApplyDiagnosticCodeAction { line, action_index },
                        );
                    }
                }
                if result.collapse_bubble {
                    self.diagnostics.collapse();
                    ctx.request_render();
                }

                if result.cursor_moved || result.undo || result.redo {
                    // Scroll viewport to keep cursor visible
                    if let Some(rect) = self.cursor_rect() {
                        ctx.request_scroll_to(rect);
                    }
                    let (line, col) = self.cursor_position();
                    ctx.submit_action::<CodeEditorAction>(CodeEditorAction::CursorMoved {
                        line,
                        col,
                    });
                    ctx.submit_action::<CodeEditorAction>(self.selection_changed_action());
                }
            }

            TextEvent::WindowFocusChange(_) => {
                ctx.request_paint_only();
            }

            TextEvent::Ime(e) => {
                use masonry::core::Ime;
                match e {
                    Ime::Commit(text) => self.insert_pasted_text(ctx, text),
                    Ime::Preedit(_text, _cursor) => {
                        // TODO: IME preedit support (CJK input)
                        // For now, we skip preedit and only handle commits
                    }
                    Ime::Disabled | Ime::Enabled => {}
                }
                ctx.set_handled();
            }

            TextEvent::ClipboardPaste(text) => self.insert_pasted_text(ctx, text),
        }
    }

    /// Shared by IME commit (CJK input, etc.) and clipboard paste — both
    /// insert a block of external text and need identical post-edit
    /// housekeeping (resize/invalidate/rehighlight/notify).
    fn insert_pasted_text(&mut self, ctx: &mut EventCtx<'_>, text: &str) {
        let cursor_snapshot = self.cursor.selection.regions().to_vec();
        self.document.begin_edit_group(&cursor_snapshot);
        let result = input::handle_paste(&mut self.cursor, self.document.as_mut(), text);
        if !result.edited {
            self.document.end_edit_group(&cursor_snapshot);
            return;
        }

        let cursor_after = self.cursor.selection.regions().to_vec();
        self.document.end_edit_group(&cursor_after);
        self.is_search_highlight = false;
        let line_count = self.document.line_count();
        self.lines.resize(line_count);
        self.lines.invalidate_all();
        // Recompute syntax highlights against the new text. Without this,
        // cached highlight spans from before the paste still reference byte
        // ranges from the OLD text. When the pasted text contains
        // multi-byte chars (CJK IME input, box-drawing characters like
        // `├──`), those stale ranges land inside the new multi-byte chars
        // and parley panics with "byte index N is not a char boundary".
        self.rehighlight();
        let new_text = self.document.to_string();
        self.viewport.last_synced_revision = self.document.revision();
        ctx.submit_action::<CodeEditorAction>(CodeEditorAction::ContentChanged(new_text));
        if let Some(rect) = self.cursor_rect() {
            ctx.request_scroll_to(rect);
        }
        ctx.request_layout();
    }
}
