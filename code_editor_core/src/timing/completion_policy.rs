// Copyright 2026 the xilem_code_editor developers
// SPDX-License-Identifier: Apache-2.0

/// What to do with an open completion popup after the query changed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompletionStep {
    /// Filter the items already received (the list was complete and the
    /// new query extends the old one).
    Refilter,
    /// Ask the provider again.
    Request,
    /// Close the popup.
    Close,
}

/// zed's policy: no debounce; re-filter client-side while the previous
/// response was complete and the query only grew; re-request otherwise;
/// close on an empty query.
pub fn next_completion_step(
    previous_query: &str,
    query: &str,
    is_incomplete: bool,
) -> CompletionStep {
    if query.is_empty() {
        CompletionStep::Close
    } else if !is_incomplete && query.starts_with(previous_query) {
        CompletionStep::Refilter
    } else {
        CompletionStep::Request
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn growing_query_on_complete_list_refilters() {
        assert_eq!(
            next_completion_step("un", "unw", false),
            CompletionStep::Refilter
        );
    }

    #[test]
    fn incomplete_list_re_requests() {
        assert_eq!(
            next_completion_step("un", "unw", true),
            CompletionStep::Request
        );
    }

    #[test]
    fn backspace_re_requests_and_empty_closes() {
        assert_eq!(
            next_completion_step("unw", "un", false),
            CompletionStep::Request
        );
        assert_eq!(next_completion_step("u", "", false), CompletionStep::Close);
    }
}
