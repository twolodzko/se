use crate::program::Memory;
use std::cell::Cell;

#[derive(Debug, PartialEq)]
pub(crate) enum Address {
    /// always matches
    Always,
    /// never matches
    Final,
    /// specific index
    Location(usize),
    /// /regex/ matching the line
    Regex(crate::Regex),
    /// !addr negates the addr match
    Negate(Box<Address>),
    /// addr1 - addr2
    Between(Between),
    /// start ~ step
    Nth(usize, usize),
    /// addr + window
    Extend(Extend),
    /// addr1, addr2, ...
    Set(Vec<Address>),
    /// addr1 & addr2 & ...
    And(Vec<Address>),
    /// ?
    Maybe,
}

impl Address {
    pub(crate) fn matches(&self, memory: &Memory) -> bool {
        use Address::*;
        match self {
            Always => true,
            Final => false,
            Location(idx) => *idx == memory.line.0,
            Regex(regex) => regex.0.is_match(&memory.this),
            Negate(addr) => !addr.matches(memory),
            Between(this) => this.matches(memory),
            Nth(start, step) => {
                if memory.line.0 < *start {
                    false
                } else {
                    (memory.line.0 - *start).is_multiple_of(*step)
                }
            }
            Extend(this) => this.matches(memory),
            Set(set) => {
                for addr in set.iter() {
                    if addr.matches(memory) {
                        return true;
                    }
                }
                false
            }
            And(and) => {
                for addr in and.iter() {
                    if !addr.matches(memory) {
                        return false;
                    }
                }
                true
            }
            Maybe => unreachable!(),
        }
    }
}

#[derive(Debug)]
pub(crate) struct Between {
    pub(crate) start: Box<Address>,
    pub(crate) end: Box<Address>,
    inside: Cell<bool>,
}

impl Between {
    pub(crate) fn new(lhs: Address, rhs: Address) -> Self {
        Between {
            start: Box::new(lhs),
            end: Box::new(rhs),
            inside: Cell::new(false),
        }
    }

    pub(crate) fn matches(&self, memory: &Memory) -> bool {
        if self.inside.get() {
            if self.end.matches(memory) {
                self.inside.set(false)
            }
            true
        } else {
            if self.start.matches(memory) {
                if !self.end.matches(memory) {
                    self.inside.set(true)
                }
                return true;
            }
            false
        }
    }
}

impl PartialEq for Between {
    fn eq(&self, other: &Self) -> bool {
        self.start == other.start && self.end == other.end
    }
}

#[derive(Debug)]
pub(crate) struct Extend {
    pub(crate) start: Box<Address>,
    pub(crate) size: usize,
    count: Cell<usize>,
}

impl Extend {
    pub(crate) fn new(start: Address, size: usize) -> Extend {
        Extend {
            start: Box::new(start),
            size,
            count: Cell::new(0),
        }
    }

    pub(crate) fn matches(&self, memory: &Memory) -> bool {
        match self.count.get() {
            0 => {
                if self.start.matches(memory) {
                    self.count.set(self.size);
                    true
                } else {
                    false
                }
            }
            n => {
                self.count.set(n - 1);
                true
            }
        }
    }
}

impl PartialEq for Extend {
    fn eq(&self, other: &Self) -> bool {
        self.start == other.start && self.size == other.size
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
            Negate(addr) => write!(f, "!{}", addr),
            Between(this) => write!(f, "{}-{}", this.start, this.end),
            Nth(start, step) => write!(f, "{}~{}", start, step),
            Extend(window) => write!(f, "{}+{}", window.start, window.size),
            Set(addrs) => {
                let list = addrs
                    .iter()
                    .map(|a| a.to_string())
                    .collect::<Vec<String>>()
                    .join(", ");
                write!(f, "{}", list)
            }
            And(addrs) => {
                let list = addrs
                    .iter()
                    .map(|a| a.to_string())
                    .collect::<Vec<String>>()
                    .join(" + ");
                write!(f, "{}", list)
            }
            Maybe => write!(f, "?"),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        Line,
        address::Address::{self, *},
        parser::StringReader,
        program::Memory,
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
        let mut memory = Memory::default();
        memory.read(line);
        assert_eq!(addr.matches(&memory), expected)
    }

    #[test_case(
        "//",
        vec![true, true, true, true, true, true, true, true, true, true];
        "any"
    )]
    #[test_case(
        "()",
        vec![true, true, true, true, true, true, true, true, true, true];
        "empty brackets"
    )]
    #[test_case(
        "!//",
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
        "(2,3)-(7,8)",
        vec![false, true, true, true, true, true, true, false, false, false];
        "range containing brackets"
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
    #[test_case(
        "/a/&/b/",
        vec![false, false, false, false, true, true, false, false, false, false];
        "and"
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
        let addr = crate::parser::address::parse(&mut reader).unwrap();
        assert_eq!(
            example
                .lines()
                .enumerate()
                .map(|(i, s)| {
                    let line = Line(i + 1, s.to_string());
                    let mut memory = Memory::default();
                    memory.read(line);
                    addr.matches(&memory)
                })
                .collect::<Vec<bool>>(),
            expected
        )
    }
}
