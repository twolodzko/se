mod address;
mod command;
mod editor;
mod parser;
pub use {command::Action, editor::Editor, parser::parse};

#[derive(Debug, PartialEq, Clone)]
pub struct Line(pub usize, pub String);

#[derive(Debug)]
pub enum Error {
    Io(std::io::Error),
    Fmt(std::fmt::Error),
    Regex(regex::Error),
    ParseInt(std::num::ParseIntError),
    Missing(char),
    Unexpected(char),
    InvalidAddr(String),
    ParsingError(String),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use Error::*;
        match self {
            Io(msg) => msg.fmt(f),
            Fmt(msg) => msg.fmt(f),
            Regex(msg) => msg.fmt(f),
            ParseInt(msg) => msg.fmt(f),
            Missing(c) => write!(f, "missing '{}'", c),
            Unexpected(c) => write!(f, "unexpected '{}'", c),
            InvalidAddr(a) => write!(f, "invalid address: {}", a),
            ParsingError(s) => write!(f, "failed to parse: {}", s),
        }
    }
}
