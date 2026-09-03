pub(crate) mod address;
mod command;
mod instruction;
mod program;
mod reader;
mod regex;

use anyhow::Result;
use reader::Reader;
#[cfg(test)]
pub(crate) use reader::StringReader;

#[derive(Debug)]
pub enum Error {
    Missing(char),
    Unexpected(char),
    EndOfInput,
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use Error::*;
        match self {
            Missing(c) => write!(f, "missing '{c}'"),
            Unexpected(c) => write!(f, "unexpected '{c}'"),
            EndOfInput => write!(f, "unexpected end of input"),
        }
    }
}

fn skip_whitespace<R: Reader>(reader: &mut R) {
    while reader
        .peek()
        .is_ok_and(|o| o.is_some_and(|c| c.is_whitespace()))
    {
        reader.skip();
    }
}

fn skip_line<R: Reader>(reader: &mut R) {
    while reader.next().is_ok_and(|o| o.is_some_and(|c| c != '\n')) {}
}

fn read_integer<R: Reader>(reader: &mut R) -> Result<String> {
    let mut num = String::new();
    loop {
        match reader.peek()? {
            Some(c) if c.is_ascii_digit() => num.push(c),
            _ => break,
        }
        reader.skip();
    }
    Ok(num)
}
