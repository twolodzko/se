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
    type Err = anyhow::Error;

    fn from_str(s: &str) -> std::result::Result<Regex, Self::Err> {
        let regex = regex::Regex::new(s)?;
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
    InvalidAddr(String),
    LabelInFinal,
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use Error::*;
        match self {
            Missing(c) => write!(f, "missing '{}'", c),
            Unexpected(c) => write!(f, "unexpected '{}'", c),
            InvalidAddr(a) => write!(f, "invalid address: {}", a),
            LabelInFinal => write!(f, "labels are not allowed in the final block"),
        }
    }
}
