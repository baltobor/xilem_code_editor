// Copyright 2026 the xilem_code_editor developers
// SPDX-License-Identifier: Apache-2.0

//! Snippet expansion for LSP completions.
//!
//! Handles VSCode-style snippet syntax used by language servers:
//! - `$0` - Final cursor position
//! - `$1`, `$2`, etc. - Tab stops
//! - `${1:placeholder}` - Tab stop with default text
//! - `${1|one,two,three|}` - Choice (rendered as first option)

/// A parsed tab stop from a snippet.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TabStop {
    /// Tab stop index (0 = final position, 1+ = navigation order).
    pub index: usize,
    /// Placeholder text (empty if none).
    pub placeholder: String,
    /// Byte offset in the expanded text where this tab stop begins.
    pub offset: usize,
    /// Length of the placeholder text.
    pub length: usize,
}

/// Result of parsing and expanding a snippet.
#[derive(Debug, Clone)]
pub struct ExpandedSnippet {
    /// The expanded text with placeholders substituted.
    pub text: String,
    /// Tab stops in navigation order (1, 2, 3, ..., 0).
    pub tab_stops: Vec<TabStop>,
    /// Index into tab_stops for current position.
    pub current_tab_stop: usize,
}

impl ExpandedSnippet {
    /// Get the current tab stop, if any.
    pub fn current(&self) -> Option<&TabStop> {
        self.tab_stops.get(self.current_tab_stop)
    }

    /// Move to the next tab stop. Returns true if moved, false if at end.
    pub fn next(&mut self) -> bool {
        if self.current_tab_stop + 1 < self.tab_stops.len() {
            self.current_tab_stop += 1;
            true
        } else {
            false
        }
    }

    /// Move to the previous tab stop. Returns true if moved.
    pub fn prev(&mut self) -> bool {
        if self.current_tab_stop > 0 {
            self.current_tab_stop -= 1;
            true
        } else {
            false
        }
    }

    /// Check if we're at the final tab stop ($0).
    pub fn is_final(&self) -> bool {
        self.current().map_or(true, |ts| ts.index == 0)
    }
}

/// Parses and expands snippet syntax.
pub struct SnippetParser;

impl SnippetParser {
    /// Parse a snippet string and expand it.
    ///
    /// Returns the expanded text and tab stop information.
    pub fn parse(snippet: &str) -> ExpandedSnippet {
        let mut result = String::new();
        let mut tab_stops: Vec<TabStop> = Vec::new();
        let mut chars = snippet.chars().peekable();

        while let Some(ch) = chars.next() {
            if ch == '$' {
                if let Some(&next) = chars.peek() {
                    if next == '{' {
                        // ${...} syntax
                        chars.next(); // consume '{'
                        Self::parse_braced_tabstop(&mut chars, &mut result, &mut tab_stops);
                    } else if next.is_ascii_digit() {
                        // $N syntax
                        Self::parse_simple_tabstop(&mut chars, &mut result, &mut tab_stops);
                    } else if next == '$' {
                        // $$ = escaped $
                        chars.next();
                        result.push('$');
                    } else {
                        result.push('$');
                    }
                } else {
                    result.push('$');
                }
            } else if ch == '\\' {
                // Escape sequences
                if let Some(&next) = chars.peek() {
                    if matches!(next, '$' | '}' | '\\') {
                        chars.next();
                        result.push(next);
                    } else {
                        result.push(ch);
                    }
                } else {
                    result.push(ch);
                }
            } else {
                result.push(ch);
            }
        }

        // Sort tab stops: 1, 2, 3, ..., then 0 at the end
        tab_stops.sort_by_key(|ts| if ts.index == 0 { usize::MAX } else { ts.index });

        // If no $0, add one at the end
        if !tab_stops.iter().any(|ts| ts.index == 0) {
            tab_stops.push(TabStop {
                index: 0,
                placeholder: String::new(),
                offset: result.len(),
                length: 0,
            });
        }

        ExpandedSnippet {
            text: result,
            tab_stops,
            current_tab_stop: 0,
        }
    }

    /// Parse a simple $N tab stop.
    fn parse_simple_tabstop(
        chars: &mut std::iter::Peekable<std::str::Chars>,
        result: &mut String,
        tab_stops: &mut Vec<TabStop>,
    ) {
        let mut num_str = String::new();
        while let Some(&ch) = chars.peek() {
            if ch.is_ascii_digit() {
                num_str.push(ch);
                chars.next();
            } else {
                break;
            }
        }

        if let Ok(index) = num_str.parse::<usize>() {
            tab_stops.push(TabStop {
                index,
                placeholder: String::new(),
                offset: result.len(),
                length: 0,
            });
        }
    }

    /// Parse a braced ${N} or ${N:placeholder} or ${N|choices|} tab stop.
    fn parse_braced_tabstop(
        chars: &mut std::iter::Peekable<std::str::Chars>,
        result: &mut String,
        tab_stops: &mut Vec<TabStop>,
    ) {
        // Read the tab stop number
        let mut num_str = String::new();
        while let Some(&ch) = chars.peek() {
            if ch.is_ascii_digit() {
                num_str.push(ch);
                chars.next();
            } else {
                break;
            }
        }

        let index = num_str.parse::<usize>().unwrap_or(0);
        let mut placeholder = String::new();

        // Check what comes after the number
        if let Some(&ch) = chars.peek() {
            if ch == ':' {
                // ${N:placeholder}
                chars.next(); // consume ':'
                let mut depth = 1;
                while let Some(ch) = chars.next() {
                    if ch == '}' {
                        depth -= 1;
                        if depth == 0 {
                            break;
                        }
                        placeholder.push(ch);
                    } else if ch == '{' {
                        depth += 1;
                        placeholder.push(ch);
                    } else if ch == '\\' {
                        // Handle escapes within placeholder
                        if let Some(&next) = chars.peek() {
                            if matches!(next, '$' | '}' | '\\') {
                                chars.next();
                                placeholder.push(next);
                            } else {
                                placeholder.push(ch);
                            }
                        } else {
                            placeholder.push(ch);
                        }
                    } else {
                        placeholder.push(ch);
                    }
                }
            } else if ch == '|' {
                // ${N|choice1,choice2,...|}
                chars.next(); // consume '|'
                let mut choices = String::new();
                while let Some(ch) = chars.next() {
                    if ch == '|' {
                        // Next should be '}'
                        if chars.peek() == Some(&'}') {
                            chars.next();
                        }
                        break;
                    } else {
                        choices.push(ch);
                    }
                }
                // Use first choice as placeholder
                if let Some(first) = choices.split(',').next() {
                    placeholder = first.to_string();
                }
            } else if ch == '}' {
                // ${N} - no placeholder
                chars.next();
            } else {
                // Malformed, consume until }
                while let Some(ch) = chars.next() {
                    if ch == '}' {
                        break;
                    }
                }
            }
        }

        // Handle nested snippets in placeholder by recursively parsing
        let expanded_placeholder = if placeholder.contains('$') {
            let nested = Self::parse(&placeholder);
            nested.text
        } else {
            placeholder
        };

        let offset = result.len();
        let length = expanded_placeholder.len();
        result.push_str(&expanded_placeholder);

        tab_stops.push(TabStop {
            index,
            placeholder: expanded_placeholder,
            offset,
            length,
        });
    }

    /// Strip snippet syntax and return plain text (for display in ghost text).
    ///
    /// This shows placeholders but removes the snippet markers.
    pub fn strip_to_display(snippet: &str) -> String {
        Self::parse(snippet).text
    }

    /// Check if a string contains snippet syntax.
    pub fn contains_snippet_syntax(text: &str) -> bool {
        let mut chars = text.chars().peekable();
        while let Some(ch) = chars.next() {
            if ch == '$' {
                if let Some(&next) = chars.peek() {
                    if next == '{' || next.is_ascii_digit() {
                        return true;
                    }
                }
            }
        }
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_tabstop() {
        let result = SnippetParser::parse("hello $1 world");
        assert_eq!(result.text, "hello  world");
        assert_eq!(result.tab_stops.len(), 2); // $1 and implicit $0
        assert_eq!(result.tab_stops[0].index, 1);
        assert_eq!(result.tab_stops[0].offset, 6);
    }

    #[test]
    fn test_placeholder() {
        let result = SnippetParser::parse("match ${1:value} { $0 }");
        assert_eq!(result.text, "match value {  }");
        assert_eq!(result.tab_stops[0].index, 1);
        assert_eq!(result.tab_stops[0].placeholder, "value");
        assert_eq!(result.tab_stops[0].length, 5);
    }

    #[test]
    fn test_choice() {
        let result = SnippetParser::parse("${1|true,false|}");
        assert_eq!(result.text, "true");
        assert_eq!(result.tab_stops[0].placeholder, "true");
    }

    #[test]
    fn test_multiple_tabstops() {
        let result = SnippetParser::parse("fn ${1:name}(${2:args}) { $0 }");
        assert_eq!(result.text, "fn name(args) {  }");
        // Tab stops should be in order: 1, 2, 0
        assert_eq!(result.tab_stops[0].index, 1);
        assert_eq!(result.tab_stops[1].index, 2);
        assert_eq!(result.tab_stops[2].index, 0);
    }

    #[test]
    fn test_contains_snippet_syntax() {
        assert!(SnippetParser::contains_snippet_syntax("hello $1"));
        assert!(SnippetParser::contains_snippet_syntax("${1:foo}"));
        assert!(!SnippetParser::contains_snippet_syntax("hello world"));
        // Note: "$100.00" is detected as containing snippet syntax because "$1" matches.
        // This is correct VSCode behavior - if you need literal "$", use "$$".
        assert!(SnippetParser::contains_snippet_syntax("$100.00"));
    }

    #[test]
    fn test_escaped_dollar() {
        let result = SnippetParser::parse("price: $$100");
        assert_eq!(result.text, "price: $100");
    }

    #[test]
    fn test_navigation() {
        let mut result = SnippetParser::parse("${1:a} ${2:b} $0");
        assert_eq!(result.current().unwrap().index, 1);
        assert!(result.next());
        assert_eq!(result.current().unwrap().index, 2);
        assert!(result.next());
        assert_eq!(result.current().unwrap().index, 0);
        assert!(result.is_final());
        assert!(!result.next());
    }

    #[test]
    fn shift_tab_walks_back_through_stops() {
        // Tab → Tab walks 1, 2, 0; Shift+Tab from $0 must land on 2.
        let mut result = SnippetParser::parse("${1:a} ${2:b} $0");
        assert!(result.next()); // 1 → 2
        assert!(result.next()); // 2 → 0
        assert!(result.prev()); // 0 → 2
        assert_eq!(result.current().unwrap().index, 2);
        assert!(result.prev()); // 2 → 1
        assert_eq!(result.current().unwrap().index, 1);
        assert!(!result.prev(), "prev at the first stop must return false");
    }

    #[test]
    fn strip_to_display_returns_visible_text_only() {
        // The popup uses this to render snippet bodies without
        // showing `${1:...}` syntax. It should equal the parsed
        // expanded text.
        let visible = SnippetParser::strip_to_display("fn ${1:name}(${2:args}) { $0 }");
        assert_eq!(visible, "fn name(args) {  }");
    }

    #[test]
    fn parser_inserts_implicit_final_stop_when_missing() {
        // VSCode semantics: a snippet with placeholders but no
        // explicit `$0` must still emit one (placed at the end of
        // the expanded text) so Tab-cycling has a known terminator.
        let result = SnippetParser::parse("${1:foo}");
        let final_stop = result
            .tab_stops
            .iter()
            .find(|s| s.index == 0)
            .expect("parser must insert an implicit $0");
        assert_eq!(
            final_stop.offset,
            result.text.len(),
            "implicit $0 should land at end of expanded text"
        );
    }

    #[test]
    fn contains_snippet_syntax_is_quiet_for_plain_dollar() {
        // A bare `$` followed by whitespace / letters is not
        // snippet syntax. (We documented `$100` IS detected
        // upstream, but `$ foo` and `$bar` must not be.)
        assert!(!SnippetParser::contains_snippet_syntax("price: $ owed"));
        assert!(!SnippetParser::contains_snippet_syntax(
            "rust uses $bar style"
        ));
        assert!(!SnippetParser::contains_snippet_syntax(""));
    }
}
