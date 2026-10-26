// Copyright 2026 the xilem_code_editor developers
// SPDX-License-Identifier: Apache-2.0

//! Pointer hover popup (zed timing, see `code_editor_core::timing`): the
//! widget tracks the word under the pointer and asks the host about it
//! with `HoverAt`; the host answers through `set_hover`.

use std::time::Instant;

use code_editor_core::cursor::is_bracket;
use code_editor_core::cursor::word_helpers::find_word_start;
use code_editor_core::document::{offset_of_position, position_of_offset};
use code_editor_core::model::HoverInfo;
use code_editor_core::timing::HoverEffect;
use masonry::core::keyboard::{Key, KeyState, NamedKey};
use masonry::core::{EventCtx, PaintCtx, PointerEvent, TextEvent, UpdateCtx, WidgetMut};
use masonry::imaging::Painter;
use masonry::kurbo::Point;

use super::CodeEditorWidget;
use crate::action::CodeEditorAction;
use crate::popup::{CodeBlockHighlighter, QuickHelpPaintResult};

impl CodeEditorWidget {
    /// Feed pointer events to the hover timer.
    pub(super) fn hover_pointer_event(&mut self, ctx: &mut EventCtx<'_>, event: &PointerEvent) {
        let now = Instant::now();
        let effect = match event {
            PointerEvent::Move(update) if update.current.buttons.is_empty() => {
                let pos = ctx.local_position(update.current.position);
                let anchor = self.hover_anchor_at(pos);
                self.hover.pointer_moved(anchor, pos, now)
            }
            PointerEvent::Leave(..) => {
                self.hover
                    .pointer_moved(None, Point::new(f64::MAX, f64::MAX), now)
            }
            PointerEvent::Down(..) | PointerEvent::Scroll(..) => {
                self.hover.dismiss().then_some(HoverEffect::Hide)
            }
            _ => None,
        };
        self.handle_hover_effect(effect, |action| {
            ctx.submit_action::<CodeEditorAction>(action)
        });
        if effect == Some(HoverEffect::Hide) {
            ctx.request_render();
        }
        if self.hover.next_deadline().is_some() {
            ctx.request_anim_frame();
        }
    }

    /// Typing hides the popup (modifier keys alone don't).
    pub(super) fn hover_text_event(&mut self, ctx: &mut EventCtx<'_>, event: &TextEvent) {
        let typed = match event {
            TextEvent::Keyboard(key) => key.state == KeyState::Down && !is_modifier(&key.key),
            TextEvent::Ime(_) | TextEvent::ClipboardPaste(_) => true,
            _ => false,
        };
        if typed && self.hover.dismiss() {
            ctx.request_render();
        }
    }

    /// Advance the hover timer on an animation frame.
    pub(super) fn hover_anim_frame(&mut self, ctx: &mut UpdateCtx<'_>) {
        let Some(deadline) = self.hover.next_deadline() else {
            return;
        };
        let now = Instant::now();
        if now >= deadline {
            let was_visible = self.hover.visible().is_some();
            let effect = self.hover.tick(now);
            self.handle_hover_effect(effect, |action| {
                ctx.submit_action::<CodeEditorAction>(action)
            });
            if was_visible != self.hover.visible().is_some() {
                ctx.request_render();
            }
        }
        if self.hover.next_deadline().is_some() {
            ctx.request_anim_frame();
        }
    }

    /// Focus loss hides the popup.
    pub(super) fn hover_focus_lost(&mut self, ctx: &mut UpdateCtx<'_>) {
        if self.hover.dismiss() {
            ctx.request_render();
        }
    }

    /// The Markdown balloon (quick help and hover) below or above the text
    /// at `anchor`. `None` when that line is not on screen.
    pub(super) fn paint_markdown_balloon(
        &self,
        ctx: &mut PaintCtx<'_>,
        painter: &mut Painter<'_>,
        text: &str,
        anchor: usize,
        scroll_offset: f64,
    ) -> Option<QuickHelpPaintResult> {
        let line = self.document.line_of_offset(anchor);
        let sl = self
            .screen_lines
            .as_ref()?
            .lines
            .iter()
            .find(|sl| sl.line == line)?;
        let caret = self.lines.caret_geometry(self.document.as_ref(), anchor);
        let x = self.config.gutter_width + caret.x - self.scroll.pos().x;
        let theme = self.theme.as_ref().cloned().unwrap_or_default();
        Some(crate::paint::paint_quick_help_balloon(
            ctx,
            painter,
            text,
            x,
            sl.y + caret.y,
            caret.height,
            self.scroll.viewport_size().width,
            self.scroll.viewport_size().height,
            scroll_offset,
            theme.gutter_bg,
            theme.active_line_bg,
            theme.line_number,
            theme.default_text(),
            &theme.syntax,
            CodeBlockHighlighter {
                provider: self.highlighter.as_deref(),
                language: &self.language,
            },
        ))
    }

    /// The host's answer to a `HoverAt` request.
    pub fn set_hover(this: &mut WidgetMut<'_, Self>, info: Option<HoverInfo>) {
        let Some(info) = info else { return };
        let doc = this.widget.document.as_ref();
        let anchor = find_word_start(doc, offset_of_position(doc, info.line, info.col));
        this.widget
            .hover
            .set_content(anchor, info.markdown, Instant::now());
        this.ctx.request_anim_frame();
        this.ctx.request_render();
    }

    /// Start of the word under `pos`, or `None` over the gutter, empty
    /// space, punctuation or past the end of a line.
    fn hover_anchor_at(&self, pos: Point) -> Option<usize> {
        if pos.x < self.config.gutter_width {
            return None;
        }
        let text_x = pos.x - self.config.gutter_width + self.scroll.pos().x;
        let doc_y = pos.y + self.scroll.pos().y;
        if doc_y >= self.lines.total_height() {
            return None;
        }
        let doc = self.document.as_ref();
        let offset = self.lines.offset_at_point(doc, text_x, doc_y);
        // offset_at_point clamps to the line end: reject points beyond it.
        let caret = self.lines.caret_geometry(doc, offset);
        if text_x > caret.x + self.lines.char_width() {
            return None;
        }
        let ch = doc
            .text()
            .get_char(doc.text().byte_to_char(offset.min(doc.len_bytes())))?;
        let is_word = ch.is_alphanumeric() || ch == '_';
        (is_word && !is_bracket(ch)).then(|| find_word_start(doc, offset))
    }

    fn handle_hover_effect(
        &self,
        effect: Option<HoverEffect>,
        mut submit: impl FnMut(CodeEditorAction),
    ) {
        if let Some(HoverEffect::Request(anchor)) = effect {
            let (line, col) = position_of_offset(self.document.as_ref(), anchor);
            submit(CodeEditorAction::HoverAt { line, col });
        }
    }
}

fn is_modifier(key: &Key) -> bool {
    matches!(
        key,
        Key::Named(NamedKey::Shift | NamedKey::Control | NamedKey::Alt | NamedKey::Meta)
    )
}
