use std::error::Error;
use std::fmt::Display;

use line_index::{LineCol, LineIndex, TextRange};

pub fn position(point: LineCol) -> lsp_types::Position {
    lsp_types::Position {
        line: point.line,
        character: point.col,
    }
}

pub fn range(range: TextRange, index: &LineIndex) -> lsp_types::Range {
    lsp_types::Range {
        start: position(index.line_col(range.start())),
        end: position(index.line_col(range.end())),
    }
}

pub fn io_error(message: String) -> std::io::Error {
    std::io::Error::other(SimpleMessage { message })
}

#[derive(Debug)]
pub struct SimpleMessage {
    message: String,
}

impl Display for SimpleMessage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.message)
    }
}

impl Error for SimpleMessage {}
