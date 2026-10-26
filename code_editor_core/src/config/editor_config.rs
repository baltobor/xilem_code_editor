// Copyright 2026 the xilem_code_editor developers
// SPDX-License-Identifier: Apache-2.0

//! EditorConfig — editor display and behavior settings.
//!
//! Extracted from widget.rs to follow Single Responsibility Principle.
//! Contains configuration that affects editor appearance and behavior.

/// Editor configuration settings.
///
/// These are display and behavior preferences that can be changed at runtime.
#[derive(Debug, Clone)]
pub struct EditorConfig {
    /// Soft-wrap lines at the viewport width instead of scrolling
    /// horizontally. Off by default (code); prose (Markdown) turns it on.
    pub word_wrap: bool,
    /// Show line numbers in the gutter.
    pub show_line_numbers: bool,

    /// Computed gutter width in pixels.
    /// Updated based on line count and display settings.
    pub gutter_width: f64,

    /// Number of spaces per tab character.
    pub tab_size: usize,

    /// Whether to use text rendering hints for better clarity.
    pub hint: bool,

    /// Which modifier key combination triggers "Go to Definition" on click.
    /// 0 = Ctrl+Cmd, 1 = Ctrl, 2 = Cmd
    pub goto_def_click_mode: u8,
}

impl Default for EditorConfig {
    fn default() -> Self {
        Self::new()
    }
}

impl EditorConfig {
    /// Create a new editor config with default values.
    pub fn new() -> Self {
        Self {
            word_wrap: false,
            show_line_numbers: true,
            gutter_width: 0.0,
            tab_size: 4,
            hint: true,
            goto_def_click_mode: 0,
        }
    }

    /// Update gutter width based on line count.
    ///
    /// The gutter needs to be wide enough to display the largest line number.
    pub fn update_gutter_width(&mut self, line_count: usize) {
        if self.show_line_numbers {
            self.gutter_width = compute_gutter_width(line_count);
        } else {
            self.gutter_width = 0.0;
        }
    }

    /// Set whether to show line numbers.
    ///
    /// Returns true if the value changed.
    pub fn set_show_line_numbers(&mut self, show: bool) -> bool {
        if self.show_line_numbers != show {
            self.show_line_numbers = show;
            true
        } else {
            false
        }
    }

    /// Set the tab size.
    pub fn set_tab_size(&mut self, size: usize) {
        self.tab_size = size;
    }

    /// Set the goto definition click mode.
    pub fn set_goto_def_click_mode(&mut self, mode: u8) {
        self.goto_def_click_mode = mode;
    }

    /// Check if a click with the given modifiers should trigger goto definition.
    ///
    /// Returns `Some(true)` for goto-def, `Some(false)` for hover, `None` for neither.
    pub fn check_goto_click(
        &self,
        ctrl: bool,
        cmd: bool,
        alt: bool,
        is_primary: bool,
        is_secondary: bool,
    ) -> Option<bool> {
        // Hover is triggered by Alt + modifier click
        let hover_triggered = alt && ((ctrl && is_primary) || is_secondary || (cmd && is_primary));

        if hover_triggered {
            return Some(false); // Hover request
        }

        // Check goto definition based on configured mode
        let goto_triggered = match self.goto_def_click_mode {
            0 => {
                // Ctrl + Cmd + Click: on macOS this fires as Secondary + cmd
                (ctrl && cmd && is_primary) || (is_secondary && cmd)
            }
            1 => {
                // Ctrl + Click: on macOS this fires as Secondary button
                (ctrl && !cmd && is_primary) || (is_secondary && !cmd && !alt)
            }
            2 => cmd && !ctrl && is_primary, // Cmd + Click (always Primary)
            _ => false,
        };

        if goto_triggered {
            Some(true) // Goto definition
        } else {
            None // No action
        }
    }
}

/// Compute the gutter width based on line count.
///
/// Dynamic width: enough digits for the highest line number, plus padding.
pub fn compute_gutter_width(line_count: usize) -> f64 {
    let digits = if line_count >= 10000 {
        5
    } else if line_count >= 1000 {
        4
    } else if line_count >= 100 {
        3
    } else {
        2
    };
    // ~8px per digit at 13px font size, plus left + right padding
    (digits as f64) * 8.0 + 32.0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = EditorConfig::new();
        assert!(config.show_line_numbers);
        assert_eq!(config.gutter_width, 0.0);
        assert_eq!(config.tab_size, 4);
        assert!(config.hint);
    }

    #[test]
    fn test_gutter_width_calculation() {
        assert_eq!(compute_gutter_width(10), 48.0); // 2 digits
        assert_eq!(compute_gutter_width(99), 48.0); // 2 digits
        assert_eq!(compute_gutter_width(100), 56.0); // 3 digits
        assert_eq!(compute_gutter_width(999), 56.0); // 3 digits
        assert_eq!(compute_gutter_width(1000), 64.0); // 4 digits
        assert_eq!(compute_gutter_width(9999), 64.0); // 4 digits
        assert_eq!(compute_gutter_width(10000), 72.0); // 5 digits
    }

    #[test]
    fn test_update_gutter_width() {
        let mut config = EditorConfig::new();

        config.update_gutter_width(500);
        assert_eq!(config.gutter_width, 56.0); // 3 digits

        config.show_line_numbers = false;
        config.update_gutter_width(500);
        assert_eq!(config.gutter_width, 0.0); // No gutter when disabled
    }

    #[test]
    fn test_set_show_line_numbers() {
        let mut config = EditorConfig::new();
        assert!(config.show_line_numbers);

        // Setting to same value returns false
        assert!(!config.set_show_line_numbers(true));

        // Changing returns true
        assert!(config.set_show_line_numbers(false));
        assert!(!config.show_line_numbers);
    }
}
