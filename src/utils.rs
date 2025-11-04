use std::fmt::Display;

use line_index::{LineCol, LineIndex, TextRange};

use crate::database::Database;

pub fn position(point: LineCol) -> lsp_types::Position {
    lsp_types::Position {
        line: point.line,
        character: point.col,
    }
}

pub fn line_col(position: lsp_types::Position) -> LineCol {
    LineCol {
        line: position.line,
        col: position.character,
    }
}

pub fn range(range: TextRange, index: &LineIndex) -> lsp_types::Range {
    lsp_types::Range {
        start: position(index.line_col(range.start())),
        end: position(index.line_col(range.end())),
    }
}

pub trait OrLog {
    type Result;
    type Error: Display;

    /// Convert the error to a string using the given function and then report the error to the
    /// LSP client
    fn or_log(self, db: &Database) -> Option<Self::Result>;
}

impl<T, E: Display> OrLog for Result<T, E> {
    type Result = T;
    type Error = E;

    fn or_log(self, db: &Database) -> Option<Self::Result> {
        match self {
            Ok(value) => Some(value),
            Err(err) => {
                db.log_error(&format!("{err}"));
                None
            }
        }
    }
}
