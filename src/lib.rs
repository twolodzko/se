mod address;
mod command;
mod editor;
mod parser;

use std::string::FromUtf8Error;
pub use {
    command::Command,
    editor::Editor,
    parser::parse,
    parser::{FileReader, StringReader},
};

#[derive(Debug, PartialEq, Clone)]
pub struct Line(pub usize, pub String);

#[derive(Debug, Clone)]
pub struct Regex(regex::Regex);

impl Regex {
    pub(crate) fn new(s: &str) -> Result<Regex, Error> {
        let regex = regex::Regex::new(s).map_err(Error::Regex)?;
        Ok(Regex(regex))
    }
}

impl PartialEq for Regex {
    fn eq(&self, other: &Self) -> bool {
        self.0.as_str() == other.0.as_str()
    }
}

impl std::fmt::Display for Regex {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

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
    FromUtf8Error(FromUtf8Error),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use Error::*;
        match self {
            Io(msg) => msg.fmt(f),
            Fmt(msg) => msg.fmt(f),
            Regex(msg) => msg.fmt(f),
            ParseInt(msg) => msg.fmt(f),
            FromUtf8Error(msg) => msg.fmt(f),
            Missing(c) => write!(f, "missing '{}'", c),
            Unexpected(c) => write!(f, "unexpected '{}'", c),
            InvalidAddr(a) => write!(f, "invalid address: {}", a),
            ParsingError(s) => write!(f, "failed to parse: {}", s),
        }
    }
}
