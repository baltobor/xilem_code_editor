// Copyright 2026 the xilem_code_editor developers
// SPDX-License-Identifier: Apache-2.0
// Adapted from Lapce (Apache-2.0) and Floem (MIT); see README.md.

//! Document trait — the buffer abstraction for the code editor.
//!
//! This trait separates text storage from the editor widget, allowing different
//! backends (Rope, String, collaborative CRDT) while keeping the widget unchanged.
//! Inspired by Lapce's Document trait, adapted for our Xilem/Masonry context.

use std::ops::Range;

use ropey::{Rope, RopeSlice};

use crate::selection::SelRegion;

/// The Document trait abstracts over text buffer implementations.
///
/// The editor widget operates exclusively through this trait — it never
/// accesses the underlying storage directly. This enables:
/// - `RopeDocument` for efficient large-file editing (the default)
/// - Future collaborative backends (OT/CRDT) without widget changes
/// - Testing with simple in-memory buffers
pub trait Document: Send + Sync {
    /// Returns a reference to the full text content as a Rope.
    /// Even non-rope backends must present their data through this type,
    /// because the layout engine needs efficient random access by line.
    fn text(&self) -> &Rope;

    /// Returns the number of lines in the document.
    /// A trailing newline counts as an extra empty line.
    fn line_count(&self) -> usize;

    /// Returns the text of a specific line (without trailing newline).
    /// Returns None if `line` is out of bounds.
    fn line_content(&self, line: usize) -> Option<RopeSlice<'_>>;

    /// Returns the byte offset where a given line starts.
    /// Panics if `line > line_count()`.
    fn offset_of_line(&self, line: usize) -> usize;

    /// Returns the line number (0-based) containing the given byte offset.
    /// Clamps to the last line if offset exceeds document length.
    fn line_of_offset(&self, offset: usize) -> usize;

    /// Returns the total length of the document in bytes.
    fn len_bytes(&self) -> usize;

    /// Returns true if the document contains no text.
    fn is_empty(&self) -> bool {
        self.len_bytes() == 0
    }

    /// Apply an edit: replace the byte range `range` with `new_text`.
    /// This is the single mutation entry point — all edits go through here.
    /// Implementations must increment their revision counter.
    fn edit(&mut self, range: Range<usize>, new_text: &str);

    /// Returns a revision counter that increments on every edit.
    /// Used by the layout cache to detect when cached layouts are stale.
    fn revision(&self) -> u64;

    /// Collect the full document text into an owned String.
    /// Used when syncing back to the view layer (Xilem needs owned strings).
    fn to_string(&self) -> String;

    /// Begin an edit group (transaction). All `edit()` calls until `end_edit_group()`
    /// will be grouped into a single undoable unit.
    fn begin_edit_group(&mut self, cursor: &[SelRegion]);

    /// End the current edit group, saving cursor state after the action.
    fn end_edit_group(&mut self, cursor: &[SelRegion]);

    /// Undo the most recent edit group. Returns the cursor state to restore,
    /// or None if there is nothing to undo.
    fn undo(&mut self) -> Option<Vec<SelRegion>>;

    /// Redo the most recently undone edit group. Returns the cursor state to
    /// restore, or None if there is nothing to redo.
    fn redo(&mut self) -> Option<Vec<SelRegion>>;

    /// Drop all recorded undo / redo history. Call after a wholesale
    /// document replacement so future undos don't replay edits
    /// against a stale rope.
    fn clear_history(&mut self);
}
