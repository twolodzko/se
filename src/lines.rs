use crate::Error;
use std::{
    fs::File,
    io::{BufRead, BufReader, Lines},
    path::PathBuf,
};

#[derive(Debug, PartialEq, Default)]
pub struct Line(pub usize, pub String);

pub struct StdinReader {
    buffer: Lines<BufReader<std::io::Stdin>>,
    counter: usize,
}

impl Default for StdinReader {
    fn default() -> Self {
        StdinReader {
            buffer: BufReader::new(std::io::stdin()).lines(),
            counter: 0,
        }
    }
}

impl Iterator for StdinReader {
    type Item = crate::Result<Line>;

    fn next(&mut self) -> Option<Self::Item> {
        match self.buffer.next()? {
            Ok(line) => {
                self.counter += 1;
                let line = Line(self.counter, line.to_string());
                Some(Ok(line))
            }
            Err(err) => Some(Err(Error::Io(err))),
        }
    }
}

pub struct FilesReader {
    paths: Vec<PathBuf>,
    file: Option<Lines<BufReader<File>>>,
    counter: usize,
}

impl FilesReader {
    fn next_file(&mut self) -> Option<crate::Result<()>> {
        let path = self.paths.pop()?;
        let file = match File::open(path) {
            Ok(file) => file,
            Err(err) => return Some(Err(Error::Io(err))),
        };
        let reader = BufReader::new(file).lines();
        self.file = Some(reader);
        Some(Ok(()))
    }
}

impl From<Vec<PathBuf>> for FilesReader {
    fn from(value: Vec<PathBuf>) -> Self {
        FilesReader {
            paths: value.iter().cloned().rev().collect(),
            file: None,
            counter: 0,
        }
    }
}

impl Iterator for FilesReader {
    type Item = crate::Result<Line>;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            if let Some(ref mut buffer) = self.file {
                match buffer.next() {
                    Some(Ok(line)) => {
                        self.counter += 1;
                        let line = Line(self.counter, line.to_string());
                        return Some(Ok(line));
                    }
                    Some(Err(err)) => return Some(Err(Error::Io(err))),
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

#[cfg(test)]
pub(crate) struct MockReader {}

#[cfg(test)]
impl Iterator for MockReader {
    type Item = crate::Result<Line>;
    fn next(&mut self) -> Option<Self::Item> {
        None
    }
}
