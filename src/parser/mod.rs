pub(crate) mod address;
mod command;
mod program;
mod reader;
mod regex;

use crate::{Error, Result, address::Address, error};
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
    fn is_final(&self) -> bool {
        use Address::*;
        match self {
            Final => true,
            Extend(extend) => extend.start.is_final(),
            Set(set) => set.iter().any(|a| a.is_final()),
            Between(between) => between.start.is_final(),
            _ => false,
        }
    }

    fn is_regular(&self) -> bool {
        if let Address::Set(set) = self {
            return set.iter().any(|a| a.is_regular());
        }
        !self.is_final()
    }

    fn is_impossible(&self) -> bool {
        use Address::*;
        match self {
            And(and) => and.iter().any(|a| a.is_final() || a.is_impossible()),
            Negate(not) => not.is_final(),
            Extend(extend) => extend.start.is_impossible(),
            Between(between) => between.start.is_impossible(),
            _ => false,
        }
    }

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
            Set(set) => set.iter_mut().try_for_each(|a| a.replace_maybe(subst))?,
            And(and) => and.iter_mut().try_for_each(|a| a.replace_maybe(subst))?,
            Extend(extend) => extend.start.replace_maybe(subst)?,
            _ => {}
        }
        Ok(())
    }

    fn simplify(&mut self) -> Result<()> {
        use Address::*;
        match self {
            Final => *self = Never,
            Set(set) => {
                let mut i = 0;
                while i < set.len() {
                    set[i].simplify()?;
                    if set[i] == Never {
                        set.remove(i);
                    } else {
                        i += 1;
                    }
                }
                merge_regex(set)?;
                match set.len() {
                    0 => *self = Never,
                    1 => *self = set.remove(0),
                    _ => {}
                }
            }
            And(and) => {
                for addr in and.iter_mut() {
                    addr.simplify()?;
                    if *addr == Never {
                        *self = Never;
                        break;
                    }
                }
            }
            Extend(extend) => {
                extend.start.simplify()?;
                if *extend.start == Never {
                    *self = Never
                }
            }
            Between(between) => {
                between.start.simplify()?;
                if *between.start == Never {
                    *self = Never
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
        let mut s = regex.0.as_str().to_string();
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
