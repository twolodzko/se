use super::{read_integer, reader::Reader, regex, skip_line, skip_whitespace};
use crate::{
    Error, Result,
    command::Command::{self, *},
    error,
};
use std::str::FromStr;
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
            'p' => {
                let val = read_string_or_none(reader)?;
                Println(val)
            }
            'P' => {
                let val = read_string_or_none(reader)?;
                Print(val)
            }
            'a' => {
                skip_whitespace(reader);
                let s = read_string(reader)?;
                Append(s)
            }
            'i' => {
                skip_whitespace(reader);
                let s = read_string(reader)?;
                Prepend(s)
            }
            'l' => Escape,
            'L' => UnEscape,
            's' => parse_substitute(reader)?,
            'c' => {
                skip_whitespace(reader);
                keeps_range(reader)?
            }
            '=' => LineNumber,
            'd' => Delete,
            'z' => {
                let val = read_string_or_none(reader)?;
                Reset(val)
            }
            'h' => {
                let val = read_string_or_none(reader)?;
                Hold(val)
            }
            'g' => Get,
            'x' => Exchange,
            'j' => Joinln,
            'J' => Join,
            'k' => Collectln,
            'K' => Collect,
            'e' => {
                skip_whitespace(reader);
                if let Some(s) = reader.peek()?
                    && matches!(s, '"' | '\'')
                {
                    let s = read_string(reader)?;
                    Eval(Some(s))
                } else {
                    Eval(None)
                }
            }
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
            ':' => {
                let label = read_label(reader)?;
                Label(label)
            }
            'b' => {
                let label = read_label(reader)?;
                Branch(label, 0)
            }
            '#' => {
                skip_line(reader);
                continue;
            }
            c if c.is_whitespace() => continue,
            _ => return Err(Error::Unexpected(c)),
        };
        cmds.push(cmd);
        skip_whitespace(reader);
    }
    Ok(cmds)
}

fn read_escaped<R: Reader>(reader: &mut R) -> Result<String> {
    let mut acc = "\\".to_string();
    let Some(c) = reader.next()? else {
        return Err(Error::EndOfInput);
    };
    match c {
        'u' => {
            acc.push(c);
            for _ in 0..4 {
                let Some(c) = reader.next()? else {
                    return Err(Error::EndOfInput);
                };
                acc.push(c);
            }
            Ok(unescape(&acc)?)
        }
        'x' => {
            acc.push(c);
            for _ in 0..2 {
                let Some(c) = reader.next()? else {
                    return Err(Error::EndOfInput);
                };
                acc.push(c);
            }
            Ok(unescape(&acc).map_err(Error::from)?)
        }
        c => {
            acc.push(c);
            unescape(&acc).or(Ok(c.to_string()))
        }
    }
}

fn parse_substitute<R: Reader>(reader: &mut R) -> Result<Command> {
    // Parse: s/src/dst/[limit]
    let (s, delim) = regex::read(reader)?;
    let src = crate::Regex::from_str(&s)?;
    let dst = read_template(reader, delim)?;

    let mut limit = 0;
    if let Some(c) = reader.peek()?
        && c.is_ascii_digit()
    {
        limit = read_integer(reader)?.parse()?;
    }

    Ok(Substitute(src, dst, limit))
}

fn read_template<R: Reader>(reader: &mut R, delim: char) -> Result<String> {
    let mut acc = String::new();
    while let Some(c) = reader.peek()? {
        match c {
            c if c == delim => {
                reader.skip();
                return unescape(&acc).map_err(Error::from);
            }
            '&' => {
                // use sed's symbol for initial input
                reader.skip();
                acc.push_str("${0}");
            }
            '\\' => {
                reader.skip();
                if let Some(e) = reader.peek()? {
                    match e {
                        c if c == delim => {
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
                        '1'..='9' => {
                            // replace sed's \N with Rust's ${N}
                            // keep \0 as null character
                            acc.push('$');
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
    Err(Error::Missing(delim))
}

fn keeps_range<R: Reader>(reader: &mut R) -> Result<Command> {
    let s = read_integer(reader)?;
    let lhs = if s.is_empty() {
        0
    } else if s == "0" {
        return error!("character indexes need to be >0");
    } else {
        s.parse::<usize>().map_err(Error::from)? - 1
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
            return error!("invalid character index range: {}-{}", lhs + 1, rhs);
        }
        Some(rhs - lhs)
    };
    Ok(Keep(lhs, rhs))
}

fn read_string<R: Reader>(reader: &mut R) -> Result<String> {
    let Some(c) = reader.next()? else {
        return Err(Error::EndOfInput);
    };
    let mut s = match c {
        '\\' => read_escaped(reader)?,
        '"' | '\'' => unescape(&read_until(reader, c)?)?,
        _ => return Err(Error::Unexpected(c)),
    };

    skip_whitespace(reader);
    if let Some('*') = reader.peek()? {
        reader.skip();
        skip_whitespace(reader);
        let n = usize::from_str(&read_integer(reader)?)?;
        s = s.repeat(n);
    }

    Ok(s)
}

fn read_string_or_none<R: Reader>(reader: &mut R) -> Result<Option<String>> {
    skip_whitespace(reader);
    if let Some(s) = reader.peek()?
        && matches!(s, '"' | '\'' | '\\')
    {
        let s = read_string(reader)?;
        Ok(Some(s))
    } else {
        Ok(None)
    }
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
    Err(Error::Missing(delim))
}

fn read_label<R: Reader>(reader: &mut R) -> Result<String> {
    skip_whitespace(reader);
    let mut acc = String::new();
    while let Some(c) = reader.peek()? {
        match c {
            c if c.is_alphanumeric() || c == '_' => {
                reader.skip();
                acc.push(c)
            }
            _ => break,
        }
    }
    Ok(acc)
}
