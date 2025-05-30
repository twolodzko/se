mod address;
mod command;
mod lines;
mod parser;
mod program;

pub use {
    command::Status,
    lines::{FilesReader, Line, StdinReader},
    program::Program,
};

#[derive(Debug)]
pub(crate) struct Regex(regex::Regex);

impl std::str::FromStr for Regex {
    type Err = Error;

    fn from_str(s: &str) -> std::result::Result<Regex, Self::Err> {
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

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug)]
pub enum Error {
    Io(std::io::Error),
    Fmt(std::fmt::Error),
    Regex(regex::Error),
    ParseInt(std::num::ParseIntError),
    Missing(char),
    Unexpected(char),
    InvalidAddr(String),
    Custom(String),
    FromUtf8Error(std::string::FromUtf8Error),
    Utf8Error(std::str::Utf8Error),
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
            Utf8Error(msg) => msg.fmt(f),
            Missing(c) => write!(f, "missing '{}'", c),
            Unexpected(c) => write!(f, "unexpected '{}'", c),
            InvalidAddr(a) => write!(f, "invalid address: {}", a),
            Custom(s) => s.fmt(f),
        }
    }
}
