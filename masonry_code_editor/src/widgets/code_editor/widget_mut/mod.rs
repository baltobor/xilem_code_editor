//! WidgetMut API for the CodeEditorWidget.
//!
//! These methods provide the external mutation API used by the View
//! layer to update the widget state (text content, theme, language,
//! breakpoints, LSP-driven mutations, etc.).
//!
//! ## File layout
//!
//! - `document.rs`        — `set_text` + display config (brushes,
//!                          line numbers, font size, tab size,
//!                          language, highlights).
//! - `edit_ops.rs`        — undo / redo / cut / copy / select-all,
//!                          plus the menu-invoked `goto_line`.
//! - `lsp_mutations.rs`   — `perform_text_edit`,
//!                          `perform_workspace_edits`, and
//!                          `perform_insert` — the paths LSP and
//!                          quick-fixes use to mutate the document
//!                          while keeping undo coherent.
//! - `state_setters.rs`   — theme, breakpoints, execution-line, and
//!                          the LSP-driven UI state setters
//!                          (diagnostics, completion popup,
//!                          inlay hints, etc.).

mod document;
mod edit_commands;
mod edit_ops;
mod state_setters;
