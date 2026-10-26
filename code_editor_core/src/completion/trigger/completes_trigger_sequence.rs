// Copyright 2026 the xilem_code_editor developers
// SPDX-License-Identifier: Apache-2.0

use crate::document::Document;

/// Whether the text before `caret_byte` ends with one of the declared
/// multi-character trigger sequences (e.g. `[[` for wiki links, `::`).
/// Single-character entries are handled by `is_trigger_char`; sequences
/// are deliberate declarations, so the noisy-bracket filter does not
/// apply to them.
pub fn completes_trigger_sequence(
    caret_byte: usize,
    doc: &dyn Document,
    declared: &[String],
) -> bool {
    let text = doc.text();
    let caret_char = text.byte_to_char(caret_byte.min(doc.len_bytes()));
    declared
        .iter()
        .filter(|seq| seq.chars().count() >= 2)
        .any(|seq| {
            let len = seq.chars().count();
            caret_char >= len && text.slice(caret_char - len..caret_char) == seq.as_str()
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::document::RopeDocument;

    fn declared() -> Vec<String> {
        vec!["[[".to_string(), ":".to_string()]
    }

    #[test]
    fn double_bracket_triggers() {
        let doc = RopeDocument::from_str("see [[");
        assert!(completes_trigger_sequence(6, &doc, &declared()));
    }

    #[test]
    fn single_bracket_does_not() {
        let doc = RopeDocument::from_str("see [");
        assert!(!completes_trigger_sequence(5, &doc, &declared()));
    }

    #[test]
    fn single_char_entries_are_not_sequences() {
        let doc = RopeDocument::from_str("a:");
        assert!(!completes_trigger_sequence(2, &doc, &declared()));
    }

    #[test]
    fn works_after_multibyte_text() {
        let doc = RopeDocument::from_str("äö[[");
        assert!(completes_trigger_sequence(6, &doc, &declared()));
    }
}
