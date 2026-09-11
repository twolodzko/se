mod address;
mod command;
mod lines;
mod parser;
mod program;
mod reader;

use crate::address::Address;
pub use {
    command::Status,
    program::Program,
    reader::{Line, Reader},
};

#[derive(Debug, Clone)]
pub(crate) struct Regex(regex::Regex);

#[derive(Debug, PartialEq)]
pub(crate) enum Action {
    Condition(address::Address, usize),
    Command(command::Command),
}

#[derive(Debug, PartialEq, Default)]
pub(crate) struct Memory {
    pub(crate) index: usize,
    pub(crate) this: String,
    pub(crate) hold: String,
}

impl Memory {
    pub(crate) fn read(&mut self, line: Line) {
        self.index = line.0;
        self.this = line.1;
    }
}

impl std::fmt::Display for Action {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use Action::*;
        match self {
            Condition(a, _) => a.fmt(f),
            Command(c) => c.fmt(f),
        }
    }
}

impl std::str::FromStr for Regex {
    type Err = Error;

    fn from_str(s: &str) -> std::result::Result<Regex, Self::Err> {
        let regex = regex::Regex::new(s).map_err(Error::from)?;
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
    Missing(char),
    Unexpected(char),
    EndOfInput,
    #[allow(private_interfaces)]
    Impossible(Address),
    Io(std::io::Error),
    Custom(String),
    ParseInt(std::num::ParseIntError),
    Unescape(unescaper::Error),
    Regex(regex::Error),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use Error::*;
        match self {
            Missing(c) => write!(f, "missing '{c}'"),
            Unexpected(c) => write!(f, "unexpected '{c}'"),
            EndOfInput => write!(f, "unexpected end of input"),
            Impossible(a) => write!(f, "{} is an impossible condition", a),
            Io(e) => write!(f, "{}", e),
            Custom(s) => write!(f, "{}", s),
            ParseInt(e) => write!(f, "{}", e),
            Unescape(e) => write!(f, "{}", e),
            Regex(e) => write!(f, "{}", e),
        }
    }
}

impl From<std::io::Error> for Error {
    fn from(value: std::io::Error) -> Self {
        Error::Io(value)
    }
}

impl From<std::num::ParseIntError> for Error {
    fn from(value: std::num::ParseIntError) -> Self {
        Error::ParseInt(value)
    }
}

impl From<unescaper::Error> for Error {
    fn from(value: unescaper::Error) -> Self {
        Error::Unescape(value)
    }
}

impl From<regex::Error> for Error {
    fn from(value: regex::Error) -> Self {
        Error::Regex(value)
    }
}

#[macro_export]
macro_rules! error {
    ($($arg:tt)*) => {
        Err(Error::Custom(format!($($arg)*)))
    };
}

pub type Result<T> = std::result::Result<T, Error>;
