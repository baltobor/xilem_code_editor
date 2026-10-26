//! CodeEditorWidget — the Masonry Widget that orchestrates the code editor.
//!
//! This is a thin orchestrator that owns all editor state and delegates to:
//! - `input.rs` for event handling
//! - `painting.rs` for rendering
//! - `lines.rs` for layout management
//! - `cursor.rs` for cursor/selection state
//! - `document.rs` for text storage
//! - `widget_mut.rs` for external mutation API (WidgetMut methods)
//!
//! The widget itself only handles Masonry lifecycle (layout, paint, events)
//! and coordinates between these subsystems.

use std::any::TypeId;
use std::sync::Arc;

use masonry::accesskit::{Node, Role};
use masonry::core::{
    AccessCtx, AccessEvent, ChildrenIds, CursorIcon, EventCtx, LayoutCtx, MeasureCtx, PaintCtx,
    PointerEvent, PropertiesMut, PropertiesRef, QueryCtx, RegisterCtx, TextEvent, Update,
    UpdateCtx, Widget, WidgetId,
};
use masonry::imaging::Painter;
use masonry::kurbo::{Axis, Point, Size};
use masonry::layout::{LenReq, Length};
use masonry::peniko::Brush;
use masonry::properties::{CaretColor, ContentColor, SelectionColor};
use tracing::{Span, trace_span};

use crate::action::CodeEditorAction;
use crate::gutter::{
    BreakpointsControl, DiagnosticMarkersControl, ExecutionLineControl, GutterRegistry,
    LineNumbersControl, VcsChangesControl,
};
use crate::properties::EditorTheme;
use crate::text_layout::Lines;
use crate::text_layout::ScreenLines;
use crate::text_layout::ViewportState;
use code_editor_core::config::EditorConfig;
use code_editor_core::cursor::Cursor;
use code_editor_core::document::Document;
use code_editor_core::provider::{HighlightPostPass, HighlightProvider};
use code_editor_core::timing::CursorAnimator;

mod paint_impl;
mod pointer_impl;
mod text_event_impl;

// MARK: Renderer invariants
//
// Four overlay systems sit on top of the rope:
//
//   1. `bracket_ghost: Option<char>` — single-char gray ghost
//      after the caret (e.g. `)` after the user types `(`).
//   2. `completion_ghost_text: Option<String>` — multi-char
//      gray preview of the marked completion item.
//   3. Phantom text via `Lines::set_phantoms` — gray inline
//      annotations inside a line (e.g. inlay hints).
//   4. Diagnostic / completion popup overlays painted by
//      `paint_inline_diagnostic_bubbles` and the popup painter.
//
// **Invariant: none of these overlays write to the rope.** A
// user gesture (Tab / Return / click) is required to commit
// any of them. Specifically:
//
//   - `cursor.insert(doc, …)` runs exactly three times in
//     `text_event_impl.rs` — all three gated by an explicit
//     `result.accept_*_ghost*` flag the input handler set in
//     response to a real keystroke.
//   - The widget_mut paths (`perform_text_edit`,
//     `perform_workspace_edits`, `perform_insert`) accept
//     text from the LSP / quick-fix flow, not from the
//     overlay systems.
//   - `selected_text` reads only the rope. The clipboard
//     gets only what the rope holds.
//
// If you find an overlay value in the rope, the bug is in the
// LSP / paste path, not in the overlay code.

// MARK: CodeEditorWidget — state fields

/// A production-quality code editor widget with:
/// - Viewport virtualization (only lays out visible lines)
/// - Multi-cursor support
/// - Column/block selection
/// - Inline diagnostic overlays
/// - Syntax highlighting
///
/// Built on Parley's raw Layout API + ropey::Rope. Does NOT use PlainEditor.
#[allow(dead_code)]
pub struct CodeEditorWidget {
    /// The text buffer — a rope for efficient large-file editing.
    pub(super) document: Box<dyn Document>,
    /// Cursor and selection state (supports multiple cursors).
    pub(super) cursor: Cursor,
    /// Line layout cache and coordinate conversion engine.
    pub(super) lines: Lines,
    /// Currently visible lines (computed each frame during layout).
    pub(super) screen_lines: Option<ScreenLines>,

    /// Complete editor theme (all colors).
    pub(super) theme: Option<EditorTheme>,
    /// Brush palette for render_text (index 0 = default, 1+ = syntax categories).
    pub(super) brushes: Vec<Brush>,

    /// Editor configuration (gutter, tabs, etc.) — SOLID extracted.
    pub(super) config: EditorConfig,

    /// Viewport synchronization state — SOLID extracted.
    pub(super) viewport: ViewportState,

    /// Cursor blink animation — SOLID extracted.
    pub(super) animator: CursorAnimator,

    /// Syntax highlighting source; `None` = no highlighting.
    pub(super) highlighter: Option<Arc<dyn HighlightProvider>>,
    /// App-specific adjustments applied after `highlighter`.
    pub(super) highlight_passes: Vec<Arc<dyn HighlightPostPass>>,
    /// Language passed to the highlighter (file extension or id).
    pub(super) language: String,

    /// Breakpoint lines (1-based).
    pub(super) breakpoints: Vec<usize>,

    /// Current debugger-paused execution line (1-based) for this file. None
    /// when the debugger is not paused on this file (or not paused at all).
    pub(super) execution_line: Option<usize>,

    /// Inline diagnostic bubble state: markers, diagnostic list,
    /// quick-fix cache, and expanded/selected bubble tracking.
    pub(super) diagnostics: crate::state::DiagnosticBubbleState,

    /// Section markers parsed from the document text. Recomputed in
    /// `rehighlight()` whenever the text changes; rendered as
    /// horizontal rules + bold names by `paint_section_markers`.
    pub(super) section_markers: Vec<code_editor_core::model::SectionMarker>,
    /// Whether `section_markers` differs from what we last emitted to
    /// the View layer. Set by `rehighlight()` when it produces a new
    /// list, consumed by `update()` which fires
    /// `CodeEditorAction::SectionMarkersChanged`. We don't emit
    /// inside `rehighlight` directly because that path doesn't have
    /// a `ctx`.
    pub(super) section_markers_dirty: bool,

    /// Extensible gutter control registry.
    /// Contains line numbers, diagnostic markers, and breakpoints controls.
    pub(super) gutter_registry: GutterRegistry,

    // =========================================================================
    // INTERNAL SCROLL MANAGEMENT (derived from Portal)
    // =========================================================================
    // The editor handles its own scrolling instead of being wrapped in Portal.
    // This ensures gutter and content are painted with the SAME scroll position
    // in the SAME paint call - no sync issues, no frame delays.
    /// Scroll/viewport geometry: current scroll position, cached
    /// viewport + content sizes, and an in-progress scrollbar drag.
    pub(super) scroll: crate::scroll::ScrollState,

    // =========================================================================
    // GHOST TEXT FOR AUTOCOMPLETE AND BRACKETS
    // =========================================================================
    /// Completion feature state: bracket/completion ghost text, popup
    /// list + block-mode payload, visibility, and navigation flags.
    /// The gutter `⏎` (U+23CE) glyph and the Enter-key accept-vs-
    /// newline contract both key off `popup_user_navigated()`.
    pub(super) completion: crate::state::CompletionState,

    /// Quick Help balloon state (context-menu hover documentation popup).
    pub(super) quick_help: crate::state::QuickHelpState,

    /// Pointer-hover popup (zed timing).
    pub(super) hover: crate::state::HoverState,

    /// VCS gutter state: live diff, enabled flag, hovered marker, and
    /// the removed-text balloon. Owned here (not just on the gutter
    /// control) so the pointer handler can hit-test deletion markers
    /// and read their removed text.
    pub(super) vcs: crate::state::VcsGutterState,

    /// Bracket auto-closing behavior mode (Ghost vs AutoClose).
    pub(super) bracket_mode: code_editor_core::model::BracketMode,

    /// Whether current selection is from a search operation.
    /// When true, uses search_highlight color instead of selection color.
    pub(super) is_search_highlight: bool,
    /// Requested alignment for the next viewport scroll triggered by
    /// `request_scroll_to_cursor`. Set by external navigation (Structure
    /// outline click → Top, Goto Definition / search → Center). Reset to
    /// `Edge` after the scroll happens so subsequent typing/arrow scrolls
    /// don't keep recentering.
    /// Marker: Structure Click Appearance: Click shows the function at the
    /// top of the text editor.
    pub(super) pending_scroll_alignment: code_editor_core::model::GotoAlignment,

    /// Active multi-stop snippet session. Set right after a snippet
    /// completion is inserted; tracks placeholder ranges live as the
    /// user edits them. Tab / Shift+Tab walk through stops; typing
    /// inside the active stop grows/shrinks it; an edit outside any
    /// stop, Esc, or Return at the final stop ends the session.
    pub(super) snippet_session: Option<code_editor_core::snippet::SnippetSession>,

    /// Snapshot of the document text taken right after an internal
    /// edit-group operation (`perform_workspace_edits` /
    /// `perform_text_edit`). Used by `set_text` to absorb stale
    /// rebuilds: when the wrapper view re-runs with a `tab.content`
    /// that hasn't yet caught up to the post-edit text, set_text
    /// would otherwise wipe the document AND the just-recorded undo
    /// group. The snapshot lets it recognise the in-flight gap and
    /// skip the rollback.
    ///
    /// Lifecycle:
    /// - Set by `perform_workspace_edits` / `perform_text_edit` to
    ///   the document's text immediately after the edit lands.
    /// - Cleared by `set_text` whenever the wrapper has caught up
    ///   (`incoming text == widget.text_string()`).
    /// - Skipped (no rollback, no history clear) when the wrapper is
    ///   still stale (`widget.text_string() == snapshot &&
    ///   incoming != widget.text_string()`).
    pub(super) pending_workspace_edit_text: Option<String>,
}

mod builders;
mod hover;
mod viewport_scroll;
mod widget_mut;

// MARK: Inherent impl — accessors, rehighlight, gutter registry
impl CodeEditorWidget {
    // MARK: Document + scroll accessors

    /// Get the current text as an owned String.
    pub fn text_string(&self) -> String {
        self.document.to_string()
    }

    /// The primary selection as a `SelectionChanged` action.
    pub(crate) fn selection_changed_action(&self) -> CodeEditorAction {
        let selection = self.cursor.selection.primary();
        CodeEditorAction::SelectionChanged {
            start: selection.start.min(selection.end),
            end: selection.start.max(selection.end),
        }
    }

    /// Get the current vertical scroll position.
    pub fn current_scroll_y(&self) -> f64 {
        self.scroll.pos().y
    }

    /// Get the current horizontal scroll position.
    pub fn current_scroll_x(&self) -> f64 {
        self.scroll.pos().x
    }

    /// Run syntax highlighting on the current document and push results to Lines.
    pub(super) fn rehighlight(&mut self) {
        use code_editor_core::highlight::{HighlightCategory, HighlightSpan, split_into_lines};

        let source = self.document.to_string();
        let mut flat = self
            .highlighter
            .as_ref()
            .map(|h| h.highlight(&source, &self.language))
            .unwrap_or_default();
        for pass in &self.highlight_passes {
            pass.apply(&source, &self.language, &mut flat);
        }
        let mut spans = split_into_lines(&source, &flat);

        // Section markers ride the same trigger as syntax highlighting.
        let new_markers = code_editor_core::model::parse_section_markers(&source);
        if new_markers != self.section_markers {
            self.section_markers = new_markers;
            // Emitted from update(), which has a ctx (rehighlight may not).
            self.section_markers_dirty = true;
        }

        // Paint each marker's name in the `Type` color on top of the
        // comment color (parley: later spans win), so section names read
        // as accented headings inside a faded comment line.
        for marker in &self.section_markers {
            let Some(range) = marker.name_range_in_line.clone() else {
                continue;
            };
            if let Some(line) = spans.get_mut(marker.line) {
                line.push(HighlightSpan {
                    byte_range: range,
                    category: HighlightCategory::Type,
                });
            }
        }

        self.lines.set_highlights(spans);
        self.gutter_registry
            .document_changed(self.document.as_ref());
    }

    // MARK: Gutter registry

    /// Create the default gutter registry with line numbers, diagnostics, and breakpoints.
    fn create_default_gutter_registry(line_count: usize) -> GutterRegistry {
        let mut registry = GutterRegistry::new();

        // Line numbers control (z-order 0)
        let line_nums = LineNumbersControl::new(
            masonry::peniko::Color::from_rgb8(150, 150, 150), // default color, updated in paint
            line_count,
        );
        registry.register(Box::new(line_nums));

        // VCS change markers (z-order 5) — green added/changed bars + beige
        // removed-line markers. Disabled until the widget feeds it a diff.
        registry.register(Box::new(VcsChangesControl::new()));

        // Diagnostic markers control (z-order 10)
        let diag_markers = DiagnosticMarkersControl::new();
        registry.register(Box::new(diag_markers));

        // Breakpoints control (z-order 20)
        let breakpoints = BreakpointsControl::new();
        registry.register(Box::new(breakpoints));

        // Execution-line marker (z-order 25 — drawn above the breakpoint dot
        // so the arrow is visible when paused on a breakpointed line).
        let exec_line = ExecutionLineControl::new();
        registry.register(Box::new(exec_line));

        registry
    }
}

// --- MARK: IMPL WIDGET ---
impl Widget for CodeEditorWidget {
    type Action = CodeEditorAction;

    // MARK: Widget — animation frame

    fn on_anim_frame(
        &mut self,
        ctx: &mut UpdateCtx<'_>,
        _props: &mut PropertiesMut<'_>,
        interval: u64,
    ) {
        // If paint() discovered wider content than layout() reported, trigger re-layout
        // so Portal learns the true content width and enables horizontal scrolling.
        if self.viewport.check_and_clear_width_update() {
            ctx.request_layout();
        }

        // Handle pending scroll from external navigation (goto_line, search results).
        // This runs in on_anim_frame because perform_goto_line calls request_anim_frame(),
        // ensuring scroll happens after layout computes correct viewport dimensions.
        if self.viewport.needs_scroll_to_cursor {
            if let Some(rect) = self.cursor_rect() {
                if self.scroll_to_show_rect(rect) {
                    self.viewport.needs_scroll_to_cursor = false;
                    ctx.request_render();
                } else {
                    // Viewport size not ready yet, retry on next frame
                    ctx.request_anim_frame();
                }
            } else {
                // No cursor rect available, clear the flag
                self.viewport.needs_scroll_to_cursor = false;
            }
        }

        self.hover_anim_frame(ctx);

        // Delegate cursor blink animation to CursorAnimator
        let is_focused = ctx.is_window_focused() && ctx.is_focus_target();
        let result = self.animator.tick(interval, is_focused);

        if result.visibility_changed {
            ctx.request_paint_only();
        }
        if result.request_frame {
            ctx.request_anim_frame();
        }
    }

    // MARK: Widget — pointer events (mouse, scroll, drag)

    fn on_pointer_event(
        &mut self,
        ctx: &mut EventCtx<'_>,
        _props: &mut PropertiesMut<'_>,
        event: &PointerEvent,
    ) {
        self.on_pointer_event_impl(ctx, _props, event);
    }

    // MARK: Widget — text events (keyboard, IME, completion + snippet routing)

    fn on_text_event(
        &mut self,
        ctx: &mut EventCtx<'_>,
        _props: &mut PropertiesMut<'_>,
        event: &TextEvent,
    ) {
        self.on_text_event_impl(ctx, _props, event);
    }

    // MARK: Widget — focus + accessibility

    fn accepts_focus(&self) -> bool {
        true
    }

    fn accepts_text_input(&self) -> bool {
        true
    }

    fn on_access_event(
        &mut self,
        _ctx: &mut EventCtx<'_>,
        _props: &mut PropertiesMut<'_>,
        _event: &AccessEvent,
    ) {
        // TODO: Accessibility selection support
    }

    fn register_children(&mut self, _ctx: &mut RegisterCtx<'_>) {}

    // MARK: Widget — property + update lifecycle

    fn property_changed(&mut self, ctx: &mut UpdateCtx<'_>, property_type: TypeId) {
        CaretColor::prop_changed(ctx, property_type);
        ContentColor::prop_changed(ctx, property_type);
        ContentColor::prop_changed(ctx, property_type);
        SelectionColor::prop_changed(ctx, property_type);
        SelectionColor::prop_changed(ctx, property_type);
    }

    // MARK: Widget — measure + layout

    fn measure(
        &mut self,
        _ctx: &mut MeasureCtx<'_>,
        _props: &PropertiesRef<'_>,
        axis: Axis,
        len_req: LenReq,
        _cross_length: Option<Length>,
    ) -> Length {
        // =========================================================================
        // INTERNAL SCROLLING: Fit available space, scroll content internally
        // =========================================================================
        // Unlike Portal-wrapped widgets that return full content size, we return
        // the available space and handle scrolling ourselves. This ensures the
        // widget fits its container while allowing internal scroll.
        //
        // CRITICAL: MinContent must be small. Returning a large value (e.g.
        // 400px for height) propagates up through nested splits and forces
        // the parent flex column to allocate at least that much, which on
        // shorter windows pushes the status bar off the bottom of the
        // screen when the debug area is open. Since the editor scrolls
        // internally, it can functionally render at any height — even one
        // line — so we report a small minimum here.
        let line_height = self.lines.line_height().max(16.0);
        match axis {
            Axis::Vertical => match len_req {
                LenReq::FitContent(available) => available,
                LenReq::MinContent => Length::px(line_height),
                LenReq::MaxContent => Length::px(400.0),
            },
            Axis::Horizontal => match len_req {
                LenReq::FitContent(available) => available,
                LenReq::MinContent => Length::px(80.0),
                LenReq::MaxContent => Length::px(600.0),
            },
        }
    }

    fn update(&mut self, ctx: &mut UpdateCtx<'_>, _props: &mut PropertiesMut<'_>, event: &Update) {
        // Handle pending scroll from external navigation (goto_line, search results)
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

        // Emit ScrollChanged whenever the viewport actually moved since
        // the last emission. Listeners (e.g. compile-output autoscroll)
        // use this to learn the user's scroll position. We coalesce
        // tiny movements (<0.5 px) to avoid spamming the channel.
        if let Some(scroll_y) = self.scroll.take_scroll_change() {
            let line_height = self.lines.line_height().max(1.0);
            ctx.submit_action::<Self::Action>(CodeEditorAction::ScrollChanged {
                scroll_y,
                line_height,
            });
        }

        // Push the freshly-parsed section markers up to the View
        // layer so the Structure outline can render section roots.
        // Set by `rehighlight` when the marker list actually
        // changed (so we don't spam the channel on every tick).
        if self.section_markers_dirty {
            self.section_markers_dirty = false;
            ctx.submit_action::<Self::Action>(CodeEditorAction::SectionMarkersChanged(
                self.section_markers.clone(),
            ));
        }

        // If paint() discovered wider content, trigger re-layout for horizontal scrolling.
        if self.viewport.needs_width_update {
            self.viewport.needs_width_update = false;
            ctx.request_layout();
        }
        match event {
            Update::FocusChanged(false) => {
                self.hover_focus_lost(ctx);
                ctx.request_render();
            }
            Update::FocusChanged(_) | Update::DisabledChanged(_) => {
                ctx.request_render();
            }
            _ => {}
        }
    }

    fn layout(&mut self, ctx: &mut LayoutCtx<'_>, _props: &PropertiesRef<'_>, size: Size) {
        // Store viewport size for scroll calculations
        self.scroll.set_viewport_size(size);

        // =========================================================================
        // CLIP TO BOUNDS
        // =========================================================================
        // Set clip path to prevent content from painting outside the widget bounds.
        // This is essential for internal scroll handling - without Portal's automatic
        // clipping, we must explicitly clip scrolled content to the viewport.
        // (Adapted from Portal widget: ctx.set_clip_path(size.to_rect()))
        ctx.set_clip_path(size.to_rect());

        // Compute gutter width from line count
        self.config.gutter_width = if self.config.show_line_numbers {
            crate::text_layout::compute_gutter_width(self.document.line_count())
        } else {
            0.0
        };

        // Ensure the line cache matches the document size
        let line_count = self.document.line_count();
        self.lines.resize(line_count);

        // Compute max content width from all cached line layouts.
        // This is O(line_count) but only reads cached values — no Parley work.
        // Lines that haven't been painted yet have no cached layout and are skipped.
        let mut max_line_width: f64 = 0.0;
        for i in 0..line_count {
            if let Some(ll) = self.lines.get_layout(i) {
                let w = ll.layout.width() as f64;
                if w > max_line_width {
                    max_line_width = w;
                }
            }
        }
        self.viewport.max_content_width = max_line_width;

        // Compute content size for scroll bounds
        // Width: gutter + max line width + padding
        // Height: total document height + padding
        let content_width = self.config.gutter_width + max_line_width + 100.0;
        let content_height = self.lines.total_height() + 48.0;
        self.scroll
            .set_content_size(Size::new(content_width, content_height));

        // Clamp viewport position to valid range after content size change
        let _ = self.scroll.set_viewport_pos(self.scroll.pos());

        // Store the available width for later use (e.g., detecting resize)
        self.viewport.last_available_width = Some(size.width as f32);
    }

    // MARK: Widget — paint orchestration

    fn paint(
        &mut self,
        ctx: &mut PaintCtx<'_>,
        props: &PropertiesRef<'_>,
        painter: &mut Painter<'_>,
    ) {
        self.paint_impl(ctx, props, painter);
    }

    // MARK: Widget — Masonry trait misc (cursor icon, accessibility, debug)

    fn get_cursor(&self, _ctx: &QueryCtx<'_>, _pos: Point) -> CursorIcon {
        CursorIcon::Text
    }

    fn accessibility_role(&self) -> Role {
        Role::MultilineTextInput
    }

    fn accessibility(
        &mut self,
        _ctx: &mut AccessCtx<'_>,
        _props: &PropertiesRef<'_>,
        _node: &mut Node,
    ) {
        // TODO: Accessibility support via AccessKit
        // The old PlainEditor-based widget used editor.try_accessibility()
        // which we no longer have. This needs a custom implementation.
    }

    fn children_ids(&self) -> ChildrenIds {
        ChildrenIds::new()
    }

    fn make_trace_span(&self, id: WidgetId) -> Span {
        trace_span!("CodeEditor", id = id.trace())
    }

    fn get_debug_text(&self) -> Option<String> {
        let text = self.document.to_string();
        Some(text.chars().take(100).collect())
    }
}
