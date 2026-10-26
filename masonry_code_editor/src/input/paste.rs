//! Clipboard paste handler.

use code_editor_core::cursor::Cursor;
use code_editor_core::document::Document;

use super::InputResult;

// MARK: handle_paste — clipboard insertion

pub fn handle_paste(cursor: &mut Cursor, doc: &mut dyn Document, text: &str) -> InputResult {
    cursor.insert(doc, text);
    InputResult::edit()
}
