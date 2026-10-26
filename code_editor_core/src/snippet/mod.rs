// Copyright 2026 the xilem_code_editor developers
// SPDX-License-Identifier: Apache-2.0

//! LSP/VSCode snippet syntax: parsing and live tab-stop sessions.

mod parser;
mod session;

pub use parser::{ExpandedSnippet, SnippetParser, TabStop};
pub use session::{EditDisposition, SnippetSession, TabStopSpan};
