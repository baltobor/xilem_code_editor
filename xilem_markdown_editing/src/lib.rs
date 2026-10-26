// Copyright 2026 the xilem_code_editor developers
// SPDX-License-Identifier: Apache-2.0

//! The Markdown editing bar of a Xilem app: formatting buttons (bold …
//! code block) whose actions become `xilem_code_editor` commands, and
//! the Edit / Preview / Split and word-wrap toggles.

mod format_command;
mod format_toolbar;
mod markdown_format;
mod toolbar_style;
mod view_mode;

pub use format_command::format_command;
pub use format_toolbar::{markdown_format_toolbar, toolbar_icon_button};
pub use markdown_format::MarkdownFormat;
pub use toolbar_style::ToolbarStyle;
pub use view_mode::{MarkdownViewMode, view_mode_buttons};
