// Copyright 2026 the xilem_code_editor developers
// SPDX-License-Identifier: Apache-2.0

//! Text storage: the [`Document`] trait, its rope-backed implementation,
//! and transaction-based undo.

mod document;
mod position;
mod rope_document;
mod undo;

pub use document::Document;
pub use position::{offset_of_position, position_of_offset};
pub use rope_document::RopeDocument;
pub use undo::UndoManager;
