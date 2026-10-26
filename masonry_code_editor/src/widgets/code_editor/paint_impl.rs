//! `CodeEditorWidget::paint_impl` — the entire paint pass.
//!
//! Extracted from `widget/mod.rs` so the orchestrator there can stay
//! browseable. The Masonry `Widget::paint` trait method in `mod.rs`
//! delegates to this single inherent method.
//!
//! ## Paint order (matters!)
//!
//! 1. Column guide (text-width 80 vertical line)
//! 2. Active line highlight (focused only)
//! 3. Selection rectangles (incl. snippet stops underneath)
//! 4. Phantom-text backgrounds, then text content
//! 5. Cursor carets (focused + window-focused)
//! 5.25. Ghost text (bracket auto-close / completion preview)
//! 5.4.  Section-marker rules
//! 5.5.  Inline diagnostic bubbles + their quick-fix popovers
//! 5.6.  Caret-anchored completion popup
//! 6.    Sticky gutter (line numbers, diagnostic markers, breakpoints)
//! 7.    Scrollbars
//!
//! Any layer painted later sits on top of layers below it. The gutter
//! deliberately paints *after* text so it covers anything that
//! scrolls under it horizontally.

use masonry::core::{PaintCtx, PropertiesRef};
use masonry::imaging::Painter;
use masonry::peniko::Color;
use masonry::properties::{CaretColor, ContentColor, SelectionColor};

use super::CodeEditorWidget;
use crate::gutter::{
    BreakpointsControl, DiagnosticMarkersControl, ExecutionLineControl, GutterPaintContext,
    LineNumbersControl, VcsChangesControl,
};
use crate::text_layout::ScreenLines;

impl CodeEditorWidget {
    pub(super) fn paint_impl(
        &mut self,
        ctx: &mut PaintCtx<'_>,
        props: &PropertiesRef<'_>,
        painter: &mut Painter<'_>,
    ) {
        // =========================================================================
        // INTERNAL SCROLL MANAGEMENT
        // =========================================================================
        // We handle scrolling internally (not via Portal). This ensures gutter and
        // content are painted with the SAME scroll position in ONE paint call.
        // Scroll position is updated by on_pointer_event handling PointerEvent::Scroll.
        //
        // Scroll handling adapted from xilem Portal widget:
        // https://github.com/linebender/xilem/blob/05db616/masonry/src/widgets/portal.rs
        // (see on_pointer_event for PointerScrollEvent handling)

        let scroll_x = self.scroll.pos().x;
        let scroll_y = self.scroll.pos().y;
        let viewport_height = self.scroll.viewport_size().height.max(1000.0);

        // Soft wrap: cap each line's layout width at the visible content
        // width (viewport minus gutter); without it lines run on and the
        // editor scrolls horizontally.
        let wrap_width: Option<f32> = self.config.word_wrap.then(|| {
            (self.scroll.viewport_size().width - self.config.gutter_width).max(50.0) as f32
        });

        // Compute ScreenLines for the visible region, lazily building Parley
        // layouts only for lines that are actually on screen.
        let (fctx, lctx) = ctx.text_contexts();
        let syntax_palette = self.theme.as_ref().map(|t| &t.syntax);
        let screen = if let Some(palette) = syntax_palette {
            ScreenLines::compute(
                &mut self.lines,
                self.document.as_ref(),
                fctx,
                lctx,
                palette,
                scroll_y,
                viewport_height,
                wrap_width,
            )
        } else {
            // No theme set - return empty screen (shouldn't happen in practice)
            ScreenLines::empty()
        };
        // Store for use in painting helpers
        self.screen_lines = Some(screen);

        // Check if any newly-built visible line layout exceeds the width we reported
        // in layout(). If so, flag a re-layout so Portal learns the true content width.
        // Only checks visible lines (O(screen_height/line_height), not O(total_lines)).
        if let Some(ref screen) = self.screen_lines {
            for sl in &screen.lines {
                if let Some(ll) = self.lines.get_layout(sl.line) {
                    let w = ll.layout.width() as f64;
                    if w > self.viewport.max_content_width + 1.0 {
                        self.viewport.needs_width_update = true;
                        break;
                    }
                }
            }
        }

        // After computing layouts for visible lines (and the buffer zone),
        // the total document height may have changed (estimated → actual).
        // If so, request a re-layout so Portal updates its scrollable area.
        let new_total = (self.lines.total_height() * 10.0).round() / 10.0;
        let old_total = (self.viewport.last_total_height * 10.0).round() / 10.0;
        if (new_total - old_total).abs() > 0.5 {
            self.viewport.last_total_height = self.lines.total_height();
            self.viewport.needs_width_update = true; // piggyback on existing re-layout flag
        }

        let screen = self.screen_lines.as_ref().unwrap();

        let gutter_w = self.config.gutter_width;
        let viewport_width = ctx.paint_box().size().width;

        // Resolve theme colors
        let gutter_bg = self
            .theme
            .as_ref()
            .map_or(masonry::peniko::Color::from_rgb8(37, 37, 38), |t| {
                t.gutter_bg
            });

        // 1. Column guide (vertical line at character 80) - painted first so it's behind text
        if let Some(theme) = &self.theme {
            let char_width = self.lines.char_width();
            crate::paint::paint_column_guide(
                painter,
                gutter_w,
                char_width,
                80, // TEXT_COLUMN_WIDTH
                viewport_height,
                scroll_x,
                theme.column_guide,
            );
        }

        // 2. Active line highlight (when focused)
        if ctx.is_focus_target() {
            if let Some(theme) = &self.theme {
                crate::paint::paint_active_line(
                    painter,
                    &self.cursor,
                    self.document.as_ref(),
                    &self.lines,
                    screen,
                    theme,
                    viewport_width,
                );
            }
        }

        // Extract property values upfront (property_cache borrows ctx mutably)
        let selection_color = {
            let cache = ctx.property_cache();
            props.get::<SelectionColor>(cache).color
        };
        let text_content_color = {
            let cache = ctx.property_cache();
            props.get::<ContentColor>(cache).color
        };
        let caret_color_val = {
            let cache = ctx.property_cache();
            props.get::<CaretColor>(cache).color
        };

        // 3. Selection rectangles
        // Paint selections even when not focused (for search result highlighting)
        // Use bright search_highlight color when showing search results
        let sel_color = if self.is_search_highlight {
            // Search result: use bright amber/orange color from theme
            self.theme
                .as_ref()
                .map(|t| t.search_highlight)
                .unwrap_or(Color::from_rgba8(255, 165, 50, 180))
        } else if ctx.is_focus_target() && ctx.is_window_focused() {
            selection_color
        } else if ctx.is_focus_target() {
            selection_color.multiply_alpha(0.5)
        } else {
            // Not focused, not search - dimmer selection
            selection_color.multiply_alpha(0.4)
        };
        // Paint snippet stops *before* the selection so the
        // selection rectangle (which mirrors the active stop's
        // range) sits cleanly on top.
        if let Some(session) = self.snippet_session.as_ref() {
            let stops_ranges: Vec<std::ops::Range<usize>> =
                session.stops.iter().map(|s| s.range.clone()).collect();
            // Subtle theme accents — use the existing column-selection
            // background tint for inactive stops (already a muted
            // cool-grey/indigo) and a brighter accent for the active
            // one. Falls back to the cursor color (multiplied) if the
            // theme is missing.
            let inactive = self
                .theme
                .as_ref()
                .map(|t| t.column_selection_bg)
                .unwrap_or(masonry::peniko::Color::from_rgba8(80, 100, 140, 60));
            let active = self
                .theme
                .as_ref()
                .map(|t| t.cursor.multiply_alpha(0.30))
                .unwrap_or(masonry::peniko::Color::from_rgba8(120, 170, 220, 90));
            crate::paint::paint_snippet_stops(
                painter,
                self.document.as_ref(),
                &self.lines,
                screen,
                &stops_ranges,
                session.current_idx,
                inactive,
                active,
                gutter_w,
                scroll_x,
            );
        }

        crate::paint::paint_selections(
            painter,
            &self.cursor,
            self.document.as_ref(),
            &self.lines,
            screen,
            sel_color,
            gutter_w,
            scroll_x,
        );

        // 4. Text content (only visible lines)
        let brushes = if self.brushes.is_empty() {
            vec![text_content_color.into()]
        } else {
            self.brushes.clone()
        };
        crate::paint::paint_phantom_backgrounds(painter, &self.lines, screen, gutter_w, scroll_x);
        crate::paint::paint_text(
            painter,
            &self.lines,
            screen,
            &brushes,
            gutter_w,
            scroll_x,
            self.config.hint,
        );

        // 5. Cursor carets (when focused and window focused)
        if ctx.is_focus_target() && ctx.is_window_focused() {
            let caret_color = caret_color_val;
            crate::paint::paint_cursors(
                painter,
                &self.cursor,
                self.document.as_ref(),
                &self.lines,
                screen,
                caret_color,
                gutter_w,
                scroll_x,
                self.animator.is_visible(),
            );
        }

        // 5.25. Ghost text (bracket auto-close or completion preview)
        // Bracket ghost takes priority over completion ghost
        let ghost_to_paint = self.completion.ghost_to_paint();
        if let Some(ref ghost_text) = ghost_to_paint {
            crate::paint::paint_ghost_text(
                ctx,
                painter,
                &self.cursor,
                self.document.as_ref(),
                &self.lines,
                screen,
                ghost_text,
                gutter_w,
                scroll_x,
            );
        }

        // 5.4. Section markers (horizontal rule + bold name).
        // Painted after text so the rule sits visually behind cursor /
        // selections / diagnostic bubbles but on top of the text the
        // marker comment is replacing.
        if let Some(theme) = &self.theme {
            crate::paint::paint_section_markers(
                ctx,
                painter,
                screen,
                &self.lines,
                &self.section_markers,
                theme,
                self.scroll.viewport_size().width,
                gutter_w,
                scroll_x,
            );
        }

        // 5.5. Inline diagnostic bubbles (Xcode-style).
        // Paint after text/cursors so bubbles appear on top, but before
        // gutter so gutter can cover bubbles that scroll under it. The
        // painter calls back into `bubble_actions_for_line` to look up
        // the per-line quick-fix list — the cache lives outside the
        // widget (on AppModel) and is fed in via the
        // `bubble_code_actions_for_line` accessor below.
        let actions_lookup = self.diagnostics.code_actions_view();
        let result = crate::paint::paint_inline_diagnostic_bubbles(
            ctx,
            painter,
            screen,
            &self.lines,
            self.diagnostics.inline_diagnostics(),
            gutter_w,
            scroll_x,
            self.diagnostics.expanded_line(),
            self.diagnostics.selected_action_index(),
            |line: usize| {
                actions_lookup
                    .get(&line)
                    .map(|v| v.as_slice())
                    .unwrap_or(&[])
            },
        );
        self.diagnostics
            .set_painted_bounds(result.bubble_bounds, result.action_bounds);

        // 5.6. Caret-anchored completion popup. Paints AFTER bubbles
        // so the popup overlaps a diagnostic bubble below the caret;
        // gutter still paints last and covers anything scrolling under
        // it. Skipped when no items or popup hidden.
        self.completion.clear_item_bounds();
        // Block-mode branch: a single full-preview code-action (e.g.
        // "Fill match arms") gets a multi-line preview at the start
        // of the line, with a slightly green-tinted background. The
        // strict-mode rule on the AppModel side guarantees the
        // regular list popup is suppressed whenever this is set, so
        // we render it here in place of (not next to) the list.
        if self.completion.should_paint_block_popup() {
            let caret_offset = self.cursor.offset();
            let caret_line = self.document.line_of_offset(caret_offset);
            if let Some(sl) = screen.lines.iter().find(|sl| sl.line == caret_line) {
                let line_height = self
                    .lines
                    .get_layout(caret_line)
                    .map(|l| l.line_height)
                    .unwrap_or(self.lines.line_height());
                let theme = self.theme.as_ref().cloned().unwrap_or_default();
                let (title, preview, source_idx, source_total) = self
                    .completion
                    .block_action()
                    .expect("checked is_some above")
                    .clone();
                let popup_result = crate::paint::paint_completion_block_popup(
                    ctx,
                    painter,
                    &title,
                    &preview,
                    source_idx,
                    source_total,
                    sl.y,
                    line_height,
                    gutter_w,
                    self.scroll.viewport_size().width,
                    self.scroll.viewport_size().height,
                    theme.gutter_bg,
                    theme.line_number,
                    theme.default_text(),
                    theme.line_number,
                );
                self.completion.set_item_bounds(popup_result.item_bounds);
            }
        } else if self.completion.should_paint_list_popup() {
            // Convert caret offset → viewport-local (x, y) by going
            // through screen lines. If the caret line isn't visible
            // (caret scrolled off-screen) we don't bother painting.
            let caret_offset = self.cursor.offset();
            let caret_line = self.document.line_of_offset(caret_offset);
            if let Some(sl) = screen.lines.iter().find(|sl| sl.line == caret_line) {
                let line_height = self
                    .lines
                    .get_layout(caret_line)
                    .map(|l| l.line_height)
                    .unwrap_or(self.lines.line_height());
                let caret_x_doc = self.lines.offset_to_x(self.document.as_ref(), caret_offset);
                let caret_x = gutter_w + caret_x_doc - scroll_x;
                let caret_y = sl.y;

                let theme = self.theme.as_ref().cloned().unwrap_or_default();
                let popup_result = crate::paint::paint_completion_popup(
                    ctx,
                    painter,
                    self.completion.items(),
                    self.completion.selected(),
                    caret_x,
                    caret_y,
                    line_height,
                    self.scroll.viewport_size().width,
                    self.scroll.viewport_size().height,
                    theme.gutter_bg,
                    theme.line_number,
                    theme.default_text(),
                    theme.line_number,
                    theme.active_line_bg,
                );
                self.completion.set_item_bounds(popup_result.item_bounds);
            }
        }

        // 5b. Quick Help balloon (explicit request) — or, when none is open,
        // the pointer-hover popup. Both above the completion popup.
        if let Some(qh_text) = self.quick_help.text().map(str::to_string) {
            let caret = self.cursor.offset();
            let scroll = self.quick_help.scroll();
            if let Some(result) = self.paint_markdown_balloon(ctx, painter, &qh_text, caret, scroll)
            {
                self.quick_help
                    .set_painted_geometry(result.balloon_rect, result.content_height);
            }
        } else {
            let hover = self
                .hover
                .visible()
                .map(|(anchor, text)| (anchor, text.to_string()));
            let rect = hover
                .and_then(|(anchor, text)| {
                    self.paint_markdown_balloon(ctx, painter, &text, anchor, 0.0)
                })
                .map_or(masonry::kurbo::Rect::ZERO, |result| result.balloon_rect);
            self.hover.set_painted_rect(rect);
        }

        // 6. Sticky gutter - painted AFTER text content so it covers scrolled text
        // The gutter stays fixed on the left side even when scrolling horizontally.
        if self.config.show_line_numbers {
            // Debug: log scroll values when actively scrolling
            if scroll_x > 1.0 || scroll_y > 1.0 {
                tracing::debug!(
                    "SCROLL: x={:.1}, y={:.1}, viewport={:?}",
                    scroll_x,
                    scroll_y,
                    self.scroll.viewport_size()
                );
            }

            // 6a. Gutter background (painted over any text that scrolled under it)
            // Uses scroll_x offset to counteract Portal's horizontal transform
            crate::paint::paint_gutter(
                painter,
                gutter_w,
                gutter_bg,
                scroll_x,
                scroll_y,
                viewport_height,
            );

            // 6b-d. Update gutter controls with current state, then paint via registry
            let line_count = self.document.line_count();
            let line_num_color_inner = self
                .theme
                .as_ref()
                .map_or(masonry::peniko::Color::from_rgb8(150, 150, 150), |t| {
                    t.line_number
                });

            // Update controls with current state
            if let Some(line_nums) = self
                .gutter_registry
                .get_mut::<LineNumbersControl>("line_numbers")
            {
                line_nums.set_line_count(line_count);
                line_nums.set_color(line_num_color_inner);
            }
            if let Some(diag) = self
                .gutter_registry
                .get_mut::<DiagnosticMarkersControl>("diagnostic_markers")
            {
                diag.set_markers(self.diagnostics.markers().to_vec());
            }
            if let Some(bp) = self
                .gutter_registry
                .get_mut::<BreakpointsControl>("breakpoints")
            {
                bp.set_breakpoints(self.breakpoints.clone());
            }
            if let Some(exec) = self
                .gutter_registry
                .get_mut::<ExecutionLineControl>("execution_line")
            {
                exec.set_line(self.execution_line);
            }
            if let Some(vcs) = self
                .gutter_registry
                .get_mut::<VcsChangesControl>("vcs_changes")
            {
                if self.vcs.is_enabled() {
                    match self.vcs.diff() {
                        Some(diff) => vcs.set_diff(diff),
                        None => vcs.clear(),
                    }
                    vcs.set_hovered(self.vcs.hovered_deletion());
                    vcs.set_enabled(true);
                } else {
                    vcs.set_enabled(false);
                }
            }

            // Paint all gutter controls via registry
            let mut gctx = GutterPaintContext {
                ctx,
                painter,
                screen,
                lines: &self.lines,
                gutter_width: gutter_w,
                scroll_x,
                font_size: self.lines.font_size(),
                height: viewport_height,
            };
            self.gutter_registry.paint_all(&mut gctx, &brushes);
        }

        // 6e. Popup-focus glyph (`⏎`) — painted on the caret
        // line when the popup is visible AND the user has
        // navigated it with Up/Down at least once. Signals
        // that Return is armed to accept (otherwise Return
        // inserts a newline). Spec: popup_mode/mod.rs.
        if self.config.show_line_numbers
            && self.completion.is_visible()
            && self.completion.popup_user_navigated()
        {
            let theme = self.theme.as_ref().cloned().unwrap_or_default();
            let caret_offset = self.cursor.offset();
            let caret_line = self.document.line_of_offset(caret_offset);
            crate::paint::paint_popup_focus_glyph(
                ctx,
                painter,
                screen,
                &self.lines,
                caret_line,
                gutter_w,
                scroll_x,
                theme.line_number,
            );
        }

        // 7. Scroll bars - painted last so they appear on top
        if let Some(theme) = &self.theme {
            crate::paint::paint_scrollbars(
                painter,
                (
                    self.scroll.viewport_size().width,
                    self.scroll.viewport_size().height,
                ),
                (
                    self.scroll.content_size().width,
                    self.scroll.content_size().height,
                ),
                (scroll_x, scroll_y),
                theme.scrollbar_track,
                theme.scrollbar_thumb,
            );
        }

        // 8. VCS removed-text balloon — floats on top of everything else.
        self.vcs
            .set_painted_geometry(masonry::kurbo::Rect::ZERO, masonry::kurbo::Rect::ZERO);
        if self.vcs.is_enabled() {
            if let Some((after, removed)) = self.vcs.balloon().cloned() {
                // Boundary y of the deletion marker: top of line `after`, or the
                // bottom of the preceding line for an end-of-file deletion.
                let anchor_y = screen
                    .lines
                    .iter()
                    .find(|sl| sl.line == after)
                    .map(|sl| sl.y)
                    .or_else(|| {
                        screen
                            .lines
                            .iter()
                            .find(|sl| sl.line + 1 == after)
                            .map(|sl| {
                                let h = self
                                    .lines
                                    .get_layout(sl.line)
                                    .map(|l| l.line_height)
                                    .unwrap_or(self.lines.line_height());
                                sl.y + h
                            })
                    });
                if let Some(anchor_y) = anchor_y {
                    let theme = self.theme.as_ref().cloned().unwrap_or_default();
                    let painted = crate::paint::paint_vcs_deletion_balloon(
                        ctx,
                        painter,
                        &removed,
                        crate::paint::VcsBalloonAnchor {
                            x: gutter_w,
                            y: anchor_y,
                            viewport_width: self.scroll.viewport_size().width,
                            viewport_height: self.scroll.viewport_size().height,
                        },
                        crate::paint::VcsBalloonColors {
                            bg: theme.gutter_bg,
                            border: theme.line_number,
                            text: theme.default_text(),
                            accent_bg: theme.active_line_bg,
                        },
                    );
                    self.vcs
                        .set_painted_geometry(painted.balloon_rect, painted.insert_rect);
                }
            }
        }
    }
}
