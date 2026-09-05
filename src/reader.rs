use crate::lines::ByteLines;
use std::{
    fs::File,
    io::{BufReader, Result},
    path::PathBuf,
};

#[derive(Debug, PartialEq, Default)]
pub struct Line(pub usize, pub String);

pub struct Reader<'a> {
    iter: Box<dyn Iterator<Item = Result<Vec<u8>>> + 'a>,
    counter: usize,
}

impl<'a> Reader<'a> {
    pub fn new<I>(reader: I) -> Self
    where
        I: Iterator<Item = Result<Vec<u8>>> + 'a,
    {
        Self {
            iter: Box::new(reader),
            counter: 0,
        }
    }

    #[cfg(test)]
    pub(crate) fn empty() -> Self {
        Self {
            iter: Box::new(std::iter::empty()),
            counter: 0,
        }
    }
}

impl Default for Reader<'_> {
    fn default() -> Self {
        Self::new(ByteLines::new(BufReader::new(std::io::stdin())))
    }
}

impl From<&[PathBuf]> for Reader<'_> {
    fn from(paths: &[PathBuf]) -> Self {
        if paths.is_empty() {
            Self::default()
        } else {
            Self::new(FilesReader::from(paths))
        }
    }
}

impl Iterator for Reader<'_> {
    type Item = Result<Line>;

    fn next(&mut self) -> Option<Self::Item> {
        self.counter += 1;
        let buf = match self.iter.next()? {
            Ok(line) => line,
            Err(err) => return Some(Err(err)),
        };
        let str = String::from_utf8_lossy(&buf).into_owned();
        Some(Ok(Line(self.counter, str)))
    }
}

struct FilesReader {
    paths: Vec<PathBuf>,
    file: Option<ByteLines<BufReader<File>>>,
}

impl FilesReader {
    fn next_file(&mut self) -> Option<Result<()>> {
        let path = self.paths.pop()?;
        let file = match File::open(path) {
            Ok(file) => file,
            Err(err) => return Some(Err(err)),
        };
        let reader = ByteLines::new(BufReader::new(file));
        self.file = Some(reader);
        Some(Ok(()))
    }
}

impl From<&[PathBuf]> for FilesReader {
    fn from(value: &[PathBuf]) -> Self {
        FilesReader {
            paths: value.iter().rev().cloned().collect(),
            file: None,
        }
    }
}

impl Iterator for FilesReader {
    type Item = Result<Vec<u8>>;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            if let Some(ref mut buffer) = self.file {
                match buffer.next() {
                    Some(Ok(line)) => {
                        return Some(Ok(line));
                    }
                    Some(Err(err)) => return Some(Err(err)),
                    None => {
                        if let Err(err) = self.next_file()? {
                            return Some(Err(err));
                        }
                    }
                }
            } else if let Err(err) = self.next_file()? {
                return Some(Err(err));
            }
        }
    }
}
