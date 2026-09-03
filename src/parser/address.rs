use std::str::FromStr;

use super::{read_integer, reader::Reader, regex, skip_line, skip_whitespace};
use crate::address::{
    self,
    Address::{self, *},
};
use anyhow::{Result, bail};

pub(crate) fn parse<R: Reader>(reader: &mut R) -> Result<Address> {
    set(reader)
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
            Always => {}
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
        if reader.next_is('+')? {
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
    let negated = reader.next_is('!')?;
    skip_whitespace(reader);
    let addr = between(reader)?;
    if negated {
        return Ok(!addr);
    }
    Ok(addr)
}

fn between<R: Reader>(reader: &mut R) -> Result<Address> {
    let addr = atom(reader)?;
    skip_whitespace(reader);
    if reader.next_is('-')? {
        let lhs = addr.unwrap_or(Location(1));
        skip_whitespace(reader);
        let rhs = atom(reader)?.unwrap_or(Final);
        if let (Location(lo), Location(hi)) = (&lhs, &rhs)
            && lo > hi
        {
            bail!("invalid bounds: {} > {} in {}-{}", lo, hi, lo, hi);
        }
        return Ok(Between(address::Between::new(lhs, rhs)));
    }
    Ok(addr.unwrap_or(Always))
}

fn atom<R: Reader>(reader: &mut R) -> Result<Option<Address>> {
    if let Some(c) = reader.peek()? {
        match c {
            '/' | '^' => {
                let regex = regex::read(reader)?;
                let addr = if regex.is_empty() {
                    Always
                } else {
                    Regex(crate::Regex::from_str(&regex)?)
                };
                return Ok(Some(addr));
            }
            c if c.is_ascii_digit() => {
                let s = read_integer(reader)?;
                match s.parse() {
                    Ok(num) => {
                        if num == 0 {
                            bail!("line numbering starts at 1");
                        }
                        return Ok(Some(Location(num)));
                    }
                    Err(err) => bail!(err),
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
            _ => (),
        }
    }
    Ok(None)
}

#[cfg(test)]
mod tests {
    use super::Address::{self, *};
    use crate::{address, parser::StringReader};
    use std::str::FromStr;
    use test_case::test_case;

    #[test_case("", Always; "empty")]
    #[test_case("()", Always; "empty brackets")]
    #[test_case("//", Always; "empty regex")]
    #[test_case("!//", Negate(Box::new(Always)); "negated empty regex")]
    #[test_case("!", Negate(Box::new(Always)); "negated empty")]
    #[test_case("$", Final; "finally")]
    #[test_case("!1-5", Negate(Box::new(Between(address::Between::new(Location(1), Location(5))))); "negated range")]
    #[test_case("(!(1-5))", Negate(Box::new(Between(address::Between::new(Location(1), Location(5))))); "brackets and negated range")]
    #[test_case("1,$", Set(vec![Location(1), Final]); "first or last")]
    #[test_case("1,!$", Set(vec![Location(1), Negate(Box::new(Final))]); "first or last negated")]
    #[test_case("!(1,$)", Negate(Box::new(Set(vec![Location(1), Final]))); "negate set in brackets")]
    #[test_case("/a/,/b/+(/c/,/d/),/e/",
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
    #[test_case("/a/+1-5+/b/",
        And(vec![
            Regex(FromStr::from_str("a").unwrap()),
            Between(address::Between::new(Location(1), Location(5))),
            Regex(FromStr::from_str("b").unwrap()),
        ]);
    "range and and")]
    fn parse(input: &str, expected: Address) {
        let mut reader = StringReader::from(input);
        let result = super::parse(&mut reader).unwrap();
        assert_eq!(result, expected)
    }
}
