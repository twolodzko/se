use super::{
    reader::Reader,
    utils::{parse_regex, read_integer, skip_line, skip_whitespace},
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
                cmds.push(Stop);
                break;
            }
            'p' => Println,
            'P' => Print,
            'l' => Escape,
            's' => parse_substitute(reader)?,
            'y' => parse_translate(reader)?,
            '=' => LineNumber,
            '\\' => match reader.next()? {
                Some('n') => Insert('\n'.to_string()),
                Some('t') => Insert('\t'.to_string()),
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

fn parse_translate<R: Reader>(reader: &mut R) -> Result<Command, Error> {
    if reader.next()? != Some('/') {
        return Err(Error::Missing('/'));
    }
    let src = read_translate_pattern(reader)?;
    let dst = read_translate_pattern(reader)?;
    if src.len() != dst.len() {
        return Err(Error::Custom(format!(
            "number of characters in /{}/ differs from /{}/",
            src, dst,
        )));
    }
    Ok(Translate(src, dst))
}

fn read_translate_pattern<R: Reader>(reader: &mut R) -> Result<String, Error> {
    let pattern = unescape(read_until(reader, '/')?)?;
    Ok(maybe_range(&pattern))
}

fn maybe_range(s: &str) -> String {
    let chars: Vec<char> = s.chars().collect();
    let mut acc: Vec<char> = Vec::new();
    let mut i = 0;
    while i + 2 < chars.len() {
        if chars[i + 1] == '-' {
            for c in range(chars[i], chars[i + 2]) {
                acc.push(c);
            }
            i += 3;
        } else {
            acc.push(chars[i]);
            i += 1;
        }
    }
    for c in chars[i..].iter() {
        acc.push(*c);
    }
    acc.iter().collect()
}

fn range(a: char, b: char) -> Box<dyn Iterator<Item = char>> {
    if a <= b {
        Box::new(a..=b)
    } else {
        Box::new((b..=a).rev())
    }
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
    unescape::unescape(&s).ok_or(Error::Custom(format!(
        "unrecognized escape characters in '{}'",
        s
    )))
}

#[cfg(test)]
mod tests {
    use super::maybe_range;
    use test_case::test_case;

    #[test_case(
        "",
        "";
        "empty"
    )]
    #[test_case(
        "a",
        "a";
        "one char"
    )]
    #[test_case(
        "ab",
        "ab";
        "two chars"
    )]
    #[test_case(
        "abc",
        "abc";
        "non-range"
    )]
    #[test_case(
        "a-",
        "a-";
        "minus at back"
    )]
    #[test_case(
        "-a",
        "-a";
        "minus at front"
    )]
    #[test_case(
        "-a-",
        "-a-";
        "minus at front and back"
    )]
    #[test_case(
        "ab",
        "ab";
        "short non-range"
    )]
    #[test_case(
        "a-d",
        "abcd";
        "range"
    )]
    #[test_case(
        "a-ce-g",
        "abcefg";
        "two ranges"
    )]
    #[test_case(
        "a-cde-ghijk",
        "abcdefghijk";
        "ranges mixed with chars"
    )]
    #[test_case(
        "abc-g",
        "abcdefg";
        "chars and range"
    )]
    fn expand_range(example: &str, expected: &str) {
        let result = maybe_range(example);
        assert_eq!(result, expected)
    }
}
