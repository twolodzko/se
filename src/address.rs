use crate::Line;
use std::sync::atomic;

#[derive(Debug, PartialEq)]
pub(crate) enum Address {
    // always matches
    Always,
    // never matches
    Final,
    // specific index
    Location(usize),
    // /regex/ matching the line
    Regex(crate::Regex),
    // addr! negates the addr match
    Negate(Box<Address>),
    // // addr1 - addr2
    Between(Between),
    // addr1, addr2, ...
    Set(Vec<Address>),
}

impl Address {
    pub(crate) fn matches(&self, line: &Line) -> bool {
        use Address::*;
        match self {
            Always => true,
            Final => false,
            Location(idx) => *idx == line.0,
            Regex(ref regex) => regex.0.is_match(&line.1),
            Negate(addr) => !addr.matches(line),
            Between(this) => this.matches(line),
            Set(set) => {
                for addr in set.iter() {
                    if addr.matches(line) {
                        return true;
                    }
                }
                false
            }
        }
    }
}

#[derive(Debug)]
pub(crate) struct Between {
    lhs: Box<Address>,
    rhs: Box<Address>,
    inside: atomic::AtomicBool,
}

impl Between {
    pub(crate) fn new(lhs: Address, rhs: Address) -> Self {
        Between {
            lhs: Box::new(lhs),
            rhs: Box::new(rhs),
            inside: atomic::AtomicBool::new(false),
        }
    }

    pub(crate) fn matches(&self, line: &Line) -> bool {
        if self.inside.load(atomic::Ordering::Relaxed) {
            if self.rhs.matches(line) {
                self.inside.store(false, atomic::Ordering::Relaxed)
            }
            true
        } else {
            if self.lhs.matches(line) {
                if !self.rhs.matches(line) {
                    self.inside.store(true, atomic::Ordering::Relaxed)
                }
                return true;
            }
            false
        }
    }
}

impl PartialEq for Between {
    fn eq(&self, other: &Self) -> bool {
        self.lhs == other.lhs && self.rhs == other.rhs
    }
}

impl std::ops::Not for Address {
    type Output = Address;

    fn not(self) -> Self::Output {
        use Address::*;
        match self {
            Negate(inner) => *inner,
            _ => Negate(Box::new(self)),
        }
    }
}

impl std::fmt::Display for Address {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use Address::*;
        match self {
            Always => write!(f, "//"),
            Final => write!(f, "$"),
            Location(idx) => write!(f, "{}", idx),
            Regex(regex) => write!(f, "/{}/", regex),
            Negate(addr) => write!(f, "{}!", addr),
            Between(this) => write!(f, "{}-{}", this.lhs, this.rhs),
            Set(addrs) => {
                let list = addrs
                    .iter()
                    .map(|a| a.to_string())
                    .collect::<Vec<String>>()
                    .join(", ");
                write!(f, "{}", list)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        address::Address::{self, *},
        parser::StringReader,
        Line,
    };
    use std::str::FromStr;
    use test_case::test_case;

    #[test_case(Always, Line(1, "".to_string()), true; "any matches line 1")]
    #[test_case(Always, Line(279, "".to_string()), true; "any matches line 279")]
    #[test_case(Negate(Box::new(Always)), Line(1, "".to_string()), false; "negated any does not match line 1")]
    #[test_case(Negate(Box::new(Always)), Line(279, "".to_string()), false; "negated any does not match line 279")]
    #[test_case(Location(1), Line(1, "".to_string()), true; "index 1 matches line 1")]
    #[test_case(Location(1), Line(279, "".to_string()), false; "index 1 does not match line 279")]
    #[test_case(
        Regex(crate::Regex::from_str("abc").unwrap()),
        Line(1, "abc".to_string()),
        true;
        "regex abc matches line abc"
    )]
    #[test_case(
        Regex(crate::Regex::from_str("abc").unwrap()),
        Line(1, "hello, world!".to_string()),
        false;
        "regex abc does not match line hello"
    )]
    #[test_case(
        Set(vec![Location(1), Location(2), Location(3)]),
        Line(1, "".to_string()),
        true;
        "set 1,2,3 matches line 1"
    )]
    #[test_case(
        Set(vec![Location(1), Location(2), Location(3)]),
        Line(279, "".to_string()),
        false;
        "set 1,2,3 does not match line 279"
    )]
    fn matches(addr: Address, line: Line, expected: bool) {
        assert_eq!(addr.matches(&line), expected)
    }

    #[test_case(
        "//",
        vec![true, true, true, true, true, true, true, true, true, true];
        "any"
    )]
    #[test_case(
        "//!",
        vec![false, false, false, false, false, false, false, false, false, false];
        "any negated"
    )]
    #[test_case(
        "7",
        vec![false, false, false, false, false, false, true, false, false, false];
        "index 7"
    )]
    #[test_case(
        "89",
        vec![false, false, false, false, false, false, false, false, false, false];
        "index 89"
    )]
    #[test_case(
        "2,5,9",
        vec![false, true, false, false, true, false, false, false, true, false];
        "set of indexes"
    )]
    #[test_case(
        "2-7",
        vec![false, true, true, true, true, true, true, false, false, false];
        "range of indexes 2:7"
    )]
    #[test_case(
        "1-1",
        vec![true, false, false, false, false, false, false, false, false, false];
        "range of indexes 1:1"
    )]
    #[test_case(
        "1-5",
        vec![true, true, true, true, true, false, false, false, false, false];
        "left-open range of indexes"
    )]
    #[test_case(
        "/aa/",
        vec![false, false, true, false, true, true, false, false, false, false];
        "regex aa"
    )]
    #[test_case(
        "/start/-/end/",
        vec![false, true, true, true, false, true, true, false, false, false];
        "regex range matches twice"
    )]
    #[test_case(
        "5-/123/",
        vec![false, false, false, false, true, true, true, true, true, false];
        "mixed range"
    )]
    #[test_case(
        "6-$",
        vec![false, false, false, false, false, true, true, true, true, true];
        "half-open range"
    )]
    fn multiline_example(addr: &str, expected: Vec<bool>) {
        let example = r"
            start
            aaa
            end
            zzz aa bb c
            start aabcd
            def end

            123
        ";
        let mut reader = StringReader::from(addr);
        let mut addr = crate::parser::address::parse(&mut reader).unwrap();
        assert_eq!(
            example
                .lines()
                .enumerate()
                .map(|(i, s)| {
                    let line = Line(i + 1, s.to_string());
                    (&mut addr).matches(&line)
                })
                .collect::<Vec<bool>>(),
            expected
        )
    }
}
