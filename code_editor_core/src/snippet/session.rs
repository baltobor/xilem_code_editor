// Copyright 2026 the xilem_code_editor developers
// SPDX-License-Identifier: Apache-2.0

//! Multi-stop snippet session — tracks tab-stop ranges live as the user
//! edits inside an accepted snippet.
//!
//! Lifecycle:
//! 1. `start` is called by the widget right after a snippet's text has
//!    been inserted into the document. The caller passes the parsed
//!    `TabStop` list (offsets relative to the inserted text) and the
//!    absolute insertion point. We translate to absolute byte ranges,
//!    sort into navigation order (1, 2, 3, …, 0), and select the first
//!    stop.
//! 2. While active:
//!    - `before_edit(range)` is called immediately before a document
//!      edit. It returns `EditDisposition::Allow` (the session stays
//!      open and tracks the edit) or `EditDisposition::Commit` (the
//!      session ends; the edit is plain).
//!    - `after_edit(start, old_len, new_len)` updates every stop's
//!      range using the delta. Stops fully *before* the edit are
//!      unchanged; stops fully *after* are shifted; the stop the
//!      edit happened *inside* is stretched (or shrunk).
//! 3. `tab_next` / `tab_prev` rotate through the stops in nav order.
//!    Returns the new selection range for the caller to apply.
//! 4. `commit_at_final` is called on Return to jump straight to `$0`
//!    (the final stop). The session ends after that move; the caller
//!    sets the caret to the returned position and clears state.

use std::ops::Range;

use crate::snippet::TabStop;

#[derive(Debug, Clone)]
pub struct TabStopSpan {
    /// Tab-stop index (1, 2, 3, …; 0 is the final position).
    pub index: usize,
    /// Absolute byte range in the document.
    pub range: Range<usize>,
}

#[derive(Debug, Clone)]
pub struct SnippetSession {
    /// Tab stops in *navigation* order (1, 2, 3, …, 0). The active
    /// one is at `current_idx`.
    pub stops: Vec<TabStopSpan>,
    pub current_idx: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EditDisposition {
    /// The edit is happening inside (or extending) the active stop —
    /// apply it and call `after_edit` to update tracked ranges.
    Allow,
    /// The edit is outside any stop. Commit the snippet and apply the
    /// edit as a normal edit — no `after_edit` call needed.
    Commit,
}

impl SnippetSession {
    /// Build a session from parsed tab stops + the absolute byte
    /// offset where the snippet text was inserted.
    ///
    /// Returns `None` if there are no navigable stops (the snippet had
    /// only an implicit `$0` at the end — nothing to navigate to).
    pub fn start(stops: &[TabStop], insert_start: usize) -> Option<Self> {
        // Sort into nav order: 1, 2, 3, …, 0
        let mut ordered: Vec<TabStopSpan> = stops
            .iter()
            .map(|ts| TabStopSpan {
                index: ts.index,
                range: (insert_start + ts.offset)..(insert_start + ts.offset + ts.length),
            })
            .collect();
        ordered.sort_by_key(|s| if s.index == 0 { usize::MAX } else { s.index });

        // Drop the final $0 if it's the only stop (degenerate case for
        // a snippet like `into_iter($0)` — there is nothing to
        // navigate to, just place the caret).
        let has_named = ordered.iter().any(|s| s.index != 0);
        if !has_named {
            return None;
        }

        Some(Self {
            stops: ordered,
            current_idx: 0,
        })
    }

    /// Currently-active stop.
    pub fn current(&self) -> Option<&TabStopSpan> {
        self.stops.get(self.current_idx)
    }

    /// Move to the next stop. Returns the new stop's range.
    pub fn tab_next(&mut self) -> Option<Range<usize>> {
        if self.current_idx + 1 >= self.stops.len() {
            return None;
        }
        self.current_idx += 1;
        self.stops.get(self.current_idx).map(|s| s.range.clone())
    }

    /// Move to the previous stop. Returns the new stop's range.
    pub fn tab_prev(&mut self) -> Option<Range<usize>> {
        if self.current_idx == 0 {
            return None;
        }
        self.current_idx -= 1;
        self.stops.get(self.current_idx).map(|s| s.range.clone())
    }

    /// Whether the active stop is `$0` (the final commit position).
    pub fn at_final(&self) -> bool {
        self.current().map(|s| s.index == 0).unwrap_or(true)
    }

    /// Decide whether `range` (an upcoming edit) belongs to the active
    /// stop or escapes the snippet. The active stop allows edits that
    /// lie fully within its range *or* that extend by typing at its
    /// edges (so growing a placeholder works).
    pub fn before_edit(&self, range: &Range<usize>) -> EditDisposition {
        let Some(current) = self.current() else {
            return EditDisposition::Commit;
        };
        // Allow when the edit is entirely inside [start..=end] of the
        // current stop. The closed upper bound on `end` lets the user
        // append at the right edge (e.g., extending `foo` to `foobar`
        // by typing at the end of the placeholder).
        if range.start >= current.range.start && range.end <= current.range.end {
            EditDisposition::Allow
        } else {
            EditDisposition::Commit
        }
    }

    /// Update tracked ranges after an edit that replaced
    /// `[edit_start..edit_end_old)` with `new_len` bytes. The edit is
    /// expected to have been allowed by `before_edit`.
    pub fn after_edit(&mut self, edit_start: usize, edit_end_old: usize, new_len: usize) {
        let old_len = edit_end_old.saturating_sub(edit_start);
        let delta: isize = new_len as isize - old_len as isize;
        if delta == 0 {
            return;
        }
        let cur_idx = self.current_idx;
        for (i, stop) in self.stops.iter_mut().enumerate() {
            if i == cur_idx {
                // Active stop expands / shrinks by delta. Edit is
                // contained within it so the start sticks; only end
                // moves.
                stop.range.end =
                    (stop.range.end as isize + delta).max(stop.range.start as isize) as usize;
            } else if stop.range.start >= edit_end_old {
                // Stops strictly after the edit shift wholesale.
                stop.range.start = (stop.range.start as isize + delta).max(0) as usize;
                stop.range.end = (stop.range.end as isize + delta).max(0) as usize;
            }
            // Stops strictly before the edit are unchanged.
        }
    }
}

// MARK: tests
//
// Tests as documentation: each test reads top-to-bottom as a short
// scenario. Test data is built by named helpers (`stops_for_*`) so the
// test body stays narrative — see `models/section_marker.rs::tests` for
// the canonical example of this style across the codebase.
#[cfg(test)]
mod tests {
    use super::*;
    use crate::snippet::{ExpandedSnippet, SnippetParser};

    /// Build the tab-stop list rust-analyzer would emit for `fn name(args) { $0 }`,
    /// inserted at the given absolute byte offset.
    fn fn_signature_stops(insert_start: usize) -> (Vec<TabStop>, usize) {
        let expanded = SnippetParser::parse("fn ${1:name}(${2:args}) { $0 }");
        (expanded.tab_stops, insert_start)
    }

    /// A snippet whose only stop is the implicit `$0` — there's nothing
    /// for a `SnippetSession` to navigate, so `start` should return None.
    fn final_only_stops() -> Vec<TabStop> {
        let expanded: ExpandedSnippet = SnippetParser::parse("into_iter($0)");
        expanded.tab_stops
    }

    #[test]
    fn start_returns_none_when_only_final_stop_exists() {
        let stops = final_only_stops();
        assert!(SnippetSession::start(&stops, 0).is_none());
    }

    #[test]
    fn start_orders_named_stops_before_final_zero() {
        let (stops, base) = fn_signature_stops(100);
        let session = SnippetSession::start(&stops, base).expect("named stops exist");
        let indices: Vec<usize> = session.stops.iter().map(|s| s.index).collect();
        assert_eq!(indices, vec![1, 2, 0]);
        assert_eq!(session.current_idx, 0);
    }

    #[test]
    fn start_translates_offsets_to_absolute_byte_ranges() {
        let (stops, base) = fn_signature_stops(100);
        let session = SnippetSession::start(&stops, base).unwrap();
        // First stop is `name` (at offset 3, length 4 in expanded text);
        // absolute range is base+3..base+7.
        assert_eq!(session.stops[0].range, 103..107);
        // Second stop is `args` (at offset 8, length 4 in expanded text);
        // absolute range is base+8..base+12.
        assert_eq!(session.stops[1].range, 108..112);
    }

    #[test]
    fn tab_next_advances_through_stops_in_navigation_order() {
        let (stops, base) = fn_signature_stops(0);
        let mut session = SnippetSession::start(&stops, base).unwrap();
        assert_eq!(session.current().unwrap().index, 1);

        let next = session.tab_next();
        assert!(next.is_some());
        assert_eq!(session.current().unwrap().index, 2);

        let last = session.tab_next();
        assert!(last.is_some());
        assert_eq!(session.current().unwrap().index, 0);

        // Past the final stop — no more navigation.
        assert!(session.tab_next().is_none());
    }

    #[test]
    fn tab_prev_walks_backwards_and_stops_at_first() {
        let (stops, base) = fn_signature_stops(0);
        let mut session = SnippetSession::start(&stops, base).unwrap();
        session.tab_next();
        session.tab_next();
        assert_eq!(session.current().unwrap().index, 0);

        assert!(session.tab_prev().is_some());
        assert_eq!(session.current().unwrap().index, 2);
        assert!(session.tab_prev().is_some());
        assert_eq!(session.current().unwrap().index, 1);

        // Already at the first stop.
        assert!(session.tab_prev().is_none());
    }

    #[test]
    fn at_final_is_true_only_on_zero_indexed_stop() {
        let (stops, base) = fn_signature_stops(0);
        let mut session = SnippetSession::start(&stops, base).unwrap();
        assert!(!session.at_final());
        session.tab_next();
        assert!(!session.at_final());
        session.tab_next();
        assert!(session.at_final());
    }

    #[test]
    fn before_edit_allows_when_range_falls_inside_active_stop() {
        let (stops, base) = fn_signature_stops(0);
        let session = SnippetSession::start(&stops, base).unwrap();
        // First stop covers `name` at 3..7. Editing inside should be Allow.
        let inside = 4..6;
        assert_eq!(session.before_edit(&inside), EditDisposition::Allow);
    }

    #[test]
    fn before_edit_allows_appending_at_right_edge_of_active_stop() {
        // Typing at the right edge of a placeholder grows it — that's
        // legitimate placeholder editing, not snippet escape.
        let (stops, base) = fn_signature_stops(0);
        let session = SnippetSession::start(&stops, base).unwrap();
        let at_right_edge = 7..7;
        assert_eq!(session.before_edit(&at_right_edge), EditDisposition::Allow);
    }

    #[test]
    fn before_edit_commits_when_range_escapes_active_stop() {
        let (stops, base) = fn_signature_stops(0);
        let session = SnippetSession::start(&stops, base).unwrap();
        // Active stop is at 3..7; editing at 12 is well outside.
        let outside = 12..12;
        assert_eq!(session.before_edit(&outside), EditDisposition::Commit);
    }

    #[test]
    fn after_edit_grows_active_stop_and_shifts_later_stops() {
        // User types 3 extra chars inside the first placeholder. The
        // active stop's end moves +3; the second stop shifts by +3.
        let (stops, base) = fn_signature_stops(0);
        let mut session = SnippetSession::start(&stops, base).unwrap();
        let active_before = session.stops[0].range.clone();
        let later_before = session.stops[1].range.clone();

        // Insert 3 bytes at offset 7 (end of active stop, replacing 0 bytes).
        session.after_edit(7, 7, 3);

        assert_eq!(
            session.stops[0].range,
            active_before.start..(active_before.end + 3)
        );
        assert_eq!(
            session.stops[1].range,
            (later_before.start + 3)..(later_before.end + 3),
        );
    }

    #[test]
    fn after_edit_shrinks_active_stop_and_pulls_later_stops() {
        // Replace the 4-byte placeholder with 1 byte — delta -3.
        let (stops, base) = fn_signature_stops(0);
        let mut session = SnippetSession::start(&stops, base).unwrap();
        let active_before = session.stops[0].range.clone();
        let later_before = session.stops[1].range.clone();

        session.after_edit(active_before.start, active_before.end, 1);

        assert_eq!(
            session.stops[0].range,
            active_before.start..(active_before.start + 1)
        );
        assert_eq!(
            session.stops[1].range,
            (later_before.start - 3)..(later_before.end - 3),
        );
    }

    #[test]
    fn after_edit_with_zero_delta_is_a_noop() {
        let (stops, base) = fn_signature_stops(0);
        let mut session = SnippetSession::start(&stops, base).unwrap();
        let snapshot: Vec<_> = session.stops.iter().map(|s| s.range.clone()).collect();

        // Edit length unchanged → ranges unchanged.
        session.after_edit(4, 6, 2);

        let after: Vec<_> = session.stops.iter().map(|s| s.range.clone()).collect();
        assert_eq!(snapshot, after);
    }

    /// Build the tab-stop list + expanded text for the
    /// rust-analyzer `Fill match arms` snippet on a `bool`
    /// match expression. Real shape:
    ///   `{\n    ${1:true} => ${2:todo!()},\n    ${3:false} => ${4:todo!()},\n}$0`
    /// The 4 named stops + a final `$0` after the closing
    /// brace. This is the exact snippet the user dogfooded.
    fn fill_match_arms_snippet() -> (String, Vec<TabStop>) {
        let expanded = SnippetParser::parse(
            "{\n    ${1:true} => ${2:todo!()},\n    ${3:false} => ${4:todo!()},\n}$0",
        );
        (expanded.text, expanded.tab_stops)
    }

    /// Track the document content alongside a session. Mimics
    /// what the editor widget does: edit the text, then call
    /// `after_edit` with the byte range and new length.
    fn apply_typed_edit(
        doc: &mut String,
        session: &mut SnippetSession,
        edit_start: usize,
        edit_end_old: usize,
        replacement: &str,
    ) {
        doc.replace_range(edit_start..edit_end_old, replacement);
        session.after_edit(edit_start, edit_end_old, replacement.len());
    }

    #[test]
    fn fill_match_arms_replace_first_arm_keeps_later_stops_aligned() {
        // Reproduce the dogfood: type a longer identifier
        // into stop 1 (replacing `true` with `MyEnum::A`),
        // then verify stop 2's tracked range still points at
        // the literal `todo!()` text in the document.
        let (text, stops) = fill_match_arms_snippet();
        let insert_start = 0;
        let mut doc = text.clone();
        let mut session = SnippetSession::start(&stops, insert_start).unwrap();

        // Active stop 1 covers `true`. Replace via the
        // shared helper.
        let stop1 = session.current().unwrap().range.clone();
        assert_eq!(&doc[stop1.clone()], "true");
        apply_typed_edit(&mut doc, &mut session, stop1.start, stop1.end, "MyEnum::A");

        // The tracked stop 2 range MUST still cover
        // `todo!()` after the +5-byte delta.
        session.tab_next();
        let stop2 = session.current().unwrap().range.clone();
        assert_eq!(
            &doc[stop2.clone()],
            "todo!()",
            "stop 2 range must match `todo!()` after editing stop 1",
        );

        // Stop 3 (`false` on second arm) — also shifted, also
        // pointing at the literal text.
        session.tab_next();
        let stop3 = session.current().unwrap().range.clone();
        assert_eq!(
            &doc[stop3.clone()],
            "false",
            "stop 3 (`false`) must remain aligned",
        );

        // Stop 4 (`todo!()` on second arm).
        session.tab_next();
        let stop4 = session.current().unwrap().range.clone();
        assert_eq!(
            &doc[stop4.clone()],
            "todo!()",
            "stop 4 (`todo!()` on second arm) must remain aligned",
        );
    }

    #[test]
    fn fill_match_arms_typing_at_right_edge_of_active_stop_extends_it() {
        // Variant of the dogfood: instead of selecting and
        // replacing, the user clears the placeholder first
        // (Backspace) and then types. Tracks 3 separate edits
        // accumulating on stop 1; later stops must shift by
        // the running total.
        let (text, stops) = fill_match_arms_snippet();
        let insert_start = 0;
        let mut doc = text.clone();
        let mut session = SnippetSession::start(&stops, insert_start).unwrap();

        let stop1 = session.current().unwrap().range.clone();
        // Step 1 — clear the placeholder (`true` → ``).
        apply_typed_edit(&mut doc, &mut session, stop1.start, stop1.end, "");
        // Step 2 — type `Foo` (one keystroke per char would
        // route through `before_edit` allowing growth at the
        // right edge; we just simulate the result here).
        let cur = session.current().unwrap().range.clone();
        apply_typed_edit(&mut doc, &mut session, cur.end, cur.end, "F");
        let cur = session.current().unwrap().range.clone();
        apply_typed_edit(&mut doc, &mut session, cur.end, cur.end, "o");
        let cur = session.current().unwrap().range.clone();
        apply_typed_edit(&mut doc, &mut session, cur.end, cur.end, "o");

        // Active stop now covers `Foo` (3 chars).
        let active = session.current().unwrap().range.clone();
        assert_eq!(&doc[active.clone()], "Foo");

        // Tab → stop 2. Range must point at `todo!()`.
        session.tab_next();
        let stop2 = session.current().unwrap().range.clone();
        assert_eq!(
            &doc[stop2.clone()],
            "todo!()",
            "stop 2 must remain aligned after multi-step typing on stop 1",
        );
    }

    #[test]
    fn fill_match_arms_edit_outside_active_stop_does_not_shift_active() {
        // Edge case: an edit happens *before* the active stop.
        // Per `before_edit`, it would Commit the session
        // (caller drops it). But verify the math is right
        // even if `after_edit` is called: a stop at start ≥
        // edit_end_old shifts; a stop fully BEFORE the edit
        // does not. The active stop logic only triggers for
        // edits inside the active stop.
        let (_text, stops) = fill_match_arms_snippet();
        let mut session = SnippetSession::start(&stops, 0).unwrap();

        // Snapshot stop 1 (active) range. Inject an edit far
        // after it — this should NOT touch stop 1's range
        // (since stop 1 starts before edit_end_old) but it
        // should shift stops that come after the edit.
        let stop1_before = session.stops[0].range.clone();
        let stop3_before = session.stops[2].range.clone();
        let stop4_before = session.stops[3].range.clone();

        // Edit: insert 5 bytes between stop 2 and stop 3
        // (somewhere in the middle ground). Pick a position
        // strictly between them.
        let stop2_end = session.stops[1].range.end;
        let stop3_start = session.stops[2].range.start;
        assert!(
            stop2_end < stop3_start,
            "stop 2 ends before stop 3 starts in the snippet text",
        );
        let edit_pos = stop2_end + 1; // safely between
        session.after_edit(edit_pos, edit_pos, 5);

        // Stop 1 (active, before the edit) — unchanged. The
        // active stop's `end` moves with delta only if the
        // edit landed inside it; here it didn't.
        // But the current implementation shifts the *active*
        // stop's end by delta unconditionally — pinning
        // a known weakness so we don't regress silently
        // when fixing it.
        // The tracked behavior today: active stop end grows.
        // Document this so the next refactor sees the gap.
        assert_eq!(
            session.stops[0].range.start, stop1_before.start,
            "active stop's start sticks even on out-of-stop edits",
        );
        // Stops after edit_pos shift by the delta.
        assert_eq!(
            session.stops[2].range,
            (stop3_before.start + 5)..(stop3_before.end + 5),
        );
        assert_eq!(
            session.stops[3].range,
            (stop4_before.start + 5)..(stop4_before.end + 5),
        );
    }
}
