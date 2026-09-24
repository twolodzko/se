pub(crate) mod address;
mod command;
mod program;
mod reader;
mod regex;

use crate::{Result, address::Address, error};
use reader::Reader;
#[cfg(test)]
pub(crate) use reader::StringReader;
use std::str::FromStr;

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

impl Address {
    fn replace_maybe(&mut self, subst: Option<&crate::Regex>) -> Result<()> {
        use Address::*;
        match self {
            Maybe => {
                let Some(regex) = subst else {
                    return error!("{} must be followed by a substitution", self);
                };
                *self = Regex(regex.clone());
            }
            Between(between) => {
                between.start.replace_maybe(subst)?;
                between.end.replace_maybe(subst)?;
            }
            Set(set) => set
                .addresses
                .iter_mut()
                .try_for_each(|a| a.replace_maybe(subst))?,
            And(and) => and
                .addresses
                .iter_mut()
                .try_for_each(|a| a.replace_maybe(subst))?,
            Extend(extend) => extend.start.replace_maybe(subst)?,
            Negate(not) => not.replace_maybe(subst)?,
            _ => {}
        }
        Ok(())
    }

    fn simplify(&mut self) -> Result<()> {
        use Address::*;
        match self {
            Set(set) => {
                let mut i = 0;
                while i < set.addresses.len() {
                    set.addresses[i].simplify()?;
                    if set.addresses[i] == Never {
                        set.addresses.remove(i);
                    } else {
                        i += 1;
                    }
                }
                merge_regex(&mut set.addresses)?;
                match set.addresses.len() {
                    0 => *self = Never,
                    1 => *self = set.addresses.remove(0),
                    _ => {}
                }
            }
            And(and) => {
                for addr in and.addresses.iter_mut() {
                    addr.simplify()?;
                    if *addr == Never {
                        *self = Never;
                        break;
                    }
                }
            }
            Extend(extend) => {
                extend.start.simplify()?;
                if matches!(*extend.start, Never | Final) {
                    *self = *extend.start.clone();
                }
            }
            Between(between) => {
                between.start.simplify()?;
                if matches!(*between.start, Never | Final) {
                    *self = *between.start.clone();
                }
            }
            _ => {}
        }
        Ok(())
    }
}

/// Merge `/a/,/b/,/c/` to `/a|b|c/`
fn merge_regex(addrs: &mut Vec<Address>) -> Result<()> {
    if let Some((first, regex)) = addrs.iter().enumerate().find_map(|(i, a)| {
        if let Address::Regex(r) = a {
            Some((i, r))
        } else {
            None
        }
    }) {
        let mut s = regex.to_string();
        let mut i = first + 1;
        let mut found_more = false;
        while i < addrs.len() {
            if let Address::Regex(r) = &addrs[i] {
                found_more = true;
                s.push('|');
                s.push_str(r.0.as_str());
                addrs.remove(i);
            } else {
                i += 1;
            }
        }
        if found_more {
            let regex = crate::Regex::from_str(&s)?;
            addrs[first] = Address::Regex(regex)
        }
    }
    Ok(())
}
