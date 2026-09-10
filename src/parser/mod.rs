pub(crate) mod address;
mod command;
mod instruction;
mod program;
mod reader;
mod regex;

use crate::Result;
use reader::Reader;
#[cfg(test)]
pub(crate) use reader::StringReader;

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
