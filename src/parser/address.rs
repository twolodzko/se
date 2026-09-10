use super::{read_integer, reader::Reader, regex, skip_line, skip_whitespace};
use crate::{
    Error, Result,
    address::{
        self,
        Address::{self, *},
    },
    error,
};
use std::str::FromStr;

pub(crate) fn parse<R: Reader>(reader: &mut R) -> Result<Address> {
    let addr = set(reader)?;
    if addr.is_impossible() {
        return Err(Error::Impossible(addr));
    }
    Ok(addr)
}

fn set<R: Reader>(reader: &mut R) -> Result<Address> {
    let mut acc = Vec::new();
    loop {
        if reader.next_is('#')? {
            skip_line(reader);
            skip_whitespace(reader);
            continue;
        }
        let mut addr = and(reader)?;
        match addr {
            Set(ref mut rhs) => acc.append(rhs),
            _ => acc.push(addr),
        }

        skip_whitespace(reader);
        if reader.next_is(',')? {
            skip_whitespace(reader);
        } else {
            break;
        }
    }

    if acc.contains(&Always) {
        return Ok(Always);
    }
    let addr = match acc.len() {
        0 => Always,
        1 => acc.remove(0),
        _ => Set(acc),
    };
    Ok(addr)
}

fn and<R: Reader>(reader: &mut R) -> Result<Address> {
    let mut acc = Vec::new();
    loop {
        if reader.next_is('#')? {
            skip_line(reader);
            skip_whitespace(reader);
            continue;
        }
        let mut addr = address(reader)?;
        match addr {
            Always => {}
            And(ref mut rhs) => acc.append(rhs),
            _ => acc.push(addr),
        }

        skip_whitespace(reader);
        if reader.next_is('&')? {
            skip_whitespace(reader);
        } else {
            break;
        }
    }

    let addr = match acc.len() {
        0 => Always,
        1 => acc.remove(0),
        _ => And(acc),
    };
    Ok(addr)
}

fn address<R: Reader>(reader: &mut R) -> Result<Address> {
    skip_whitespace(reader);
    between(reader)
}

fn between<R: Reader>(reader: &mut R) -> Result<Address> {
    let addr = negated(reader)?;
    skip_whitespace(reader);
    if let Some(c) = reader.peek()? {
        match c {
            '-' => {
                reader.skip();
                let start = addr.unwrap_or(Location(1));
                skip_whitespace(reader);
                let end = negated(reader)?.unwrap_or(Final);
                if let (Location(lo), Location(hi)) = (&start, &end)
                    && lo > hi
                {
                    return error!("invalid bounds: {} > {} in {}-{}", lo, hi, lo, hi);
                }
                if start == end {
                    return Ok(start);
                }
                return Ok(Between(address::Between::new(start, end)));
            }
            '~' => {
                reader.skip();
                let start = if let Some(a) = addr {
                    if let Location(n) = a
                        && n > 0
                    {
                        n
                    } else {
                        return error!("invalid start: {}", a);
                    }
                } else {
                    1
                };
                skip_whitespace(reader);
                let step = if let Some(c) = reader.peek()?
                    && c.is_ascii_digit()
                {
                    let s = read_integer(reader)?;
                    usize::from_str(&s).map_err(Error::from)?
                } else {
                    1
                };
                return Ok(Nth(start, step));
            }
            '+' => {
                reader.skip();
                let start = addr.unwrap_or(Location(1));
                skip_whitespace(reader);
                let s = read_integer(reader)?;
                let size = usize::from_str(&s)?;
                if size == 0 {
                    return Ok(start);
                }
                return Ok(Extend(address::Extend::new(start, size)));
            }
            _ => {}
        }
    }
    Ok(addr.unwrap_or(Always))
}

fn negated<R: Reader>(reader: &mut R) -> Result<Option<Address>> {
    let negated = reader.next_is('!')?;
    skip_whitespace(reader);
    let addr = atom(reader)?;
    if negated {
        return if let Some(a) = addr {
            Ok(Some(!a))
        } else {
            Ok(Some(Never))
        };
    }
    Ok(addr)
}

fn atom<R: Reader>(reader: &mut R) -> Result<Option<Address>> {
    if let Some(c) = reader.peek()? {
        match c {
            '/' | '^' | '\\' => {
                if c == '\\' {
                    reader.skip();
                }
                let (regex, _) = regex::read(reader)?;
                let addr = if regex.is_empty() {
                    Always
                } else {
                    Regex(crate::Regex::from_str(&regex)?)
                };
                return Ok(Some(addr));
            }
            c if c.is_ascii_digit() => {
                return match read_integer(reader)?.parse() {
                    Ok(0) => error!("line numbering starts at 1"),
                    Ok(num) => Ok(Some(Location(num))),
                    Err(err) => Err(err.into()),
                };
            }
            '$' => {
                reader.skip();
                return Ok(Some(Final));
            }
            '?' => {
                reader.skip();
                return Ok(Some(Maybe));
            }
            '(' => {
                reader.skip();
                skip_whitespace(reader);
                let addr = set(reader)?;
                skip_whitespace(reader);
                reader.expect(')')?;
                return Ok(Some(addr));
            }
            _ => {}
        }
    }
    Ok(None)
}

#[cfg(test)]
mod tests {
    use super::Address::{self, *};
    use crate::{
        address::{self, Extend},
        parser::StringReader,
    };
    use std::str::FromStr;
    use test_case::test_case;

    #[test_case("", Always; "empty")]
    #[test_case("()", Always; "empty brackets")]
    #[test_case("//", Always; "empty regex")]
    #[test_case("!", Never; "never")]
    #[test_case("!//", Never; "not always")]
    #[test_case("!(!//)", Always; "double negation")]
    #[test_case("$", Final; "finally")]
    #[test_case("//-!", Between(address::Between::new(Always, Never)); "between always and never")]
    #[test_case("-5", Between(address::Between::new(Location(1), Location(5))); "left-open range")]
    #[test_case("5-", Between(address::Between::new(Location(5), Final)); "right-open range")]
    #[test_case("7~2", Nth(7, 2); "nth")]
    #[test_case("!(1-5)", Negate(Box::new(Between(address::Between::new(Location(1), Location(5))))); "negated range")]
    #[test_case("(!(1-5))", Negate(Box::new(Between(address::Between::new(Location(1), Location(5))))); "brackets and negated range")]
    #[test_case("1,$", Set(vec![Location(1), Final]); "first or last")]
    #[test_case("1,/foo/,//", Always; "set containing always reduces")]
    #[test_case("!(1,5)", Negate(Box::new(Set(vec![Location(1), Location(5)]))); "negate set in brackets")]
    #[test_case("1+5", Extend(Extend::new(Location(1), 5)); "extend location")]
    #[test_case("!,1", Set(vec![Never, Location(1)]); "set with never")]
    #[test_case("! & //", Never; "never and always")]
    #[test_case("/a/,/b/&(/c/,/d/),/e/",
        Set(vec![
            Regex(FromStr::from_str("a").unwrap()),
            And(vec![
                Regex(FromStr::from_str("b").unwrap()),
                Set(vec![
                    Regex(FromStr::from_str("c").unwrap()),
                    Regex(FromStr::from_str("d").unwrap()),
                ]),
            ]),
            Regex(FromStr::from_str("e").unwrap()),
        ]);
      "set and and together")]
    #[test_case("/a/ & 1-5 & /b/",
        And(vec![
            Regex(FromStr::from_str("a").unwrap()),
            Between(address::Between::new(Location(1), Location(5))),
            Regex(FromStr::from_str("b").unwrap()),
        ]);
    "range and and")]
    #[test_case("(1 & 2) & ((3 & 4) & 5)",
        And(vec![
            Location(1),
            Location(2),
            Location(3),
            Location(4),
            Location(5),
        ]);
    "nested and"
    )]
    #[test_case("(1, 2), ((3, 4), 5)",
        Set(vec![
            Location(1),
            Location(2),
            Location(3),
            Location(4),
            Location(5),
        ]);
    "nested or"
    )]
    fn parse(input: &str, expected: Address) {
        let mut reader = StringReader::from(input);
        let result = super::parse(&mut reader).unwrap();
        assert_eq!(result, expected)
    }

    #[test_case("!$ p"; "not final")]
    #[test_case("!(!(!$)) p"; "triple negated final")]
    #[test_case("(!$)+5 p"; "extended not final")]
    #[test_case("!($+5) p"; "negated not final extended")]
    #[test_case("5 & $ p"; "and final")]
    #[test_case("5 & !$ p"; "and not final")]
    fn fail_on_parse_impossibility(input: &str) {
        let mut reader = StringReader::from(input);
        let result = super::parse(&mut reader);
        assert!(matches!(result, Err(crate::Error::Impossible(_))))
    }
}
