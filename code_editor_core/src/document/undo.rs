// Copyright 2026 the xilem_code_editor developers
// SPDX-License-Identifier: Apache-2.0

//! Undo/Redo manager — transaction-based undo for the code editor.
//!
//! Each user action (keystroke, paste, etc.) produces a Transaction that groups
//! all `doc.edit()` calls from that action. Multi-cursor edits produce multiple
//! EditOps per transaction but undo/redo as a single unit.

use std::ops::Range;

use crate::selection::SelRegion;

/// A single rope edit operation (one `doc.edit()` call).
#[derive(Debug, Clone)]
struct EditOp {
    /// Byte range in the document that was replaced.
    range: Range<usize>,
    /// Text that was removed (empty for pure insertions).
    old_text: String,
    /// Text that was inserted (empty for pure deletions).
    new_text: String,
}

/// Groups all edits from one user action into an undoable unit.
#[derive(Debug, Clone)]
struct Transaction {
    /// Edits in application order.
    edits: Vec<EditOp>,
    /// Cursor state before the action.
    cursor_before: Vec<SelRegion>,
    /// Cursor state after the action.
    cursor_after: Vec<SelRegion>,
}

/// Manages undo/redo stacks with transaction grouping.
pub struct UndoManager {
    undo_stack: Vec<Transaction>,
    redo_stack: Vec<Transaction>,
    /// Transaction currently being built (between begin/end).
    active: Option<Transaction>,
    max_history: usize,
}

impl UndoManager {
    /// Build an empty undo manager. The history cap is 500
    /// transactions; older entries are evicted from the bottom of
    /// the undo stack as new transactions complete.
    pub fn new() -> Self {
        Self {
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            active: None,
            max_history: 500,
        }
    }

    /// Drop all recorded history. Call after a wholesale document
    /// replacement (e.g. an out-of-band `set_text`) — without this
    /// the byte ranges in pending transactions reference offsets in
    /// a rope that no longer exists, and undo will panic on the
    /// next attempt to walk them.
    pub fn clear(&mut self) {
        self.undo_stack.clear();
        self.redo_stack.clear();
        self.active = None;
    }

    /// Start a new transaction. Call before processing a user action.
    pub fn begin(&mut self, cursor: &[SelRegion]) {
        self.active = Some(Transaction {
            edits: Vec::new(),
            cursor_before: cursor.to_vec(),
            cursor_after: Vec::new(),
        });
    }

    /// Record a single edit within the active transaction.
    /// Called from `Document::edit()` before mutating the rope.
    pub fn record(&mut self, range: Range<usize>, old_text: String, new_text: String) {
        if let Some(tx) = &mut self.active {
            tx.edits.push(EditOp {
                range,
                old_text,
                new_text,
            });
        }
    }

    /// Close the active transaction and push it to the undo stack.
    /// Clears the redo stack (new edit after undo invalidates redo).
    pub fn end(&mut self, cursor: &[SelRegion]) {
        if let Some(mut tx) = self.active.take() {
            if tx.edits.is_empty() {
                return; // No edits recorded — don't push empty transactions
            }
            tx.cursor_after = cursor.to_vec();
            self.undo_stack.push(tx);
            self.redo_stack.clear();

            // Cap history size
            if self.undo_stack.len() > self.max_history {
                self.undo_stack.remove(0);
            }
        }
    }

    /// Undo the most recent transaction.
    ///
    /// Applies edits in reverse order directly to the rope, updates the
    /// revision counter, and returns the cursor state to restore.
    pub fn undo(&mut self, rope: &mut ropey::Rope, revision: &mut u64) -> Option<Vec<SelRegion>> {
        // If there's an active (incomplete) transaction, discard it
        self.active = None;

        let tx = self.undo_stack.pop()?;

        // Apply edits in reverse order
        for edit in tx.edits.iter().rev() {
            let new_text_len = edit.new_text.len();
            let insert_start = edit.range.start;
            let insert_end = insert_start + new_text_len;

            // Remove the new_text that was inserted
            let start_char = rope.byte_to_char(insert_start);
            let end_char = rope.byte_to_char(insert_end);
            if start_char != end_char {
                rope.remove(start_char..end_char);
            }

            // Re-insert the old_text that was removed
            if !edit.old_text.is_empty() {
                rope.insert(start_char, &edit.old_text);
            }

            *revision += 1;
        }

        let cursor = tx.cursor_before.clone();
        self.redo_stack.push(tx);
        Some(cursor)
    }

    /// Redo the most recently undone transaction.
    ///
    /// Re-applies edits in forward order and returns the cursor state to restore.
    pub fn redo(&mut self, rope: &mut ropey::Rope, revision: &mut u64) -> Option<Vec<SelRegion>> {
        let tx = self.redo_stack.pop()?;

        // Apply edits in forward order (same as original application)
        for edit in &tx.edits {
            let start_char = rope.byte_to_char(edit.range.start);
            let end_char = rope.byte_to_char(edit.range.end);

            // Remove the old text
            if start_char != end_char {
                rope.remove(start_char..end_char);
            }

            // Insert the new text
            if !edit.new_text.is_empty() {
                rope.insert(start_char, &edit.new_text);
            }

            *revision += 1;
        }

        let cursor = tx.cursor_after.clone();
        self.undo_stack.push(tx);
        Some(cursor)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_rope(s: &str) -> ropey::Rope {
        ropey::Rope::from_str(s)
    }

    #[test]
    fn undo_single_insert() {
        let mut mgr = UndoManager::new();
        let mut rope = make_rope("hello");
        let mut rev = 1u64;

        // Simulate inserting " world" at position 5
        let cursor_before = vec![SelRegion::caret(5)];
        mgr.begin(&cursor_before);
        mgr.record(5..5, String::new(), " world".to_string());
        // Actually apply the edit to the rope
        rope.insert(rope.byte_to_char(5), " world");
        rev += 1;
        let cursor_after = vec![SelRegion::caret(11)];
        mgr.end(&cursor_after);

        assert_eq!(rope.to_string(), "hello world");

        // Undo
        let restored = mgr.undo(&mut rope, &mut rev).unwrap();
        assert_eq!(rope.to_string(), "hello");
        assert_eq!(restored, cursor_before);

        // Redo
        let restored = mgr.redo(&mut rope, &mut rev).unwrap();
        assert_eq!(rope.to_string(), "hello world");
        assert_eq!(restored, cursor_after);
    }

    #[test]
    fn undo_delete() {
        let mut mgr = UndoManager::new();
        let mut rope = make_rope("hello world");
        let mut rev = 1u64;

        // Simulate deleting " world" (bytes 5..11)
        let cursor_before = vec![SelRegion::new(5, 11)];
        mgr.begin(&cursor_before);
        mgr.record(5..11, " world".to_string(), String::new());
        let start_char = rope.byte_to_char(5);
        let end_char = rope.byte_to_char(11);
        rope.remove(start_char..end_char);
        rev += 1;
        let cursor_after = vec![SelRegion::caret(5)];
        mgr.end(&cursor_after);

        assert_eq!(rope.to_string(), "hello");

        // Undo should restore " world"
        let restored = mgr.undo(&mut rope, &mut rev).unwrap();
        assert_eq!(rope.to_string(), "hello world");
        assert_eq!(restored, cursor_before);
    }

    #[test]
    fn undo_multi_cursor_insert() {
        let mut mgr = UndoManager::new();
        // "ab" — two cursors, one at 1 and one at 2
        let mut rope = make_rope("ab");
        let mut rev = 1u64;

        let cursor_before = vec![SelRegion::caret(1), SelRegion::caret(2)];
        mgr.begin(&cursor_before);

        // Multi-cursor insert processes in reverse: cursor at 2 first, then at 1
        // Insert "X" at position 2
        mgr.record(2..2, String::new(), "X".to_string());
        rope.insert(rope.byte_to_char(2), "X");
        rev += 1;

        // Insert "X" at position 1
        mgr.record(1..1, String::new(), "X".to_string());
        rope.insert(rope.byte_to_char(1), "X");
        rev += 1;

        let cursor_after = vec![SelRegion::caret(2), SelRegion::caret(4)];
        mgr.end(&cursor_after);

        assert_eq!(rope.to_string(), "aXbX");

        // Undo should revert both inserts
        let restored = mgr.undo(&mut rope, &mut rev).unwrap();
        assert_eq!(rope.to_string(), "ab");
        assert_eq!(restored, cursor_before);
    }

    #[test]
    fn empty_undo_returns_none() {
        let mut mgr = UndoManager::new();
        let mut rope = make_rope("hello");
        let mut rev = 1u64;
        assert!(mgr.undo(&mut rope, &mut rev).is_none());
    }

    /// A quick-fix `WorkspaceEdit` typically applies several `TextEdit`s
    /// in reverse document order inside a single edit group. A single
    /// Cmd+Z must reverse the *whole* group — not just the last edit.
    /// This test is the regression net for the "quick-fix undo broken"
    /// bug fixed in commit 6628cc5.
    #[test]
    fn undo_workspace_edit_group_reverses_all_edits_at_once() {
        let mut mgr = UndoManager::new();
        let mut rope = make_rope("let p = Point;");
        let mut rev = 1u64;

        // Imagine the user clicked "Fill struct fields" — rust-analyzer
        // emits two edits replacing "Point" → "Point { x, y }" by way
        // of two TextEdits applied in reverse document order:
        //   edit 1: insert " { x, y }" at offset 13 (after "Point")
        //   edit 2: a no-op range — kept as a placeholder for a real
        //           multi-edit response shape.
        let cursor_before = vec![SelRegion::caret(0)];
        mgr.begin(&cursor_before);

        // Apply both edits in reverse document order, recording each
        // call exactly as `apply_workspace_edit` does.
        mgr.record(13..13, String::new(), " { x, y }".to_string());
        rope.insert(rope.byte_to_char(13), " { x, y }");
        rev += 1;

        mgr.record(0..0, String::new(), "// fixed: ".to_string());
        rope.insert(rope.byte_to_char(0), "// fixed: ");
        rev += 1;

        mgr.end(&[SelRegion::caret(0)]);

        assert_eq!(rope.to_string(), "// fixed: let p = Point { x, y };");

        // One undo should reverse the entire workspace edit at once.
        mgr.undo(&mut rope, &mut rev).unwrap();
        assert_eq!(rope.to_string(), "let p = Point;");
    }

    #[test]
    fn edit_after_undo_clears_redo() {
        let mut mgr = UndoManager::new();
        let mut rope = make_rope("hello");
        let mut rev = 1u64;

        // First edit
        mgr.begin(&[SelRegion::caret(5)]);
        mgr.record(5..5, String::new(), "!".to_string());
        rope.insert(rope.byte_to_char(5), "!");
        rev += 1;
        mgr.end(&[SelRegion::caret(6)]);

        // Undo
        mgr.undo(&mut rope, &mut rev);
        assert_eq!(rope.to_string(), "hello");

        // New edit should clear redo stack
        mgr.begin(&[SelRegion::caret(5)]);
        mgr.record(5..5, String::new(), "?".to_string());
        rope.insert(rope.byte_to_char(5), "?");
        rev += 1;
        mgr.end(&[SelRegion::caret(6)]);

        assert_eq!(rope.to_string(), "hello?");
        // Redo should return None (cleared)
        assert!(mgr.redo(&mut rope, &mut rev).is_none());
    }
}
