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

    fn from_str(s: &str) -> Result<Regex, Self::Err> {
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
    Custom(String),
    FromUtf8Error(std::string::FromUtf8Error),
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
            Custom(s) => s.fmt(f),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{Line, Program};
    use std::str::FromStr;
    use test_case::test_case;

    #[test_case(
        "k3-5",
        "345";
        "range"
    )]
    #[test_case(
        "k-5",
        "12345";
        "left-open range"
    )]
    #[test_case(
        "k5",
        "5";
        "n-th chars"
    )]
    #[test_case(
        "k3-",
        "3456789";
        "right-open range"
    )]
    #[test_case(
        "k1-1",
        "1";
        "single item range"
    )]
    #[test_case(
        "k1",
        "1";
        "first item"
    )]
    fn keep(command: &str, expected: &str) {
        let func = Program::from_str(command).unwrap();
        let pattern = &mut Line(0, "123456789".to_string());
        func.process(pattern, &mut String::new()).unwrap();
        assert_eq!(pattern.1, expected)
    }
}
