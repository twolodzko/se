use anyhow::{bail, Result};
use std::{
    fs::File,
    io::{BufRead, BufReader, Lines},
    iter::Peekable,
    path::PathBuf,
    vec::IntoIter,
};

use crate::Error;

pub(crate) trait Reader {
    fn next(&mut self) -> Result<Option<char>>;
    fn peek(&mut self) -> Result<Option<char>>;
    fn current_position(&self) -> String;

    fn skip(&mut self) {
        self.next().unwrap();
    }
    fn expect(&mut self, value: char) -> Result<()> {
        match self.next()? {
            Some(c) if c != value => bail!(Error::Missing(value)),
            _ => Ok(()),
        }
    }

    /// If next character is `value` proceed and return `true`,
    /// otherwise return `false` and don't proceed.
    fn next_is(&mut self, value: char) -> Result<bool> {
        if let Some(c) = self.peek()? {
            if c == value {
                self.skip();
                return Ok(true);
            }
        }
        Ok(false)
    }
}

pub(crate) struct StringReader(Peekable<IntoIter<char>>, String, usize);

impl From<&str> for StringReader {
    fn from(value: &str) -> Self {
        StringReader(
            value.chars().collect::<Vec<char>>().into_iter().peekable(),
            value.to_string(),
            0,
        )
    }
}

impl Reader for StringReader {
    fn next(&mut self) -> Result<Option<char>> {
        if let c @ Some(_) = self.0.next() {
            self.2 += 1;
            return Ok(c);
        }
        Ok(None)
    }

    fn peek(&mut self) -> Result<Option<char>> {
        if let c @ Some(_) = self.0.peek().cloned() {
            return Ok(c);
        }
        Ok(None)
    }

    fn current_position(&self) -> String {
        format!("  {}\n  {}^", self.1, " ".repeat(self.2.saturating_sub(1)))
    }
}

pub(crate) struct FileReader {
    file: Lines<BufReader<File>>,
    buffer: StringReader,
}

impl TryFrom<&PathBuf> for FileReader {
    type Error = anyhow::Error;

    fn try_from(value: &PathBuf) -> Result<Self, Self::Error> {
        let file = BufReader::new(File::open(value)?).lines();
        let chars = StringReader::from("");
        Ok(FileReader {
            file,
            buffer: chars,
        })
    }
}

impl Reader for FileReader {
    fn next(&mut self) -> Result<Option<char>> {
        loop {
            if let c @ Some(_) = self.buffer.next()? {
                return Ok(c);
            }
            if !self.next_line()? {
                return Ok(None);
            }
        }
    }

    fn peek(&mut self) -> Result<Option<char>> {
        loop {
            if let c @ Some(_) = self.buffer.peek()? {
                return Ok(c);
            }
            if !self.next_line()? {
                return Ok(None);
            }
        }
    }

    fn current_position(&self) -> String {
        self.buffer.current_position()
    }
}

impl FileReader {
    fn next_line(&mut self) -> Result<bool> {
        if let Some(res) = self.file.next() {
            let mut line = res?;
            line.push('\n');
            self.buffer = StringReader::from(line.as_str());
            return Ok(true);
        }
        Ok(false)
    }
}
