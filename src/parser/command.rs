use std::str::FromStr;

use super::{Error, read_integer, reader::Reader, regex, skip_line, skip_whitespace};
use crate::command::Command::{self, *};
use anyhow::{Result, bail};
use unescaper::unescape;

pub(crate) fn parse<R: Reader>(reader: &mut R) -> Result<Vec<Command>> {
    let mut cmds = Vec::new();
    while let Some(c) = reader.next()? {
        let cmd = match c {
            ';' => break,
            '.' => {
                cmds.push(Break);
                break;
            }
            'p' => Println,
            'P' => Print,
            '\\' => {
                let s = read_escaped(reader)?;
                Insert(s)
            }
            'l' => Escape,
            'L' => UnEscape,
            't' => ToHtml,
            'T' => FromHtml,
            'u' => ToUrl,
            'U' => FromUrl,
            'b' => ToBase64,
            'B' => FromBase64,
            's' => parse_substitute(reader)?,
            'k' => {
                skip_whitespace(reader);
                read_range(reader)?
            }
            '=' => LineNumber,
            'd' => Delete,
            '&' => GetLine,
            'z' => Reset,
            'h' => Hold,
            'g' => Get,
            'x' => Exchange,
            'j' => Joinln,
            'J' => Join,
            'e' => Eval,
            'r' => {
                skip_whitespace(reader);
                let s = read_integer(reader)?;
                let num = if s.is_empty() { 1 } else { s.parse()? };
                Readln(num)
            }
            'R' => ReadReplace,
            'q' => {
                skip_whitespace(reader);
                let s = read_integer(reader)?;
                let code = if s.is_empty() { 0 } else { s.parse()? };
                Quit(code)
            }
            '\'' | '"' => {
                let msg = unescape(&read_until(reader, c)?)?;
                Insert(msg)
            }
            '#' => {
                skip_line(reader);
                continue;
            }
            c if c.is_whitespace() => continue,
            _ => bail!(Error::Unexpected(c)),
        };
        cmds.push(cmd);
        skip_whitespace(reader);
    }
    Ok(cmds)
}

fn read_escaped<R: Reader>(reader: &mut R) -> Result<String> {
    let mut acc = "\\".to_string();
    let Some(c) = reader.next()? else {
        bail!(Error::EndOfInput)
    };
    match c {
        'u' => {
            acc.push(c);
            for _ in 0..4 {
                let Some(c) = reader.next()? else {
                    bail!(Error::EndOfInput)
                };
                acc.push(c);
            }
            Ok(unescape(&acc)?)
        }
        'x' => {
            acc.push(c);
            for _ in 0..2 {
                let Some(c) = reader.next()? else {
                    bail!(Error::EndOfInput)
                };
                acc.push(c);
            }
            Ok(unescape(&acc)?)
        }
        c => {
            acc.push(c);
            unescape(&acc).or(Ok(c.to_string()))
        }
    }
}

fn parse_substitute<R: Reader>(reader: &mut R) -> Result<Command> {
    // Parse: s/src/dst/[limit]
    let s = regex::read(reader)?;
    let src = crate::Regex::from_str(&s)?;
    let dst = read_template(reader)?;

    let mut limit = 0;
    if let Some(c) = reader.peek()?
        && c.is_ascii_digit()
    {
        limit = read_integer(reader)?.parse()?;
    }

    Ok(Substitute(src, dst, limit))
}

fn read_template<R: Reader>(reader: &mut R) -> Result<String> {
    let delim = '/';
    let mut acc = String::new();
    while let Some(c) = reader.peek()? {
        match c {
            c if c == delim => {
                reader.skip();
                return Ok(unescape(&acc)?);
            }
            '\\' => {
                reader.skip();
                if let Some(e) = reader.peek()? {
                    match e {
                        e if e == delim => {
                            reader.skip();
                            acc.push(e);
                        }
                        '$' => {
                            reader.skip();
                            acc.push_str("$$");
                        }
                        '{' => {
                            acc.push('$');
                        }
                        e if e.is_ascii_digit() => {
                            acc.push('$');
                            // replace $N with ${N}
                            // "$123something" string is interpreted as "${123}something" rather than "${123something}"
                            acc.push('{');
                            acc.push_str(&read_integer(reader)?);
                            acc.push('}');
                        }
                        _ => {
                            reader.skip();
                            acc.push('\\');
                            acc.push(e);
                        }
                    }
                } else {
                    break;
                }
            }
            _ => {
                reader.skip();
                acc.push(c)
            }
        }
    }
    bail!(Error::Missing(delim))
}

fn read_range<R: Reader>(reader: &mut R) -> Result<Command> {
    let s = read_integer(reader)?;
    let lhs = if s.is_empty() {
        0
    } else {
        let lhs: usize = s.parse()?;
        if lhs == 0 {
            bail!("character indexes need to be >0");
        }
        lhs - 1
    };

    if !reader.next_is('-')? {
        return Ok(Keep(lhs, Some(1)));
    };

    let s = read_integer(reader)?;
    let rhs = if s.is_empty() {
        None
    } else {
        let rhs: usize = s.parse()?;
        if rhs == 0 || lhs > rhs {
            bail!(
                "invalid character index range: {} > {} in {}-{}",
                lhs + 1,
                rhs,
                lhs + 1,
                rhs,
            );
        }
        Some(rhs - lhs)
    };
    Ok(Keep(lhs, rhs))
}

fn read_until<R: Reader>(reader: &mut R, delim: char) -> Result<String> {
    let mut acc = String::new();
    while let Some(c) = reader.next()? {
        match c {
            c if c == delim => return Ok(acc),
            '\\' => {
                if let Some(e) = reader.next()? {
                    if e != delim {
                        acc.push(c);
                    }
                    acc.push(e);
                } else {
                    break;
                }
            }
            _ => acc.push(c),
        }
    }
    bail!(Error::Missing(delim))
}
