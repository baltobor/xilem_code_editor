// Copyright 2026 the xilem_code_editor developers
// SPDX-License-Identifier: Apache-2.0

use super::{DeletionMark, DiffLine, DiffLineKind, LineMark, VcsGutterDiff};

impl VcsGutterDiff {
    /// Reduce diff hunks (HEAD vs the live buffer) into gutter markers:
    /// added/modified lines get a [`LineMark`], each run of removed lines
    /// becomes a [`DeletionMark`] anchored after the new-side line it follows.
    pub fn from_hunks<H: AsRef<[DiffLine]>>(hunks: &[H]) -> VcsGutterDiff {
        let mut line_marks = Vec::new();
        let mut deletions = Vec::new();

        for hunk in hunks {
            // A hunk that removes *and* adds is a modification; its added lines
            // read as Modified rather than Added.
            let has_deletion = hunk
                .as_ref()
                .iter()
                .any(|l| l.kind == DiffLineKind::Deletion);

            let mut anchor: usize = 0; // last new-side line seen
            let mut run_anchor: usize = 0; // new line before the current deletion run
            let mut run_text = String::new();
            let mut in_run = false;

            let flush = |deletions: &mut Vec<DeletionMark>, anchor: usize, text: &mut String| {
                deletions.push(DeletionMark {
                    after_new_line: anchor,
                    removed_text: std::mem::take(text),
                });
            };

            for line in hunk.as_ref() {
                match line.kind {
                    DiffLineKind::Addition => {
                        if in_run {
                            flush(&mut deletions, run_anchor, &mut run_text);
                            in_run = false;
                        }
                        if let Some(n) = line.new_line {
                            anchor = n;
                            line_marks.push((
                                n,
                                if has_deletion {
                                    LineMark::Modified
                                } else {
                                    LineMark::Added
                                },
                            ));
                        }
                    }
                    DiffLineKind::Context => {
                        if in_run {
                            flush(&mut deletions, run_anchor, &mut run_text);
                            in_run = false;
                        }
                        if let Some(n) = line.new_line {
                            anchor = n;
                        }
                    }
                    DiffLineKind::Deletion => {
                        if !in_run {
                            in_run = true;
                            run_anchor = anchor;
                            run_text.clear();
                        }
                        run_text.push_str(&line.content);
                    }
                }
            }
            if in_run {
                flush(&mut deletions, run_anchor, &mut run_text);
            }
        }

        VcsGutterDiff {
            line_marks,
            deletions,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn line(
        kind: DiffLineKind,
        content: &str,
        _old: Option<usize>,
        new: Option<usize>,
    ) -> DiffLine {
        DiffLine {
            kind,
            content: content.to_string(),
            new_line: new,
        }
    }

    fn diff(lines: Vec<DiffLine>) -> VcsGutterDiff {
        VcsGutterDiff::from_hunks(&[lines])
    }

    #[test]
    fn pure_additions_are_green_added() {
        // context line 1, then two added lines 2,3
        let d = diff(vec![
            line(DiffLineKind::Context, "a\n", Some(1), Some(1)),
            line(DiffLineKind::Addition, "b\n", None, Some(2)),
            line(DiffLineKind::Addition, "c\n", None, Some(3)),
        ]);
        let v = d;
        assert_eq!(
            v.line_marks,
            vec![(2, LineMark::Added), (3, LineMark::Added)]
        );
        assert!(v.deletions.is_empty());
    }

    #[test]
    fn pure_deletion_anchors_after_preceding_line() {
        // context line 1, remove old lines (gap), context resumes at new line 2
        let d = diff(vec![
            line(DiffLineKind::Context, "a\n", Some(1), Some(1)),
            line(DiffLineKind::Deletion, "gone1\n", Some(2), None),
            line(DiffLineKind::Deletion, "gone2\n", Some(3), None),
            line(DiffLineKind::Context, "b\n", Some(4), Some(2)),
        ]);
        let v = d;
        assert!(v.line_marks.is_empty());
        assert_eq!(v.deletions.len(), 1);
        assert_eq!(v.deletions[0].after_new_line, 1);
        assert_eq!(v.deletions[0].removed_text, "gone1\ngone2\n");
    }

    #[test]
    fn modification_is_green_modified_plus_deletion_mark() {
        // line 1 changed: remove old 1, add new 1
        let d = diff(vec![
            line(DiffLineKind::Deletion, "old\n", Some(1), None),
            line(DiffLineKind::Addition, "new\n", None, Some(1)),
        ]);
        let v = d;
        assert_eq!(v.line_marks, vec![(1, LineMark::Modified)]);
        assert_eq!(v.deletions.len(), 1);
        assert_eq!(v.deletions[0].after_new_line, 0); // removed before any surviving line
        assert_eq!(v.deletions[0].removed_text, "old\n");
    }
}
