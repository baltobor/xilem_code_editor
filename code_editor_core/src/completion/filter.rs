// Copyright 2026 the xilem_code_editor developers
// SPDX-License-Identifier: Apache-2.0

use super::CompletionItem;

/// Indices of `items` whose filter key matches `query`, best first:
/// prefix matches before substring matches, each group in original order.
/// Case-insensitive. An empty query matches everything.
pub fn filter_completions(items: &[CompletionItem], query: &str) -> Vec<usize> {
    let query = query.to_lowercase();
    let mut prefix = Vec::new();
    let mut substring = Vec::new();
    for (i, item) in items.iter().enumerate() {
        let key = item.filter_key().to_lowercase();
        if key.starts_with(&query) {
            prefix.push(i);
        } else if key.contains(&query) {
            substring.push(i);
        }
    }
    prefix.extend(substring);
    prefix
}

#[cfg(test)]
mod tests {
    use super::*;

    fn item(label: &str) -> CompletionItem {
        CompletionItem {
            label: label.into(),
            ..Default::default()
        }
    }

    #[test]
    fn prefix_before_substring() {
        let items = [item("unwrap_or"), item("map"), item("unwrap")];
        assert_eq!(filter_completions(&items, "wrap"), vec![0, 2]);
        assert_eq!(filter_completions(&items, "UNW"), vec![0, 2]);
        assert_eq!(filter_completions(&items, "ma"), vec![1]);
    }
}
