pub(crate) mod address;
mod command;
mod instruction;
mod program;
mod reader;
mod regex_reader;
mod utils;

#[cfg(test)]
pub(crate) use reader::StringReader;

#[derive(Debug)]
pub enum Error {
    Missing(char),
    Unexpected(char),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use Error::*;
        match self {
            Missing(c) => write!(f, "missing '{c}'"),
            Unexpected(c) => write!(f, "unexpected '{c}'"),
        }
    }
}
