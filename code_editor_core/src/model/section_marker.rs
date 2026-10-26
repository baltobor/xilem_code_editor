// Copyright 2026 the xilem_code_editor developers
// SPDX-License-Identifier: Apache-2.0

//! Section markers parsed from source comments.
//!
//! Recognizes Xcode/RustRover-style `MARK:` and dash-rule comment forms
//! and turns them into a list of `SectionMarker`s the editor paints as
//! visible section breaks (horizontal rule + bold name).
//!
//! ## Recognized syntax (all single-line comments)
//!
//! - `// ---` (three or more dashes, no name)            → separator only
//! - `// --- MARK: <name>`                               → separator + name
//! - `// --- MARK: -`                                    → separator + no name
//! - `// MARK: <name>`                                   → name only
//! - `// MARK: -`                                        → separator + no name
//! - `// MARK: - <name>`                                 → separator + name
//!
//! Leading whitespace before `//` and a single space between `//` and
//! the keyword are tolerated. Anything that doesn't match leaves the
//! line untouched (the editor still renders it as a normal comment).
//!
//! ## SOLID notes
//!
//! The parser is intentionally **language-agnostic**: it operates on
//! `&str` (whole document text) and emits a flat `Vec<SectionMarker>`.
//! No editor / UI / LSP types appear here, so a future Python-plugin
//! crate (or a TOML / Markdown variant) can reuse the same parser by
//! wrapping it with their own line-comment prefix detection if it
//! ever differs from `//`.

/// One parsed section marker.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SectionMarker {
    /// 0-based line number where the marker comment lives.
    pub line: usize,
    /// Section name, if the marker carried one. `None` for the bare
    /// `// ---` form and for `MARK: -` (the `-` is a "draw separator
    /// only" sentinel in Xcode).
    pub name: Option<String>,
    /// Whether the renderer should draw a horizontal rule for this
    /// marker. `// MARK: <name>` (no leading dash) is a "named
    /// section, no rule" form; everything else gets a rule.
    pub draw_separator: bool,
    /// Line-relative byte range covering the section name in the
    /// source line (i.e. the bytes of `name` in the original line
    /// text). `None` when there is no name. The editor injects an
    /// extra highlight span over this range so the name reads in a
    /// distinct color from the rest of the comment without the
    /// renderer having to overlay anything.
    pub name_range_in_line: Option<core::ops::Range<usize>>,
}

/// Scan a whole document and return every section marker it finds, in
/// document order. The caller is expected to call this whenever the
/// document changes (we currently call it from the editor's
/// `rehighlight()` hook).
///
/// Allocates one `Vec` for the result and one `String` per match (only
/// when the marker has a name); cost is ~O(line_count) with a small
/// constant. For million-line files this could be made faster by
/// scanning only changed lines, but the current rehighlight already
/// touches the whole document, so this is in line with that work.
pub fn parse_section_markers(source: &str) -> Vec<SectionMarker> {
    let mut markers = Vec::new();
    for (line_idx, raw_line) in source.lines().enumerate() {
        if let Some(marker) = parse_marker_line(line_idx, raw_line) {
            markers.push(marker);
        }
    }
    markers
}

/// Try to parse a single line. Returns `None` if it isn't a marker.
///
/// We thread a `base_offset: usize` that tracks how many bytes we've
/// already consumed from the *original* line, so when we eventually
/// locate the name we can return its byte range relative to the
/// original line (not relative to whatever sub-slice we trimmed).
fn parse_marker_line(line: usize, raw: &str) -> Option<SectionMarker> {
    let after_indent_offset = raw.len() - raw.trim_start().len();
    let trimmed = &raw[after_indent_offset..];
    let after_slashes = trimmed.strip_prefix("//")?;
    let after_slashes_offset = after_indent_offset + 2;

    // Allow zero or more spaces after the slashes.
    let body_offset =
        after_slashes_offset + (after_slashes.len() - after_slashes.trim_start().len());
    let body = &raw[body_offset..];

    // Form 1: starts with three+ dashes — `// ---`, `// --- MARK: ...`
    if let Some((dash_count, rest)) = strip_three_or_more_dashes(body) {
        let after_dashes_offset = body_offset + dash_count;
        let rest_trimmed_offset = after_dashes_offset + (rest.len() - rest.trim_start().len());
        let rest_trimmed = &raw[rest_trimmed_offset..];
        if rest_trimmed.is_empty() {
            // `// ---`
            return Some(SectionMarker {
                line,
                name: None,
                draw_separator: true,
                name_range_in_line: None,
            });
        }
        // After the dashes we expect `MARK:` (anything else is a normal comment)
        if rest_trimmed.starts_with("MARK:") {
            let after_mark_offset = rest_trimmed_offset + "MARK:".len();
            return Some(parse_mark_payload(
                line,
                raw,
                after_mark_offset,
                /*default_separator=*/ true,
            ));
        }
        // Dashes followed by something other than MARK: — not a marker.
        return None;
    }

    // Form 2: starts with `MARK:` — `// MARK: ...`
    if body.starts_with("MARK:") {
        let after_mark_offset = body_offset + "MARK:".len();
        return Some(parse_mark_payload(
            line,
            raw,
            after_mark_offset,
            /*default_separator=*/ false,
        ));
    }

    None
}

/// Parse the part *after* `MARK:`. `raw` is the full original line;
/// `payload_offset` is the byte index where `MARK:` ends in `raw`.
/// We look at the substring `raw[payload_offset..]` to figure out
/// what's there, but report the name range relative to `raw`.
fn parse_mark_payload(
    line: usize,
    raw: &str,
    payload_offset: usize,
    default_separator: bool,
) -> SectionMarker {
    let payload = &raw[payload_offset..];
    let leading_ws = payload.len() - payload.trim_start().len();
    let trimmed_offset = payload_offset + leading_ws;
    let payload_trimmed = &raw[trimmed_offset..];

    if payload_trimmed.is_empty() {
        // `// MARK:` with nothing after it — treat as anonymous.
        return SectionMarker {
            line,
            name: None,
            draw_separator: default_separator,
            name_range_in_line: None,
        };
    }

    if let Some(after_dash) = payload_trimmed.strip_prefix('-') {
        let after_dash_offset = trimmed_offset + 1;
        let name_leading_ws = after_dash.len() - after_dash.trim_start().len();
        let name_start = after_dash_offset + name_leading_ws;
        let name_text = &raw[name_start..];
        let name_trimmed = name_text.trim_end();
        if name_trimmed.is_empty() {
            // `MARK: -`  → separator only
            SectionMarker {
                line,
                name: None,
                draw_separator: true,
                name_range_in_line: None,
            }
        } else {
            // `MARK: - <name>`  → separator + name
            let name_end = name_start + name_trimmed.len();
            SectionMarker {
                line,
                name: Some(name_trimmed.to_string()),
                draw_separator: true,
                name_range_in_line: Some(name_start..name_end),
            }
        }
    } else {
        // `MARK: <name>`  → name only (separator from the form)
        let name_trimmed = payload_trimmed.trim_end();
        let name_end = trimmed_offset + name_trimmed.len();
        SectionMarker {
            line,
            name: Some(name_trimmed.to_string()),
            draw_separator: default_separator,
            name_range_in_line: Some(trimmed_offset..name_end),
        }
    }
}

/// Strip a leading run of 3+ dashes. Returns `None` if there are
/// fewer than three dashes; otherwise returns `(dash_count, rest)`
/// where `rest` is the remaining string after the dashes.
fn strip_three_or_more_dashes(s: &str) -> Option<(usize, &str)> {
    let mut bytes = s.as_bytes();
    let mut count = 0usize;
    while bytes.first() == Some(&b'-') {
        count += 1;
        bytes = &bytes[1..];
    }
    if count >= 3 {
        // SAFETY: we only advanced past ASCII '-' bytes, so the
        // remaining slice is still on a UTF-8 boundary.
        Some((count, std::str::from_utf8(bytes).expect("ASCII boundary")))
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse_one(line: &str) -> Option<SectionMarker> {
        parse_marker_line(0, line)
    }

    /// Helper: assert all the easy fields and additionally check that
    /// the name range, when present, slices `raw` back to the same
    /// name string. That validates the byte offsets without us having
    /// to hand-compute them in every test.
    fn assert_marker(raw: &str, expected_name: Option<&str>, draw_sep: bool) {
        let m = parse_marker_line(0, raw).expect("expected a marker");
        assert_eq!(m.line, 0);
        assert_eq!(m.name.as_deref(), expected_name);
        assert_eq!(m.draw_separator, draw_sep);
        match (&expected_name, &m.name_range_in_line) {
            (Some(name), Some(range)) => {
                assert_eq!(
                    &raw[range.clone()],
                    *name,
                    "name_range_in_line slice mismatch"
                );
            }
            (None, None) => {}
            _ => panic!("name presence does not match name_range presence"),
        }
    }

    #[test]
    fn anonymous_dashes() {
        assert_marker("// ---", None, true);
    }

    #[test]
    fn dashes_with_mark_name() {
        assert_marker("// --- MARK: BUILDERS", Some("BUILDERS"), true);
    }

    #[test]
    fn dashes_with_mark_dash_only() {
        assert_marker("// --- MARK: -", None, true);
    }

    #[test]
    fn mark_name_no_dash() {
        assert_marker("// MARK: Helpers", Some("Helpers"), false);
    }

    #[test]
    fn mark_dash_only() {
        assert_marker("// MARK: -", None, true);
    }

    #[test]
    fn mark_dash_plus_name() {
        assert_marker("// MARK: - Drawing", Some("Drawing"), true);
    }

    #[test]
    fn handles_indentation() {
        assert_marker("    // MARK: Indented", Some("Indented"), false);
    }

    #[test]
    fn many_dashes_still_match() {
        assert_marker("// ----- MARK: Long", Some("Long"), true);
    }

    #[test]
    fn two_dashes_is_not_a_marker() {
        // Only two dashes — looks like "//-- comment", not a section.
        assert!(parse_one("// --").is_none());
        assert!(parse_one("// -- MARK: X").is_none());
    }

    #[test]
    fn random_comment_is_not_a_marker() {
        assert!(parse_one("// just a regular comment").is_none());
        assert!(parse_one("// TODO: refactor").is_none());
    }

    #[test]
    fn name_range_survives_multibyte_indentation() {
        // The `name_range_in_line` is a byte range relative to the
        // *original* raw line. If the indentation contains non-ASCII
        // bytes (e.g. an em-space U+2003 used as decorative indent),
        // the byte offsets must still slice back to the name string —
        // never landing in the middle of a multi-byte codepoint.
        let raw = "\u{2003}// MARK: Indented";
        let m = parse_marker_line(0, raw).expect("expected a marker");
        let range = m.name_range_in_line.expect("name range present");
        assert_eq!(&raw[range], "Indented");
    }

    #[test]
    fn whole_document_returns_all_markers() {
        let src = "fn main() {}\n// --- MARK: TWO\nlet x = 1;\n// MARK: THREE\n";
        let markers = parse_section_markers(src);
        assert_eq!(markers.len(), 2);
        assert_eq!(markers[0].line, 1);
        assert_eq!(markers[0].name.as_deref(), Some("TWO"));
        assert_eq!(markers[1].line, 3);
        assert_eq!(markers[1].name.as_deref(), Some("THREE"));
    }
}
