use super::{
    reader::Reader,
    utils::{parse_regex, read_integer, read_label, skip_line, skip_whitespace},
};
use crate::{
    command::Command::{self, *},
    Error,
};

pub(crate) fn parse<R: Reader>(reader: &mut R) -> Result<Vec<Command>, Error> {
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
            'l' => Escapeln,
            's' => parse_substitute(reader)?,
            'k' => {
                skip_whitespace(reader);
                parse_keep(reader)?
            }
            '=' => LineNumber,
            'd' => Delete,
            'z' => Reset,
            'h' => Hold,
            'g' => Get,
            'x' => Exchange,
            'j' => Joinln,
            'J' => Join,
            'r' => {
                skip_whitespace(reader);
                let s = read_integer(reader)?;
                let num = if s.is_empty() {
                    1
                } else {
                    s.parse().map_err(Error::ParseInt)?
                };
                Readln(num)
            }
            'q' => {
                skip_whitespace(reader);
                let s = read_integer(reader)?;
                let code = if s.is_empty() {
                    0
                } else {
                    s.parse().map_err(Error::ParseInt)?
                };
                Quit(code)
            }
            'b' => {
                skip_whitespace(reader);
                let label = read_label(reader)?;
                GoTo(label, 0)
            }
            '\'' | '"' => {
                let msg = unescape(read_until(reader, c)?)?;
                Insert(msg)
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
        if let Some('}') = reader.peek()? {
            break;
        }
    }
    Ok(cmds)
}

fn parse_substitute<R: Reader>(reader: &mut R) -> Result<Command, Error> {
    if reader.peek()? != Some('/') {
        return Err(Error::Missing('/'));
    }

    // Parse: s/src/dst/[limit]
    let Some(src) = parse_regex(reader)? else {
        return Err(Error::Custom("empty regular expression".to_string()));
    };
    let dst = read_template(reader)?;

    let mut limit = 0;
    if let Some(c) = reader.peek()? {
        if c == 'g' {
            reader.next()?;
            // g is default, no need to update the limit
        } else if c.is_ascii_digit() {
            limit = read_integer(reader)?.parse().map_err(Error::ParseInt)?;
        }
    }

    Ok(Substitute(src, dst, limit))
}

fn read_template<R: Reader>(reader: &mut R) -> Result<String, Error> {
    let delim = '/';
    let mut acc = String::new();
    while let Some(c) = reader.peek()? {
        match c {
            c if c == delim => {
                reader.next()?;
                return unescape(acc);
            }
            c if c.is_ascii_digit() => {
                // replace $N with ${N}
                // "$123something" string is interpreted as "${123}something" rather than "${123something}"
                acc.push('{');
                acc.push_str(&read_integer(reader)?);
                acc.push('}');
            }
            '\\' => {
                reader.next()?;
                if let Some(e) = reader.next()? {
                    if e != delim {
                        acc.push(c);
                    }
                    acc.push(e);
                } else {
                    break;
                }
            }
            _ => {
                reader.next()?;
                acc.push(c)
            }
        }
    }
    Err(Error::Missing(delim))
}

fn parse_keep<R: Reader>(reader: &mut R) -> Result<Command, Error> {
    let s = read_integer(reader)?;
    let lhs = if s.is_empty() {
        0
    } else {
        let num: usize = s.parse().map_err(Error::ParseInt)?;
        if num == 0 {
            return Err(Error::Custom("character indexes need to be >0".to_string()));
        }
        num - 1
    };

    if let Some('-') = reader.peek()? {
        reader.next()?;
    } else {
        return Ok(Keep(lhs, Some(1)));
    };

    let s = read_integer(reader)?;
    let rhs = if s.is_empty() {
        None
    } else {
        let num: usize = s.parse().map_err(Error::ParseInt)?;
        if num == 0 || num < lhs {
            return Err(Error::Custom(format!(
                "invalid character index range: {}-{}",
                lhs + 1,
                num
            )));
        }
        Some(num - lhs)
    };
    Ok(Keep(lhs, rhs))
}

fn read_until<R: Reader>(reader: &mut R, delim: char) -> Result<String, Error> {
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

fn unescape(s: String) -> Result<String, Error> {
    unescape::unescape(&s).ok_or(Error::Custom(format!(
        "unrecognized escape characters in '{}'",
        s
    )))
}
