//! Actions emitted by the code editor widget.
//!
//! The Xilem view layer receives these actions and routes them to
//! the appropriate callbacks in the application model.

/// Actions emitted by the code editor widget.
#[derive(Debug)]
#[allow(dead_code)]
pub enum CodeEditorAction {
    /// The text content has changed. Contains the full new text.
    ContentChanged(String),
    /// The cursor has moved to a new position (1-based line and column).
    CursorMoved { line: usize, col: usize },
    /// The pointer rested on the word at 1-based `line`/`col` (zed timing:
    /// asked after half the hover delay). Answer with the view's `hover`
    /// property; the caret does not move.
    HoverAt { line: usize, col: usize },
    /// The primary selection as byte offsets, `start <= end`
    /// (`start == end`: a plain caret). Sent with every `CursorMoved`.
    SelectionChanged { start: usize, end: usize },
    /// The user clicked on a diagnostic overlay at the given line.
    DiagnosticClicked { line: usize },
    /// The user clicked in the gutter to toggle a breakpoint (1-based line).
    GutterClicked { line: usize },
    /// Go to Definition requested (Ctrl+Cmd+Click or menu).
    GotoDefinition { line: usize, col: usize },
    /// Hover/documentation preview requested (Alt+Click).
    HoverRequest { line: usize, col: usize },
    /// Increase editor font size (Cmd/Ctrl + Plus).
    IncreaseFontSize,
    /// Decrease editor font size (Cmd/Ctrl + Minus).
    DecreaseFontSize,
    /// The scroll position has changed (for syncing with external gutter).
    ScrollChanged { scroll_y: f64, line_height: f64 },
    /// Context menu requested (right-click without modifiers).
    ContextMenuRequested { line: usize, col: usize },
    /// Request completion at cursor position via explicit user
    /// gesture (Ctrl+Space). Forces the popup open even when
    /// the auto-trigger suppression flag is set after Esc.
    RequestCompletionExplicit { line: usize, col: usize },
    /// Request completion at cursor position via auto-trigger
    /// (typed character matched a trigger). Honours the
    /// auto-trigger suppression flag (post-Esc dismissal).
    RequestCompletion { line: usize, col: usize },
    /// Notify the model that the user typed a word-boundary
    /// character (space, punctuation, etc.). The model uses
    /// this to clear post-Esc completion suppression so the
    /// popup may auto-reopen on the next trigger.
    WordBoundaryTyped,
    /// Esc-on-visible-popup: dismiss the popup AND set the
    /// suppression flag so it doesn't auto-reopen on the next
    /// letter of the current word. Different from
    /// `DismissPopups` (which is the broader "close any popup"
    /// path triggered also by clicking outside, etc.).
    DismissCompletionByEsc,
    /// Request code actions at cursor position (Ctrl+.).
    RequestCodeActions { line: usize, col: usize },
    /// Request code actions at cursor position to surface them
    /// AS items inside the regular completion popup (rather than
    /// the dedicated code-actions popup that Ctrl+. opens).
    /// Emitted by structural events that produce a "blank slot
    /// where high-confidence code actions might apply" — most
    /// notably the cursor landing inside an empty `{ }` block
    /// after the smart-Return expansion. The model's code-actions
    /// response handler routes results into
    /// `completion_code_action_items` when `source ==
    /// CompletionPopup`, leaving the dedicated popup alone.
    RequestCodeActionsForPopup { line: usize, col: usize },
    /// Request signature help at cursor position (Ctrl+Shift+Space).
    RequestSignatureHelp { line: usize, col: usize },
    /// Dismiss all popups (Escape).
    DismissPopups,
    /// Request help documentation (F1).
    RequestHelp,
    /// Accept the current completion (Tab/Enter with completion visible).
    AcceptCompletion,
    /// Accept a specific completion item by index. Emitted when the
    /// user clicks an entry in the caret-anchored completion popup.
    AcceptCompletionAt { index: usize },
    /// Select previous completion item (Up arrow with completion visible).
    CompletionSelectPrev,
    /// Select next completion item (Down arrow with completion visible).
    CompletionSelectNext,
    /// The set of `// MARK:` / `// ---` section markers in the document
    /// has changed (a rehighlight pass produced different markers than
    /// the last reported set). Carries the full new list, sorted by
    /// line. The view layer uses this to push markers to AppModel so
    /// the Structure outline can render section roots.
    SectionMarkersChanged(Vec<code_editor_core::model::SectionMarker>),
    /// The user just expanded an inline diagnostic bubble — the view
    /// layer should request code actions for that line so quick-fix
    /// rows can populate the bubble. `line` is 0-based.
    RequestBubbleCodeActions { line: usize },
    /// The user clicked a quick-fix row inside an expanded bubble. The
    /// view layer applies the corresponding `WorkspaceEdit` from
    /// `bubble_code_actions[(rel_path, line)][action_index]`.
    ApplyDiagnosticCodeAction { line: usize, action_index: usize },
    /// A registered gutter control consumed a click on 1-based `line`.
    GutterControlClicked { control: &'static str, line: usize },
    /// Close the Quick Help balloon (click outside or Escape).
    CloseQuickHelp,
    /// Update the Quick Help balloon scroll offset.
    QuickHelpScroll { offset: f64 },
    /// Insert a removed-line run from the VCS deletion balloon. `after_new_line`
    /// is the 1-based new-side line the removal sits after (0 = top of file);
    /// `text` is the removed content to re-insert there.
    VcsInsertDeletion { after_new_line: usize, text: String },
}
