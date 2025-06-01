use super::{reader::Reader, regex_reader};
use crate::Regex;
use anyhow::Result;
use std::str::FromStr;

pub(crate) fn skip_whitespace<R: Reader>(reader: &mut R) {
    while reader
        .peek()
        .is_ok_and(|o| o.is_some_and(|c| c.is_whitespace()))
    {
        reader.skip();
    }
}

pub(crate) fn skip_line<R: Reader>(reader: &mut R) {
    while reader.next().is_ok_and(|o| o.is_some_and(|c| c != '\n')) {}
}

pub(crate) fn read_integer<R: Reader>(reader: &mut R) -> Result<String> {
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

pub(crate) fn parse_regex<R: Reader>(reader: &mut R) -> Result<Option<Regex>> {
    let regex = regex_reader::read_regex(reader)?;
    if regex.is_empty() {
        return Ok(None);
    }
    Ok(Some(Regex::from_str(&regex)?))
}
