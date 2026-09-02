use std::{
    fs::File,
    io::{BufRead, BufReader, Lines, Result},
    path::PathBuf,
};

#[derive(Debug, PartialEq, Default)]
pub struct Line(pub usize, pub String);

pub struct Reader<'a> {
    iter: Box<dyn Iterator<Item = Result<String>> + 'a>,
    counter: usize,
}

impl<'a> Reader<'a> {
    pub fn new<I>(reader: I) -> Self
    where
        I: Iterator<Item = Result<String>> + 'a,
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
        Self::new(BufReader::new(std::io::stdin()).lines())
    }
}

impl From<Vec<PathBuf>> for Reader<'_> {
    fn from(paths: Vec<PathBuf>) -> Self {
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
        let line = match self.iter.next()? {
            Ok(line) => line,
            Err(err) => return Some(Err(err)),
        };
        Some(Ok(Line(self.counter, line)))
    }
}

struct FilesReader {
    paths: Vec<PathBuf>,
    file: Option<Lines<BufReader<File>>>,
}

impl FilesReader {
    fn next_file(&mut self) -> Option<Result<()>> {
        let path = self.paths.pop()?;
        let file = match File::open(path) {
            Ok(file) => file,
            Err(err) => return Some(Err(err)),
        };
        let reader = BufReader::new(file).lines();
        self.file = Some(reader);
        Some(Ok(()))
    }
}

impl From<Vec<PathBuf>> for FilesReader {
    fn from(value: Vec<PathBuf>) -> Self {
        FilesReader {
            paths: value.into_iter().rev().collect(),
            file: None,
        }
    }
}

impl Iterator for FilesReader {
    type Item = Result<String>;

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
