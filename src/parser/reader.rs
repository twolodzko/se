use anyhow::Result;
use std::{
    fs::File,
    io::{BufRead, BufReader, Lines},
    iter::Peekable,
    path::PathBuf,
    vec::IntoIter,
};

pub(crate) trait Reader {
    fn next(&mut self) -> Result<Option<char>>;
    fn peek(&mut self) -> Result<Option<char>>;

    fn skip(&mut self) {
        self.next().unwrap();
    }

    /// If next character is `value` proceed and return `true`,
    /// otherwiser return `false` and don't proceed.
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

pub(crate) struct StringReader(Peekable<IntoIter<char>>);

impl From<&str> for StringReader {
    fn from(value: &str) -> Self {
        StringReader(value.chars().collect::<Vec<char>>().into_iter().peekable())
    }
}

impl Reader for StringReader {
    fn next(&mut self) -> Result<Option<char>> {
        Ok(self.0.next())
    }

    fn peek(&mut self) -> Result<Option<char>> {
        Ok(self.0.peek().cloned())
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
