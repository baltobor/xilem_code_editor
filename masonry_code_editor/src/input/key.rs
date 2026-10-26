//! Keyboard event router — top-level `handle_key_event` and the
//! tiny bracket-pair helpers used by its auto-close logic.
//!
//! Priority chain (matches the call order inside the function):
//!   1. snippet handler  — see `super::try_handle_snippet_key`.
//!   2. bubble handler   — see `super::try_handle_bubble_key`.
//!   3. completion popup nav, generic Esc, normal text editing.
//!
//! Word-boundary helpers used by Tab/Shift-Tab indent live in
//! `word_helpers.rs`.

use masonry::core::EventCtx;
use masonry::core::keyboard::{Key, NamedKey};

use super::{InputResult, try_handle_bubble_key, try_handle_snippet_key};
use crate::text_layout::Lines;
use code_editor_core::cursor::Cursor;
use code_editor_core::cursor::word_helpers::indent_selection;
use code_editor_core::document::Document;

// MARK: Bracket helpers

fn bracket_pair(ch: char) -> Option<char> {
    match ch {
        '{' => Some('}'),
        _ => None,
    }
}

/// Check if a character is a closing bracket.
fn is_closing_bracket_char(ch: char) -> bool {
    matches!(ch, ')' | ']' | '}' | '"' | '\'')
}

/// Handle a keyboard event (key down).
///
/// Returns what happened so the widget can decide what to request
/// (layout, render, action submission).
///
/// `bracket_ghost` is the current bracket ghost character (if any), used for
/// Tab to accept and skip-over behavior.
///
/// `completion_ghost` indicates whether completion ghost text is visible.
/// Tab accepts completion ghost with higher priority than bracket ghost.
///
/// `completion_visible` indicates whether the completion popup is visible.
/// When visible, Up/Down navigate the list and Tab always accepts the
/// selection.
///
/// `popup_user_navigated` mirrors `AppModel.popup_user_navigated`.
/// When true (popup visible AND user has pressed Up/Down at least
/// once), Enter accepts the highlighted item too. When false,
/// Enter inserts a newline + smart-bracket-expansion and dismisses
/// the popup.
pub fn handle_key_event(
    ctx: &mut EventCtx<'_>,
    cursor: &mut Cursor,
    doc: &mut dyn Document,
    lines: &Lines,
    key: &Key,
    shift: bool,
    action_mod: bool,
    _alt: bool,
    tab_size: usize,
    bracket_ghost: Option<char>,
    completion_ghost: bool,
    completion_visible: bool,
    popup_user_navigated: bool,
    // `Some(action_count)` when a diagnostic bubble is expanded and
    // has at least one quick-fix card. Drives bubble keyboard nav
    // (Up/Down/Tab/Return/Esc) before completion or normal keys
    // kick in.
    bubble_action_count: Option<usize>,
    // `true` when a multi-stop snippet session is active. Drives
    // snippet keyboard nav (Tab / Shift+Tab / Return / Esc) before
    // completion or bubble routing kicks in. The widget is the source
    // of truth for the session — input.rs only translates keys to
    // intent flags on InputResult.
    snippet_active: bool,
) -> InputResult {
    // Priority chain: snippet → bubble → (the big match below).
    // Each helper is a pure function; see their doc comments for the
    // owned-key sets and the priority-overlap rules. The order of
    // these two early-returns is the *behavioural contract* — flipping
    // them would mean a bubble would steal Tab/Enter from an active
    // snippet. The tests in `mod tests` at the bottom of the file
    // pin both the per-handler key tables and this priority order.
    if let Some(result) = try_handle_snippet_key(key, shift, action_mod, snippet_active) {
        return result;
    }
    if let Some(result) = try_handle_bubble_key(key, shift, action_mod, bubble_action_count) {
        return result;
    }

    match key {
        // --- Completion navigation (when popup is visible) ---
        // Up arrow: select previous item
        Key::Named(NamedKey::ArrowUp) if completion_visible && !action_mod && !shift => {
            return InputResult {
                completion_select_prev: true,
                handled: true,
                ..InputResult::default()
            };
        }
        // Down arrow: select next item
        Key::Named(NamedKey::ArrowDown) if completion_visible && !action_mod && !shift => {
            return InputResult {
                completion_select_next: true,
                handled: true,
                ..InputResult::default()
            };
        }
        // Enter does NOT accept popup items. The user-locked
        // contract: Tab accepts (popup item / extension /
        // snippet); Return is reserved for newline +
        // smart-bracket-expansion. Mixing the two ("Return
        // sometimes accepts, sometimes newlines") was a
        // primary source of confusion. The branch
        // that handled Enter on visible-popup is intentionally
        // deleted; Enter falls through to the regular Enter
        // handler (line ~360 below), which inserts newline
        // and triggers `accept_bracket_ghost_newline` when
        // applicable. The completion popup is still
        // implicitly dismissed because typing a newline
        // changes the cursor position to a fresh line where
        // the auto-trigger doesn't fire.

        // --- Escape: Dismiss popups ---
        Key::Named(NamedKey::Escape) => {
            // When the completion popup is visible, route to
            // the suppression-aware dismissal so the popup
            // doesn't immediately auto-reopen on the next
            // letter of the current word. Other popups (hover,
            // signature help, code-actions) still go through
            // the regular dismiss_popups path.
            if completion_visible {
                return InputResult {
                    dismiss_completion_by_esc: true,
                    handled: true,
                    ..InputResult::default()
                };
            }
            return InputResult {
                dismiss_popups: true,
                handled: true,
                ..InputResult::default()
            };
        }

        // --- Undo ---
        Key::Character(z) if action_mod && !shift && z.as_str().eq_ignore_ascii_case("z") => {
            return InputResult {
                undo: true,
                handled: true,
                ..InputResult::default()
            };
        }

        // --- Redo ---
        Key::Character(z) if action_mod && shift && z.as_str().eq_ignore_ascii_case("z") => {
            return InputResult {
                redo: true,
                handled: true,
                ..InputResult::default()
            };
        }

        // --- Go to Definition: Cmd+B (macOS) / Ctrl+B (Linux) ---
        Key::Character(b) if action_mod && !shift && b.as_str().eq_ignore_ascii_case("b") => {
            return InputResult {
                goto_definition: true,
                handled: true,
                ..InputResult::default()
            };
        }

        // --- Hover Preview: Cmd+Shift+B (macOS) / Ctrl+Shift+B (Linux) ---
        Key::Character(b) if action_mod && shift && b.as_str().eq_ignore_ascii_case("b") => {
            return InputResult {
                hover_request: true,
                handled: true,
                ..InputResult::default()
            };
        }

        // --- Font Size: Cmd/Ctrl + Plus/Equals to increase ---
        Key::Character(c) if action_mod && !shift && (c.as_str() == "+" || c.as_str() == "=") => {
            return InputResult {
                increase_font_size: true,
                handled: true,
                ..InputResult::default()
            };
        }

        // --- Font Size: Cmd/Ctrl + Minus to decrease ---
        Key::Character(c) if action_mod && !shift && c.as_str() == "-" => {
            return InputResult {
                decrease_font_size: true,
                handled: true,
                ..InputResult::default()
            };
        }

        // --- Completion: Ctrl+Space ---
        Key::Character(sp) if action_mod && !shift && sp.as_str() == " " => {
            return InputResult {
                request_completion: true,
                handled: true,
                ..InputResult::default()
            };
        }

        // --- Code Actions: Ctrl+. ---
        Key::Character(dot) if action_mod && !shift && dot.as_str() == "." => {
            return InputResult {
                request_code_actions: true,
                handled: true,
                ..InputResult::default()
            };
        }

        // --- Signature Help: Ctrl+Shift+Space ---
        Key::Character(sp) if action_mod && shift && sp.as_str() == " " => {
            return InputResult {
                request_signature_help: true,
                handled: true,
                ..InputResult::default()
            };
        }

        // --- Help: F1 ---
        Key::Named(NamedKey::F1) => {
            return InputResult {
                request_help: true,
                handled: true,
                ..InputResult::default()
            };
        }

        // --- Cut ---
        Key::Character(x) if action_mod && x.as_str().eq_ignore_ascii_case("x") => {
            if let Some(text) = cursor.selected_text(doc) {
                if !text.is_empty() {
                    cursor.delete(doc);
                    ctx.set_clipboard(text);
                    return InputResult::edit();
                }
            }
            InputResult::handled()
        }

        // --- Copy ---
        Key::Character(c) if action_mod && c.as_str().eq_ignore_ascii_case("c") => {
            if let Some(text) = cursor.selected_text(doc) {
                if !text.is_empty() {
                    ctx.set_clipboard(text);
                }
            }
            InputResult::handled()
        }

        // --- Select All / Deselect ---
        Key::Character(a) if action_mod && a.as_str().eq_ignore_ascii_case("a") => {
            if shift {
                cursor.collapse_selection();
            } else {
                cursor.select_all(doc);
            }
            InputResult::cursor_only()
        }

        // --- Arrow Left ---
        Key::Named(NamedKey::ArrowLeft) => {
            if action_mod && shift {
                cursor.select_to_line_start(doc);
            } else if action_mod {
                cursor.move_to_line_start(doc);
            } else if shift {
                cursor.select_left(doc);
            } else {
                cursor.move_left(doc);
            }
            InputResult::cursor_only()
        }

        // --- Arrow Right ---
        Key::Named(NamedKey::ArrowRight) => {
            if action_mod && shift {
                cursor.select_to_line_end(doc);
            } else if action_mod {
                cursor.move_to_line_end(doc);
            } else if shift {
                cursor.select_right(doc);
            } else {
                cursor.move_right(doc);
            }
            InputResult::cursor_only()
        }

        // --- Arrow Up ---
        Key::Named(NamedKey::ArrowUp) => {
            if shift {
                cursor.select_up(doc, lines);
            } else {
                cursor.move_up(doc, lines);
            }
            InputResult::cursor_only()
        }

        // --- Arrow Down ---
        Key::Named(NamedKey::ArrowDown) => {
            if shift {
                cursor.select_down(doc, lines);
            } else {
                cursor.move_down(doc, lines);
            }
            InputResult::cursor_only()
        }

        // --- Home ---
        Key::Named(NamedKey::Home) => {
            if action_mod && shift {
                cursor.select_to_start();
            } else if action_mod {
                cursor.move_to_start();
            } else if shift {
                cursor.select_to_line_start(doc);
            } else {
                cursor.move_to_line_start(doc);
            }
            InputResult::cursor_only()
        }

        // --- End ---
        Key::Named(NamedKey::End) => {
            if action_mod && shift {
                cursor.select_to_end(doc);
            } else if action_mod {
                cursor.move_to_end(doc);
            } else if shift {
                cursor.select_to_line_end(doc);
            } else {
                cursor.move_to_line_end(doc);
            }
            InputResult::cursor_only()
        }

        // --- Delete ---
        Key::Named(NamedKey::Delete) => {
            if action_mod {
                cursor.delete_word(doc);
            } else {
                cursor.delete(doc);
            }
            InputResult::edit()
        }

        // --- Backspace ---
        Key::Named(NamedKey::Backspace) => {
            if action_mod {
                cursor.backdelete_word(doc);
            } else {
                cursor.backdelete(doc);
            }
            InputResult::edit()
        }

        // --- Space ---
        Key::Character(sp) if sp.as_str() == " " => {
            cursor.insert(doc, " ");
            InputResult::edit()
        }

        // --- Enter ---
        Key::Named(NamedKey::Enter) => {
            // Return-armed accept: when the popup is visible AND
            // the user has navigated with Up/Down at least once,
            // Return accepts the highlighted item just like Tab
            // does. This is the "menu feel" path — after picking
            // a non-default with arrows, muscle memory expects
            // Return to commit.
            //
            // Without navigation the popup stays in "terminal
            // feel" mode: Tab accepts; Return is for newlines.
            // The flag is the load-bearing protection against
            // false-accepts from auto-opened popups.
            if completion_visible && popup_user_navigated {
                return InputResult {
                    accept_completion_ghost: true,
                    handled: true,
                    ..InputResult::default()
                };
            }

            // If there's a bracket ghost, Enter accepts it with newline in between
            if bracket_ghost.is_some() {
                return InputResult {
                    accept_bracket_ghost_newline: true,
                    handled: true,
                    // Implicitly dismiss any visible popup —
                    // Return is for newline + smart-expand, not
                    // for accepting completion items, so the
                    // popup should go away even if it was
                    // visible.
                    dismiss_popups: completion_visible,
                    ..InputResult::default()
                };
            }
            // Get current line's leading whitespace for auto-indentation
            let offset = cursor.offset();
            let line = doc.line_of_offset(offset);
            let indent = if let Some(line_content) = doc.line_content(line) {
                let content_str = line_content.to_string();
                content_str
                    .chars()
                    .take_while(|c| *c == ' ' || *c == '\t')
                    .collect::<String>()
            } else {
                String::new()
            };
            // Insert newline + indentation
            if indent.is_empty() {
                cursor.insert(doc, "\n");
            } else {
                let insert_text = format!("\n{}", indent);
                cursor.insert(doc, &insert_text);
            }
            InputResult {
                clear_bracket_ghost: true,
                // Same reasoning: dismiss the completion popup
                // when Return inserts a plain newline.
                dismiss_popups: completion_visible,
                ..InputResult::edit()
            }
        }

        // --- Tab ---
        Key::Named(NamedKey::Tab) => {
            // Tab acceptance priority:
            // 1. Completion popup visible (any item highlighted) -
            //    accept the highlighted item.
            // 2. Bracket ghost — accept auto-closed bracket.
            // 3. Multi-line selection — indent / outdent each line
            //    (shift = outdent).
            // 4. Insert tab spaces at the caret(s).
            //
            // Note: priority #1 used to require the completion
            // GHOST to be visible (a render-side flag that lags
            // popup visibility on some servers / fast typing).
            // The user got into the situation where the popup
            // showed `match` but Tab inserted four spaces because
            // the ghost hadn't rendered yet — only Return worked.
            // Promoting `completion_visible` over `completion_ghost`
            // makes Tab unconditionally accept whatever the popup
            // shows, matching the user's mental model.
            //
            // We still set `accept_completion_ghost: true` because
            // the widget-side dispatcher reads that flag to emit
            // AcceptCompletion and then nulls out
            // `completion_ghost_text`. Both paths converge on the
            // same accept action.
            if completion_visible || completion_ghost {
                return InputResult {
                    accept_completion_ghost: true,
                    handled: true,
                    ..InputResult::default()
                };
            }
            if bracket_ghost.is_some() {
                return InputResult {
                    accept_bracket_ghost: true,
                    handled: true,
                    ..InputResult::default()
                };
            }
            // Multi-line block indent / outdent.
            //
            // "Multi-line" = the primary selection spans more than one
            // line (or the selection straddles a newline). In that
            // case Tab inserts `tab_size` spaces at the start of every
            // covered line; Shift+Tab removes up to `tab_size` leading
            // spaces (or one leading tab) from each.
            let primary = cursor.selection.primary();
            let is_multiline_selection = !primary.is_caret() && {
                let start_line = doc.line_of_offset(primary.min());
                let end_line =
                    doc.line_of_offset(primary.max().saturating_sub(1).max(primary.min()));
                start_line != end_line
            };
            if is_multiline_selection {
                let outdent = shift;
                indent_selection(doc, cursor, tab_size, outdent);
                return InputResult {
                    clear_bracket_ghost: true,
                    ..InputResult::edit()
                };
            }
            let spaces: String = " ".repeat(tab_size);
            cursor.insert(doc, &spaces);
            InputResult {
                clear_bracket_ghost: true,
                ..InputResult::edit()
            }
        }

        // --- Regular character input ---
        //
        // Operator Preference Order (highest to lowest priority):
        // 1. Skip keyboard shortcuts (action_mod held)
        // 2. Closing bracket skip-over (ghost match or existing bracket in doc)
        // 3. Opening bracket auto-close (insert bracket + set ghost + trigger completion)
        // 4. Normal character insertion (clears bracket ghost)
        //
        // This order ensures bracket handling takes precedence but still allows
        // completion to be triggered after opening brackets like '{'.
        Key::Character(text) => {
            // Skip if this is a keyboard shortcut (action_mod held with non-printable)
            if action_mod {
                return InputResult::nop();
            }

            let chars: Vec<char> = text.chars().collect();
            if chars.len() == 1 {
                let ch = chars[0];

                // Priority 2: Skip-over closing brackets
                if is_closing_bracket_char(ch) {
                    // Check if next char in doc matches (skip-over existing bracket)
                    // This handles AutoClose mode where the bracket was already inserted
                    let offset = cursor.offset();
                    if offset < doc.len_bytes() {
                        let text_rope = doc.text();
                        let char_idx = text_rope.byte_to_char(offset);
                        if char_idx < text_rope.len_chars() {
                            let next_char = text_rope.char(char_idx);
                            if next_char == ch {
                                cursor.move_right(doc);
                                return InputResult {
                                    cursor_moved: true,
                                    handled: true,
                                    clear_bracket_ghost: true,
                                    ..InputResult::default()
                                };
                            }
                        }
                    }
                    // In Ghost mode: if closing bracket matches ghost, just dismiss
                    // the ghost and insert the bracket normally (fall through below)
                    if let Some(ghost) = bracket_ghost {
                        if ghost == ch {
                            // Clear ghost, then let normal insertion happen below
                            cursor.insert(doc, text);
                            return InputResult {
                                clear_bracket_ghost: true,
                                ..InputResult::edit()
                            };
                        }
                    }
                }

                // Priority 3: Opening bracket auto-close.
                // Bracket handling has priority over normal input, so we must
                // explicitly request completion here for relevant brackets.
                if let Some(closing) = bracket_pair(ch) {
                    cursor.insert(doc, text);
                    return InputResult {
                        set_bracket_ghost: Some(closing),
                        clear_bracket_ghost: false,
                        typed_char: Some(ch),
                        ..InputResult::edit()
                    };
                }

                // Normal single character - record for trigger detection
                cursor.insert(doc, text);
                return InputResult {
                    clear_bracket_ghost: true,
                    typed_char: Some(ch),
                    ..InputResult::edit()
                };
            }

            // Priority 4: Multi-character input - clears any bracket ghost
            cursor.insert(doc, text);
            InputResult {
                clear_bracket_ghost: true,
                ..InputResult::edit()
            }
        }

        _ => InputResult::nop(),
    }
}
