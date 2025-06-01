use super::{
    reader::Reader,
    utils::{parse_regex, read_integer, skip_line, skip_whitespace},
};
use crate::{
    address::{
        self,
        Address::{self, *},
    },
    Error,
};
use anyhow::{bail, Result};

pub(crate) fn parse<R: Reader>(reader: &mut R) -> Result<Address> {
    let mut addrs = Vec::new();
    let mut has_any = false;
    loop {
        let mut addr = parse_brackets(reader)?;
        match addr {
            Always => has_any = true,
            Set(ref mut rhs) => addrs.append(rhs),
            _ => addrs.push(addr),
        }

        skip_whitespace(reader);
        if reader.next_is(',')? {
            skip_whitespace(reader);
        } else {
            break;
        }
    }

    // optimizations
    if has_any {
        return Ok(Always);
    }
    if addrs.len() == 1 {
        return Ok(addrs.remove(0));
    }
    Ok(Set(addrs))
}

fn parse_brackets<R: Reader>(reader: &mut R) -> Result<Address> {
    if reader.next_is('(')? {
        skip_whitespace(reader);
        let addr = parse(reader)?;
        skip_whitespace(reader);
        if reader.next()? != Some(')') {
            bail!(Error::Missing(')'))
        }
        Ok(maybe_negate(addr, reader)?)
    } else {
        let addr = parse_range(reader)?;
        skip_whitespace(reader);
        Ok(maybe_negate(addr, reader)?)
    }
}

fn parse_range<R: Reader>(reader: &mut R) -> Result<Address> {
    let addr = parse_simple_addr(reader)?;
    skip_whitespace(reader);
    if reader.next_is('-')? {
        let lhs = addr.unwrap_or(Location(1));
        skip_whitespace(reader);
        let rhs = parse_simple_addr(reader)?.unwrap_or(Final);
        if let (Location(lo), Location(hi)) = (&lhs, &rhs) {
            if lo > hi {
                bail!(Error::InvalidAddr(format!(
                    "{} > {} in {}-{}",
                    lo, hi, lo, hi
                )));
            }
        }
        return Ok(Between(address::Between::new(lhs, rhs)));
    }
    Ok(addr.unwrap_or(Always))
}

fn parse_simple_addr<R: Reader>(reader: &mut R) -> Result<Option<Address>> {
    if let Some(c) = reader.peek()? {
        match c {
            '#' => {
                skip_line(reader);
                skip_whitespace(reader);
                return parse_simple_addr(reader);
            }
            '/' | '^' => {
                let addr = match parse_regex(reader)? {
                    Some(regex) => Regex(regex),
                    None => Always,
                };
                return Ok(Some(addr));
            }
            c if c.is_ascii_digit() => {
                let s = read_integer(reader)?;
                match s.parse() {
                    Ok(num) => {
                        if num == 0 {
                            bail!(Error::InvalidAddr(s));
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
            _ => (),
        }
    }
    Ok(None)
}

fn maybe_negate<R: Reader>(addr: Address, reader: &mut R) -> Result<Address> {
    if reader.next_is('!')? {
        Ok(!addr)
    } else {
        Ok(addr)
    }
}

#[cfg(test)]
mod tests {
    use super::Address::{self, *};
    use crate::{address, parser::StringReader};
    use test_case::test_case;

    #[test_case("", Always; "empty")]
    #[test_case("()", Always; "empty brackets")]
    #[test_case("//", Always; "empty regex")]
    #[test_case("//!", Negate(Box::new(Always)); "negated empty regex")]
    #[test_case("!", Negate(Box::new(Always)); "negated empty")]
    #[test_case("$", Final; "finally")]
    #[test_case("1-5!", Negate(Box::new(Between(address::Between::new(Location(1), Location(5))))); "negated range")]
    #[test_case("((1-5)!)", Negate(Box::new(Between(address::Between::new(Location(1), Location(5))))); "brackets and negated range")]
    #[test_case("1,$", Set(vec![Location(1), Final]); "first or last")]
    #[test_case("1,$!", Set(vec![Location(1), Negate(Box::new(Final))]); "first or last negated")]
    #[test_case("(1,$)!", Negate(Box::new(Set(vec![Location(1), Final]))); "negate set in brackets")]
    fn parse(input: &str, expected: Address) {
        let mut reader = StringReader::from(input);
        let result = super::parse(&mut reader).unwrap();
        assert_eq!(result, expected)
    }
}
