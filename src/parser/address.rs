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

pub(crate) fn parse<R: Reader>(reader: &mut R) -> Result<Address, Error> {
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
        if let Some(',') = reader.peek()? {
            reader.next()?;
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

fn parse_brackets<R: Reader>(reader: &mut R) -> Result<Address, Error> {
    if let Some('(') = reader.peek()? {
        reader.next()?;
        skip_whitespace(reader);
        let addr = parse(reader)?;
        skip_whitespace(reader);
        if reader.next()? != Some(')') {
            return Err(Error::Missing(')'));
        }
        Ok(maybe_negate(addr, reader)?)
    } else {
        let addr = parse_range(reader)?;
        skip_whitespace(reader);
        Ok(maybe_negate(addr, reader)?)
    }
}

fn parse_range<R: Reader>(reader: &mut R) -> Result<Address, Error> {
    let addr = parse_simple_addr(reader)?;
    skip_whitespace(reader);
    if let Some('-') = reader.peek()? {
        let lhs = addr.unwrap_or(Location(1));
        reader.next()?;
        skip_whitespace(reader);
        let rhs = parse_simple_addr(reader)?.unwrap_or(Never);
        if let (Location(lo), Location(hi)) = (&lhs, &rhs) {
            if lo > hi {
                return Err(Error::InvalidAddr(format!(
                    "{} > {} in {}-{}",
                    lo, hi, lo, hi
                )));
            }
        }
        return Ok(Between(address::Between::new(lhs, rhs)));
    }
    Ok(addr.unwrap_or(Always))
}

fn parse_simple_addr<R: Reader>(reader: &mut R) -> Result<Option<Address>, Error> {
    if let Some(c) = reader.peek()? {
        match c {
            '#' => {
                skip_line(reader);
                skip_whitespace(reader);
                return parse_simple_addr(reader);
            }
            '/' | '^' => {
                let regex = parse_regex(reader)?;
                return Ok(Some(Regex(regex)));
            }
            c if c.is_ascii_digit() => {
                let s = read_integer(reader)?;
                return match s.parse() {
                    Ok(num) => {
                        if num == 0 {
                            return Err(Error::InvalidAddr(s));
                        }
                        Ok(Some(Location(num)))
                    }
                    Err(err) => Err(Error::ParseInt(err)),
                };
            }
            '*' => {
                reader.next()?;
                return Ok(Some(Always));
            }
            '$' => {
                reader.next()?;
                return Ok(Some(Never));
            }
            _ => (),
        }
    }
    Ok(None)
}

fn maybe_negate<R: Reader>(addr: Address, reader: &mut R) -> Result<Address, Error> {
    if let Some('!') = reader.peek()? {
        reader.next()?;
        Ok(!addr)
    } else {
        Ok(addr)
    }
}
