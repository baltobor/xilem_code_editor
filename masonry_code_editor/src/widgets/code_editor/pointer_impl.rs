//! `CodeEditorWidget::on_pointer_event_impl` — the entire pointer
//! event router (mouse / scroll / drag).
//!
//! Extracted from `widget/mod.rs` so the orchestrator there stays
//! browseable. The Masonry trait `Widget::on_pointer_event` in
//! `mod.rs` delegates to this single inherent method.
//!
//! Internally this method delegates again to
//! `editor::input::handle_pointer_event` for the actual translation
//! of pointer state → cursor mutation. What happens here is the
//! Masonry-side glue: pre-layout side effects (viewport width
//! update, scroll position), action submission to the host
//! AppModel, and focus / capture management.

use masonry::core::{EventCtx, PointerEvent, PointerScrollEvent, PropertiesMut};
use masonry::dpi::PhysicalPosition;
use masonry::kurbo::Vec2;

use super::CodeEditorWidget;
use crate::action::CodeEditorAction;
use crate::input;

impl CodeEditorWidget {
    /// If `pos` (widget-local) sits within a few pixels of a VCS deletion
    /// marker's boundary in the gutter, return that marker's `after_new_line`
    /// and its removed text. Used for hover-thicken and click-to-balloon.
    fn vcs_deletion_at(&self, pos: masonry::kurbo::Point) -> Option<(usize, String)> {
        if !self.vcs.is_enabled() || pos.x >= self.config.gutter_width {
            return None;
        }
        let diff = self.vcs.diff()?;
        let doc_y = pos.y + self.scroll.pos().y;
        const THRESH: f64 = 5.0;
        diff.deletions.iter().find_map(|d| {
            let boundary = self.lines.y_of_line(d.after_new_line);
            if (doc_y - boundary).abs() <= THRESH {
                Some((d.after_new_line, d.removed_text.clone()))
            } else {
                None
            }
        })
    }

    pub(super) fn on_pointer_event_impl(
        &mut self,
        ctx: &mut EventCtx<'_>,
        _props: &mut PropertiesMut<'_>,
        event: &PointerEvent,
    ) {
        // If paint() discovered wider content, trigger re-layout for horizontal scrolling.
        if self.viewport.needs_width_update {
            self.viewport.needs_width_update = false;
            ctx.request_layout();
        }

        self.hover_pointer_event(ctx, event);

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

        // =========================================================================
        // SCROLL EVENT HANDLING (adapted from xilem Portal)
        // =========================================================================
        // Source: https://github.com/linebender/xilem/blob/05db616/masonry/src/widgets/portal.rs
        // Lines 416-443 (PointerEvent::Scroll handling)
        //
        // We handle scroll events internally to ensure gutter and content use the
        // same scroll position in the same paint call - no sync delays.
        if let PointerEvent::Scroll(PointerScrollEvent { delta, state, .. }) = event {
            // If the pointer is over the Quick Help balloon, scroll that
            // instead of the editor viewport.
            if self.quick_help.is_visible() {
                let scroll_pos = ctx.local_position(state.position);
                if self.quick_help.contains(scroll_pos) {
                    let scale_factor = ctx.scale_factor();
                    let line_px = PhysicalPosition {
                        x: 120.0 * scale_factor,
                        y: 120.0 * scale_factor,
                    };
                    let page_px = PhysicalPosition {
                        x: self.scroll.viewport_size().width * scale_factor,
                        y: self.scroll.viewport_size().height * scale_factor,
                    };
                    let delta_px = delta.to_pixel_delta(line_px, page_px);
                    let masonry::dpi::LogicalPosition { y, .. } =
                        delta_px.to_logical::<f64>(scale_factor);
                    if let Some(new_scroll) = self.quick_help.scroll_by(-y) {
                        ctx.submit_action::<CodeEditorAction>(CodeEditorAction::QuickHelpScroll {
                            offset: new_scroll,
                        });
                        ctx.request_render();
                        ctx.set_handled();
                    }
                    return;
                }
            }

            let scale_factor = ctx.scale_factor();
            let line_px = PhysicalPosition {
                x: 120.0 * scale_factor,
                y: 120.0 * scale_factor,
            };
            let page_px = PhysicalPosition {
                x: self.scroll.viewport_size().width * scale_factor,
                y: self.scroll.viewport_size().height * scale_factor,
            };
            let delta_px = delta.to_pixel_delta(line_px, page_px);
            let masonry::dpi::LogicalPosition { x, y } = delta_px.to_logical::<f64>(scale_factor);
            let scroll_delta = Vec2::new(-x, -y);

            if self.scroll.pan_by(scroll_delta) {
                ctx.request_render();
                ctx.set_handled();
            }
            return;
        }

        // =========================================================================
        // SCROLLBAR INTERACTION (click + drag)
        // =========================================================================
        // Hit-test runs before the text-selection handlers below so a click on
        // the scrollbar does not place a cursor at the rightmost / bottommost
        // column. Drag move/release likewise short-circuit before the
        // text-drag path.
        if let PointerEvent::Down(masonry::core::PointerButtonEvent {
            button: None | Some(masonry::core::PointerButton::Primary),
            state,
            ..
        }) = event
        {
            let pos = ctx.local_position(state.position);
            if self.scroll.handle_scrollbar_down(ctx, pos) {
                return;
            }
        }

        if let PointerEvent::Move(masonry::core::PointerUpdate { current, .. }) = event {
            if self.scroll.is_dragging() && ctx.is_active() {
                let pos = ctx.local_position(current.position);
                self.scroll.handle_scrollbar_drag(ctx, pos);
                return;
            }
            // VCS deletion-marker hover: thicken the marker under the pointer.
            if self.vcs.is_enabled() {
                let pos = ctx.local_position(current.position);
                let hovered = self.vcs_deletion_at(pos).map(|(after, _)| after);
                if self.vcs.set_hovered_deletion(hovered) {
                    ctx.request_render();
                }
            }
        }

        if let PointerEvent::Up(..) | PointerEvent::Cancel(..) = event {
            if self.scroll.take_drag() {
                ctx.set_handled();
                return;
            }
        }

        // Check for gutter click (breakpoint toggle) and modifier clicks.
        // We match both Primary and Secondary buttons because on macOS,
        // Ctrl+Click produces a Secondary (right-click) event.
        if let PointerEvent::Down(masonry::core::PointerButtonEvent { button, state, .. }) = event {
            let is_primary = matches!(button, None | Some(masonry::core::PointerButton::Primary));
            let is_secondary = matches!(button, Some(masonry::core::PointerButton::Secondary));

            let pos = ctx.local_position(state.position);

            // Check for Go to Definition click FIRST (before gutter check)
            // so modifier clicks in gutter area also trigger goto-def.
            let ctrl = state.modifiers.ctrl();
            let cmd = state.modifiers.meta();
            let alt = state.modifiers.alt();

            // On macOS, Ctrl+Click = Secondary button with ctrl modifier.
            // Match the configured mode against actual modifiers + button.
            let goto_triggered = match self.config.goto_def_click_mode {
                0 => {
                    // Ctrl + Cmd + Click: on macOS this fires as Secondary + cmd
                    (ctrl && cmd && is_primary) || (is_secondary && cmd)
                }
                1 => {
                    // Ctrl + Click: on macOS this fires as Secondary button
                    (ctrl && !cmd && is_primary) || (is_secondary && !cmd && !alt)
                }
                2 => cmd && !ctrl && is_primary, // Cmd + Click (always Primary)
                _ => false,
            };

            let hover_triggered =
                alt && ((ctrl && is_primary) || is_secondary || (cmd && is_primary));

            // Plain right-click (no modifiers) - don't handle, let it bubble up for context menu
            let is_plain_right_click = is_secondary && !ctrl && !cmd && !alt;
            if is_plain_right_click {
                // Move cursor to click position before the context menu appears.
                //
                // `text_x` must be in DOCUMENT coordinates, not
                // widget-local: x_to_offset walks the line's glyph
                // run from byte 0 outward and expects an offset
                // relative to the text's own origin. So we strip
                // the gutter strip and add `viewport_pos.x`, the
                // current horizontal scroll. Skipping `+ viewport_pos.x`
                // was the cause of the "context-menu-click selects
                // the wrong word under horizontal scroll" variant
                // of the same coordinate-mismatch bug fixed in
                // `input::pointer::handle_pointer_event`.
                let text_x = (pos.x - self.config.gutter_width).max(0.0) + self.scroll.pos().x;
                let doc_y = pos.y + self.scroll.pos().y;
                let offset = self
                    .lines
                    .offset_at_point(self.document.as_ref(), text_x, doc_y);
                self.cursor.selection.set_caret(offset);
                // Sync the new cursor position into AppModel synchronously so
                // commands invoked from the context menu (Jump to Definition,
                // Show Call Hierarchy, etc.) see the click target as the
                // current cursor — not the previous text-cursor position.
                // Without this, Jump to Definition needed two right-clicks:
                // the first synced the cursor; the second actually jumped.
                let (line, col) = self.cursor_position();
                ctx.submit_action::<CodeEditorAction>(CodeEditorAction::CursorMoved { line, col });
                ctx.submit_action::<CodeEditorAction>(self.selection_changed_action());
                ctx.request_render();
                // Don't set_handled - let context menu wrapper receive the event
                return;
            }

            // Quick Help balloon hit-test: absorb clicks inside, close on outside.
            if self.quick_help.is_visible() {
                if self.quick_help.contains(pos) {
                    // Click inside balloon — absorb to keep it open.
                    ctx.set_handled();
                    return;
                } else if is_primary {
                    // Click outside — close the balloon.
                    ctx.submit_action::<CodeEditorAction>(CodeEditorAction::CloseQuickHelp);
                    // Don't return — let the click also move the cursor.
                }
            }

            // VCS removed-text balloon hit-test. The Insert button takes
            // priority; otherwise absorb clicks inside and close on outside.
            if let Some((after, text)) = self.vcs.balloon().cloned() {
                if is_primary && self.vcs.insert_btn_rect().contains(pos) {
                    self.vcs.close_balloon();
                    ctx.submit_action::<CodeEditorAction>(CodeEditorAction::VcsInsertDeletion {
                        after_new_line: after,
                        text,
                    });
                    ctx.request_render();
                    ctx.set_handled();
                    return;
                }
                if self.vcs.balloon_rect().contains(pos) {
                    ctx.set_handled();
                    return;
                } else if is_primary {
                    self.vcs.close_balloon();
                    ctx.request_render();
                    // Don't return — let the click also place the cursor / etc.
                }
            }

            // Check for completion popup click first (the popup
            // floats on top of everything else, so an item under the
            // pointer wins over both bubble and text clicks).
            if is_primary && self.completion.is_visible() {
                if let Some(index) = self.completion.item_at(pos) {
                    ctx.submit_action::<CodeEditorAction>(CodeEditorAction::AcceptCompletionAt {
                        index,
                    });
                    ctx.request_render();
                    ctx.set_handled();
                    return;
                }
            }

            // Check for diagnostic bubble click (inline Xcode-style popovers).
            // Quick-fix action rows hit-test first; if the click landed
            // on one we apply that fix and ignore the bubble toggle.
            if is_primary {
                if let Some((line, action_index)) = self.diagnostics.action_at(pos) {
                    ctx.submit_action::<CodeEditorAction>(
                        CodeEditorAction::ApplyDiagnosticCodeAction { line, action_index },
                    );
                    ctx.request_render();
                    ctx.set_handled();
                    return;
                }
                if let Some(line) = self.diagnostics.bubble_at(pos) {
                    // Toggle expanded state
                    if self.diagnostics.is_expanded_at(line) {
                        self.diagnostics.collapse();
                    } else {
                        self.diagnostics.expand(line);
                        // Ask the view layer to fetch code actions
                        // scoped to this diagnostic. The result lands
                        // in AppModel::bubble_code_actions and is
                        // picked up by the next paint pass.
                        ctx.submit_action::<CodeEditorAction>(
                            CodeEditorAction::RequestBubbleCodeActions { line },
                        );
                        self.diagnostics.mark_actions_requested(line);
                    }
                    ctx.request_render();
                    ctx.set_handled();
                    return;
                }
            }

            if goto_triggered || hover_triggered {
                // Modifier-click → goto-def / hover. Same viewport-
                // to-document conversion as the plain-right-click
                // branch above: floor the gutter, then add the
                // horizontal scroll so the click lands on the
                // character actually under the pointer even when
                // the user has scrolled right. Without `+ scroll_x`
                // a Cmd+Click on a long line would jump to the
                // definition of a symbol several characters to the
                // left of what the user pointed at.
                let text_x = (pos.x - self.config.gutter_width).max(0.0) + self.scroll.pos().x;
                let doc_y = pos.y + self.scroll.pos().y;
                let offset = self
                    .lines
                    .offset_at_point(self.document.as_ref(), text_x, doc_y);
                // Move cursor to click position first
                self.cursor.selection.set_caret(offset);
                let (line, col) = self.cursor.position(self.document.as_ref());

                if hover_triggered {
                    // Alt + modifier click → hover/documentation preview
                    ctx.submit_action::<CodeEditorAction>(CodeEditorAction::HoverRequest {
                        line,
                        col,
                    });
                } else {
                    // Modifier click → go to definition
                    ctx.submit_action::<CodeEditorAction>(CodeEditorAction::GotoDefinition {
                        line,
                        col,
                    });
                }
                ctx.request_render();
                ctx.set_handled();
                return;
            }

            // Gutter click (breakpoint toggle, job-button run) — only for primary button
            if is_primary && pos.x < self.config.gutter_width && self.config.show_line_numbers {
                // VCS deletion marker takes priority when the click lands on its
                // (hover-thickened) boundary; elsewhere on the line, fall through
                // to the breakpoint toggle.
                if let Some((after, removed)) = self.vcs_deletion_at(pos) {
                    self.vcs.open_balloon(after, removed);
                    ctx.request_render();
                    ctx.set_handled();
                    return;
                }
                // Convert viewport y to document y by adding scroll offset
                let doc_y = pos.y + self.scroll.pos().y;
                let clicked_line = self.lines.line_at_y(doc_y);
                let line_1based = clicked_line + 1;
                // Controls get the click first (topmost first); unclaimed
                // clicks are reported as a plain gutter click (e.g. the
                // host toggles a breakpoint).
                let gutter_x = pos.x + self.scroll.pos().x;
                let action = match self.gutter_registry.on_click(line_1based, gutter_x, doc_y) {
                    Some(control) => {
                        ctx.request_render();
                        CodeEditorAction::GutterControlClicked {
                            control,
                            line: line_1based,
                        }
                    }
                    None => CodeEditorAction::GutterClicked { line: line_1based },
                };
                ctx.submit_action::<CodeEditorAction>(action);
                ctx.set_handled();
                return;
            }
        }

        // The two scroll values are how `handle_pointer_event`
        // translates widget-local pointer coordinates into document
        // coordinates that x_to_offset / line_at_y can index into.
        // `viewport_pos.x` was the missing piece before this fix:
        // x_to_offset received only the on-screen pixel offset, so
        // the moment the user scrolled horizontally clicks landed
        // ~scroll_x px to the left of where the pointer was. Search
        // for "scroll_x" in input/pointer.rs for the full diagram.
        let result = input::handle_pointer_event(
            ctx,
            &mut self.cursor,
            self.document.as_ref(),
            &self.lines,
            event,
            self.config.gutter_width,
            self.scroll.pos().x,
            self.scroll.pos().y,
        );

        if result.cursor_moved {
            // Reset blink animation on cursor movement
            self.animator.reset();
            ctx.request_anim_frame();
            ctx.request_render();

            // Clear search highlight on user click/interaction
            self.is_search_highlight = false;

            // Scroll viewport to keep cursor visible (both horizontally and vertically)
            if let Some(rect) = self.cursor_rect() {
                ctx.request_scroll_to(rect);
            }

            let (line, col) = self.cursor_position();
            ctx.submit_action::<CodeEditorAction>(CodeEditorAction::CursorMoved { line, col });
            ctx.submit_action::<CodeEditorAction>(self.selection_changed_action());
        }
    }
}
