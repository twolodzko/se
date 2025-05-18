use super::{
    reader::Reader,
    utils::{parse_regex, read_integer, skip_line, skip_whitespace},
};
use crate::{
    command::Command::{self, *},
    Error, Regex,
};

pub(crate) fn parse<R: Reader>(reader: &mut R) -> Result<Vec<Command>, Error> {
    let mut cmds = Vec::new();
    while let Some(c) = reader.next()? {
        let cmd = match c {
            ';' => break,
            '.' => {
                cmds.push(Stop);
                break;
            }
            'p' => Println,
            'P' => Print,
            'l' => Escape,
            's' => parse_substitute(reader)?,
            #[cfg(feature = "extract")]
            '`' => parse_extract(reader)?,
            #[cfg(feature = "extract")]
            '{' => parse_json_extract(reader)?,
            '=' => LineNumber,
            '\\' => match reader.next()? {
                Some('n') => Insert('\n'.to_string()),
                Some('t') => Insert('\t'.to_string()),
                Some('s') => Insert(' '.to_string()),
                Some(c) => Insert(c.to_string()),
                None => return Err(Error::Unexpected('\\')),
            },
            'd' => Delete,
            'z' => Reset,
            'h' | 'c' => Copy,
            'g' | 'v' => Paste,
            'x' => Exchange,
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
    }
    Ok(cmds)
}

fn parse_substitute<R: Reader>(reader: &mut R) -> Result<Command, Error> {
    if reader.peek()? != Some('/') {
        return Err(Error::Missing('/'));
    }

    // Parse: s/src/dst/[limit]
    let src = parse_regex(reader)?;
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

#[cfg(feature = "extract")]
fn parse_extract<R: Reader>(reader: &mut R) -> Result<Command, Error> {
    let regex = Regex::new(&read_until(reader, '`')?)?;

    if regex.0.captures_len() > 2 {
        eprintln!(
            "Warning: regular expression '{}' contains more then one capturing group",
            regex
        );
    }

    let mut index = 0;
    if let Some(c) = reader.peek()? {
        let s = read_integer(reader)?;
        if c.is_ascii_digit() {
            index = if s.is_empty() {
                0
            } else {
                let num = s.parse::<usize>().map_err(Error::ParseInt)?;
                if num == 0 {
                    return Err(Error::Custom(format!("invalid index")));
                }
                num - 1
            };
        }
    }

    Ok(Extract(regex, index))
}

#[cfg(feature = "extract")]
fn parse_json_extract<R: Reader>(reader: &mut R) -> Result<Command, Error> {
    let regex = Regex::new(&read_until(reader, '}')?)?;
    let names = regex
        .0
        .capture_names()
        .filter_map(|name| Some(name?.to_string()))
        .collect::<Vec<String>>();

    if names.is_empty() {
        return Err(Error::Custom(format!(
            "Warning: regular expression '{}' contains no named capturing groups",
            regex
        )));
    }

    let mut index = 0;
    if let Some(c) = reader.peek()? {
        let s = read_integer(reader)?;
        if c.is_ascii_digit() {
            index = if s.is_empty() {
                0
            } else {
                s.parse::<usize>().map_err(Error::ParseInt)?
            };
        }
    }

    Ok(JsonExtract(regex, names, index))
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
    unescape::unescape(&s).ok_or(Error::ParsingError(s))
}
