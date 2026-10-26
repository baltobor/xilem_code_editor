// Copyright 2026 the xilem_code_editor developers
// SPDX-License-Identifier: Apache-2.0

//! Regex-based highlighter for certificate / PEM files.
//!
//! Side-channel highlighter — there's no published tree-sitter grammar
//! for PEM or `openssl x509 -text` output, and writing one for cosmetic
//! highlighting would be heavy. The patterns here cover the markers
//! and field labels that actually carry meaning when reading a cert
//! dump:
//!
//! - PEM armour: `-----BEGIN …-----` / `-----END …-----`
//! - `x509 -text` field labels: `Certificate:`, `Data:`, `Version:`,
//!   `Serial Number:`, `Issuer:`, `Validity`, `Subject:`,
//!   `Subject Public Key Info:`, `Public Key Algorithm:`,
//!   `X509v3 extensions:`, `Signature Algorithm:`, `Signature Value:`,
//!   `Not Before:`, `Not After :`, etc.
//! - DN attribute names: `CN`, `O`, `OU`, `C`, `ST`, `L`, `emailAddress`
//! - Hex byte runs: `aa:bb:cc:dd:…` and bare hex pairs
//! - Decimal numbers and `(0xN)` annotations
//!
//! Output shape matches the tree-sitter highlighter: per-line vectors
//! of `HighlightSpan` whose byte ranges are line-relative. The centre
//! editor consumes the same shape regardless of which highlighter
//! produced the spans.

use std::ops::Range;
use std::sync::OnceLock;

use regex::Regex;

use code_editor_core::highlight::{HighlightCategory, HighlightSpan};

/// One regex pattern + category, plus an optional capture-group
/// index. The Rust `regex` crate is linear-time and therefore
/// doesn't support lookahead, so DN-attribute rules that need
/// "name followed by `=`" use a capture group on the name and
/// match-and-discard everything after it.
struct Rule {
    re: Regex,
    cat: HighlightCategory,
    /// Capture-group index whose byte range becomes the span.
    /// `0` (the default) means the whole match.
    capture: usize,
}

struct Patterns {
    rules: Vec<Rule>,
}

fn patterns() -> &'static Patterns {
    static CACHED: OnceLock<Patterns> = OnceLock::new();
    CACHED.get_or_init(|| {
        let raw: Vec<(&'static str, HighlightCategory, usize)> = vec![
            // PEM armour markers — render as keyword so they pop
            // out at the top/bottom of each block.
            (
                r"^-----(?:BEGIN|END)\s+[A-Z0-9 ]+-----$",
                HighlightCategory::Keyword,
                0,
            ),
            // OpenSSL `x509 -text` block headers (top-level, no
            // indent or up to ~16 chars of indent). The capture
            // includes the trailing colon so the colon paints with
            // the label colour.
            (
                r"^\s{0,16}(?:Certificate|Data|Issuer|Validity|Subject|Subject Public Key Info|Public Key Algorithm|Public-Key|X509v3 extensions|Signature Algorithm|Signature Value|RSA Public-Key|Modulus|Exponent|Authority Information Access|CA Issuers|OCSP|CRL Distribution Points|Full Name|URI|Authority Key Identifier|Subject Key Identifier|Key Usage|Extended Key Usage|Basic Constraints|Subject Alternative Name|DNS|IP Address|email|Not Before|Not After ?|Version|Serial Number|keyid|DirName)\s*:",
                HighlightCategory::Type,
                0,
            ),
            // Distinguished-name attributes inside `Issuer: …` /
            // `Subject: …` lines: `CN = example.com`. Capture
            // group 1 is the attribute name; the trailing
            // whitespace + `=` is matched so we know the context
            // is right but doesn't get painted.
            (
                r"\b(CN|OU|O|C|ST|L|emailAddress|serialNumber|street|postalCode|businessCategory|jurisdictionC|jurisdictionST|jurisdictionL)\s*=",
                HighlightCategory::Property,
                1,
            ),
            // Hex byte runs joined by colons (e.g. fingerprints,
            // serial number hex output, modulus pretty-print). Two
            // or more pairs to avoid eating bare two-character
            // identifiers.
            (
                r"\b[0-9a-fA-F]{2}(?::[0-9a-fA-F]{2}){1,}\b",
                HighlightCategory::String,
                0,
            ),
            // Parenthesised hex annotations like `(0x2)` or
            // `(0xFFFFFFFF)`.
            (r"\(0x[0-9a-fA-F]+\)", HighlightCategory::Number, 0),
            // Bare decimal integers — dates and times have a
            // looser shape so we leave them to the field-label
            // colour.
            (r"\b\d+\b", HighlightCategory::Number, 0),
        ];

        let rules = raw
            .into_iter()
            .filter_map(|(pat, cat, capture)| {
                Regex::new(pat).ok().map(|re| Rule { re, cat, capture })
            })
            .collect();
        Patterns { rules }
    })
}

/// Highlight `source` and return per-line `HighlightSpan` vectors.
///
/// The vector length equals `source.lines().count().max(1)` to match
/// the tree-sitter highlighter's contract.
pub fn certificate_highlights(source: &str) -> Vec<Vec<HighlightSpan>> {
    let pats = patterns();

    let mut per_line: Vec<Vec<HighlightSpan>> = Vec::new();
    for line in source.split('\n') {
        let mut spans: Vec<HighlightSpan> = Vec::new();
        for rule in &pats.rules {
            if rule.capture == 0 {
                for m in rule.re.find_iter(line) {
                    spans.push(HighlightSpan {
                        byte_range: m.start()..m.end(),
                        category: rule.cat,
                    });
                }
            } else {
                for caps in rule.re.captures_iter(line) {
                    if let Some(m) = caps.get(rule.capture) {
                        spans.push(HighlightSpan {
                            byte_range: m.start()..m.end(),
                            category: rule.cat,
                        });
                    }
                }
            }
        }
        // Earlier patterns win — sort by start, then drop any
        // later span whose range overlaps an already-kept one.
        let merged = merge_disjoint(spans);
        per_line.push(merged);
    }
    if per_line.is_empty() {
        per_line.push(Vec::new());
    }
    per_line
}

/// Drop overlapping spans, keeping the earlier (lower-start) span.
///
/// The certificate patterns are deliberately conservative — most
/// matches don't overlap — but the date / hex patterns can collide
/// at the edges (e.g. `2025` inside an ISO-ish timestamp). Earlier
/// patterns are stronger signals (a field label vs a stray decimal),
/// so we keep them.
fn merge_disjoint(mut spans: Vec<HighlightSpan>) -> Vec<HighlightSpan> {
    if spans.is_empty() {
        return spans;
    }
    spans.sort_by_key(|s| (s.byte_range.start, s.byte_range.end));
    let mut out: Vec<HighlightSpan> = Vec::with_capacity(spans.len());
    for span in spans {
        let overlaps = out
            .last()
            .map(|last| ranges_overlap(&last.byte_range, &span.byte_range))
            .unwrap_or(false);
        if !overlaps {
            out.push(span);
        }
    }
    out
}

fn ranges_overlap(a: &Range<usize>, b: &Range<usize>) -> bool {
    a.start < b.end && b.start < a.end
}

/// [`certificate_highlights`] with document byte ranges instead of
/// line-relative ones.
pub fn certificate_spans(source: &str) -> Vec<HighlightSpan> {
    let mut line_start = 0;
    let mut flat = Vec::new();
    for (line, spans) in source.split('\n').zip(certificate_highlights(source)) {
        flat.extend(spans.into_iter().map(|s| HighlightSpan {
            byte_range: s.byte_range.start + line_start..s.byte_range.end + line_start,
            category: s.category,
        }));
        line_start += line.len() + 1;
    }
    flat
}

#[cfg(test)]
mod tests {
    use super::*;

    fn line_categories(spans: &[HighlightSpan]) -> Vec<HighlightCategory> {
        spans.iter().map(|s| s.category).collect()
    }

    #[test]
    fn pem_markers_highlight_as_keyword() {
        let src = "-----BEGIN CERTIFICATE-----\nMIIDazCCAlOgAwIBAgIUF\n-----END CERTIFICATE-----\n";
        let spans = certificate_highlights(src);
        assert!(line_categories(&spans[0]).contains(&HighlightCategory::Keyword));
        assert!(line_categories(&spans[2]).contains(&HighlightCategory::Keyword));
    }

    #[test]
    fn x509_text_field_labels_highlight_as_type() {
        let src = "Certificate:\n    Data:\n        Version: 3 (0x2)\n        Serial Number:\n";
        let spans = certificate_highlights(src);
        assert!(line_categories(&spans[0]).contains(&HighlightCategory::Type));
        assert!(line_categories(&spans[1]).contains(&HighlightCategory::Type));
        assert!(line_categories(&spans[2]).contains(&HighlightCategory::Type));
        assert!(line_categories(&spans[3]).contains(&HighlightCategory::Type));
    }

    #[test]
    fn dn_attribute_names_highlight_as_property() {
        let src = "        Issuer: C = US, O = Example Inc, CN = example.com\n";
        let spans = certificate_highlights(src);
        let cats = line_categories(&spans[0]);
        assert!(cats.contains(&HighlightCategory::Property));
        // At least three property spans (C, O, CN).
        let prop_count = spans[0]
            .iter()
            .filter(|s| s.category == HighlightCategory::Property)
            .count();
        assert!(
            prop_count >= 3,
            "expected ≥3 DN attributes, got {prop_count}: {:?}",
            spans[0]
        );
    }

    #[test]
    fn hex_byte_runs_highlight_as_string() {
        let src = "                aa:bb:cc:dd:ee:ff\n";
        let spans = certificate_highlights(src);
        assert!(line_categories(&spans[0]).contains(&HighlightCategory::String));
    }

    #[test]
    fn parenthesised_hex_highlights_as_number() {
        let src = "        Version: 3 (0x2)\n";
        let spans = certificate_highlights(src);
        assert!(line_categories(&spans[0]).contains(&HighlightCategory::Number));
    }

    #[test]
    fn empty_source_returns_one_empty_line() {
        let spans = certificate_highlights("");
        assert_eq!(spans.len(), 1);
        assert!(spans[0].is_empty());
    }

    #[test]
    fn merge_disjoint_keeps_earlier_pattern_on_overlap() {
        let a = HighlightSpan {
            byte_range: 0..10,
            category: HighlightCategory::Keyword,
        };
        let b = HighlightSpan {
            byte_range: 5..15,
            category: HighlightCategory::Number,
        };
        let merged = merge_disjoint(vec![a.clone(), b]);
        assert_eq!(merged.len(), 1);
        assert_eq!(merged[0], a);
    }
}
