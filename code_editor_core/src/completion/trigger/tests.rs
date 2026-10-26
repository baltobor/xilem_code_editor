// Copyright 2026 the xilem_code_editor developers
// SPDX-License-Identifier: Apache-2.0

//! Tests for the completion-trigger helpers.

use super::identifier_run_length::identifier_run_length_ending_at;
use super::is_identifier_char::is_identifier_char;
use super::is_trigger_char::is_trigger_char;
use super::should_auto_trigger::should_auto_trigger;
use crate::document::RopeDocument;

#[test]
fn ascii_letters_and_digits_are_identifier_chars() {
    assert!(is_identifier_char('a'));
    assert!(is_identifier_char('Z'));
    assert!(is_identifier_char('0'));
    assert!(is_identifier_char('9'));
}

#[test]
fn underscore_is_an_identifier_char() {
    assert!(is_identifier_char('_'));
}

#[test]
fn punctuation_and_whitespace_are_not_identifier_chars() {
    for ch in [' ', '.', '(', ')', '{', '}', ',', ';', '\n', '\t', '!'] {
        assert!(
            !is_identifier_char(ch),
            "expected `{ch}` not to be an identifier char",
        );
    }
}

#[test]
fn unicode_letters_count_as_identifier_chars() {
    // Common Latin / Greek / CJK letters typists actually use.
    assert!(is_identifier_char('é'));
    assert!(is_identifier_char('日'));
}

#[test]
fn trigger_char_matches_declared_chars() {
    // `.` is the canonical "method on receiver" trigger.
    assert!(is_trigger_char('.', &[".".to_string(), ":".to_string()]));
}

#[test]
fn trigger_char_skips_noisy_brackets_even_when_declared() {
    // Curly braces are on the list too: typing `match l {` should
    // land the caret in an empty block, not refresh the popup with
    // whatever was suggested for `match l`.
    for ch in ['(', ')', '[', ']', '<', '>', '\'', '"', '{', '}'] {
        assert!(
            !is_trigger_char(ch, &[ch.to_string()]),
            "noisy bracket `{ch}` should be filtered out",
        );
    }
}

#[test]
fn trigger_char_returns_false_when_not_declared() {
    assert!(!is_trigger_char('a', &[".".to_string()]));
}

#[test]
fn auto_trigger_fires_when_popup_visible_and_char_is_identifier() {
    let doc = RopeDocument::from_str("");
    assert!(should_auto_trigger('a', 1, /* popup */ true, &doc));
}

#[test]
fn auto_trigger_does_not_fire_for_non_identifier_chars() {
    let doc = RopeDocument::from_str("foo");
    assert!(!should_auto_trigger('.', 4, true, &doc));
    assert!(!should_auto_trigger(' ', 4, true, &doc));
}

#[test]
fn auto_trigger_fires_at_three_char_prefix_when_popup_hidden() {
    // Doc "foo" — caret at offset 3, the user just typed `o`.
    // Identifier run length is 3 (>= the AUTO_TRIGGER_MIN_PREFIX
    // threshold) → fire. IntelliJ-style.
    let doc = RopeDocument::from_str("foo");
    assert!(should_auto_trigger('o', 3, /* popup */ false, &doc));
}

#[test]
fn auto_trigger_keeps_firing_as_user_extends_a_long_identifier() {
    let doc = RopeDocument::from_str("fooo");
    assert!(should_auto_trigger('o', 4, /* popup */ false, &doc));
}

#[test]
fn auto_trigger_does_not_fire_on_single_char_when_popup_hidden() {
    // Single identifier char must NOT open the popup unless
    // the user explicitly asks (Ctrl+Space).
    let doc = RopeDocument::from_str(" a");
    assert!(!should_auto_trigger('a', 2, /* popup */ false, &doc));
}

#[test]
fn auto_trigger_does_not_fire_on_two_char_prefix_when_popup_hidden() {
    // Two letters still stay quiet — common short keywords
    // (`if`, `fn`, `pub`) shouldn't trigger a server round-
    // trip the user is about to throw away.
    let doc = RopeDocument::from_str("fo");
    assert!(!should_auto_trigger('o', 2, /* popup */ false, &doc));
}

#[test]
fn auto_trigger_does_not_fire_at_document_start() {
    let doc = RopeDocument::from_str("a");
    assert!(!should_auto_trigger('a', 1, /* popup */ false, &doc));
}

#[test]
fn identifier_run_length_counts_only_contiguous_id_chars() {
    let doc = RopeDocument::from_str(" foo");
    assert_eq!(identifier_run_length_ending_at(4, &doc), 3);
}

#[test]
fn identifier_run_length_stops_at_punctuation() {
    let doc = RopeDocument::from_str("foo.bar");
    assert_eq!(identifier_run_length_ending_at(4, &doc), 0);
}

#[test]
fn identifier_run_length_at_document_start_is_zero() {
    let doc = RopeDocument::from_str("foo");
    assert_eq!(identifier_run_length_ending_at(0, &doc), 0);
}
