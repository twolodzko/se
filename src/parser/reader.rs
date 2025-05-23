use crate::Error;
use std::{
    fs::File,
    io::{BufRead, BufReader, Lines},
    iter::Peekable,
    path::PathBuf,
    vec::IntoIter,
};

pub(crate) trait Reader {
    fn next(&mut self) -> Result<Option<char>, Error>;
    fn peek(&mut self) -> Result<Option<char>, Error>;
}

pub(crate) struct StringReader(Peekable<IntoIter<char>>);

impl From<&str> for StringReader {
    fn from(value: &str) -> Self {
        StringReader(value.chars().collect::<Vec<char>>().into_iter().peekable())
    }
}

impl Reader for StringReader {
    fn next(&mut self) -> Result<Option<char>, Error> {
        Ok(self.0.next())
    }

    fn peek(&mut self) -> Result<Option<char>, Error> {
        Ok(self.0.peek().cloned())
    }
}

pub(crate) struct FileReader {
    file: Lines<BufReader<File>>,
    buffer: StringReader,
}

impl TryFrom<&PathBuf> for FileReader {
    type Error = Error;

    fn try_from(value: &PathBuf) -> Result<Self, Self::Error> {
        let file = BufReader::new(File::open(value).map_err(Error::Io)?).lines();
        let chars = StringReader::from("");
        Ok(FileReader {
            file,
            buffer: chars,
        })
    }
}

impl Reader for FileReader {
    fn next(&mut self) -> Result<Option<char>, Error> {
        loop {
            if let c @ Some(_) = self.buffer.next()? {
                return Ok(c);
            }
            if !self.next_line()? {
                return Ok(None);
            }
        }
    }

    fn peek(&mut self) -> Result<Option<char>, Error> {
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
    fn next_line(&mut self) -> Result<bool, Error> {
        if let Some(res) = self.file.next() {
            let mut line = res.map_err(Error::Io)?;
            line.push('\n');
            self.buffer = StringReader::from(line.as_str());
            return Ok(true);
        }
        Ok(false)
    }
}
