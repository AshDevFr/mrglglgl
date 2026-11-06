//! A reader for the files WoW writes to `WTF/.../SavedVariables/`.
//!
//! These files are Lua source, but only a small, fixed subset of it: top-level `Name = value`
//! assignments of nested table literals. Parsing that subset directly means the app never runs
//! Lua, so a file can never do anything but be read. Live Forever clients write it without
//! indentation or index comments; other client versions add both, and both are accepted.

mod lexer;
mod parser;
mod value;

use std::collections::BTreeMap;
use std::fmt;

pub use parser::MAX_DEPTH;
pub use value::{Key, Table, Value};

/// The top-level variables of one SavedVariables file, by name.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct SavedVariables {
    globals: BTreeMap<String, Value>,
}

impl SavedVariables {
    fn insert(&mut self, name: String, value: Value) {
        self.globals.insert(name, value);
    }

    pub fn get(&self, name: &str) -> Option<&Value> {
        self.globals.get(name)
    }

    pub fn names(&self) -> impl Iterator<Item = &str> {
        self.globals.keys().map(String::as_str)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseError {
    pub line: usize,
    pub column: usize,
    pub kind: ParseErrorKind,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParseErrorKind {
    UnexpectedCharacter(char),
    UnexpectedToken(String),
    UnexpectedEnd,
    UnterminatedString,
    UnterminatedComment,
    InvalidEscape,
    InvalidNumber,
    InvalidKey,
    TooDeep,
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let what = match &self.kind {
            ParseErrorKind::UnexpectedCharacter(c) => format!("unexpected character {c:?}"),
            ParseErrorKind::UnexpectedToken(t) => format!("unexpected {t}"),
            ParseErrorKind::UnexpectedEnd => "the file ends in the middle of a value".to_string(),
            ParseErrorKind::UnterminatedString => "a string is not closed".to_string(),
            ParseErrorKind::UnterminatedComment => "a comment is not closed".to_string(),
            ParseErrorKind::InvalidEscape => "invalid escape in a string".to_string(),
            ParseErrorKind::InvalidNumber => "invalid number".to_string(),
            ParseErrorKind::InvalidKey => "invalid table key".to_string(),
            ParseErrorKind::TooDeep => format!("tables nested deeper than {MAX_DEPTH} levels"),
        };
        write!(f, "line {}, column {}: {what}", self.line, self.column)
    }
}

impl std::error::Error for ParseError {}

/// Parses the text of a SavedVariables file.
pub fn parse(text: &str) -> Result<SavedVariables, ParseError> {
    parser::Parser::new(text)?.file()
}
