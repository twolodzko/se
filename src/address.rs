use crate::Memory;
use std::cell::Cell;

#[derive(Debug, PartialEq, Clone)]
pub(crate) enum Address {
    /// matches on any line
    Always,
    /// never matches
    Never,
    /// marks block that executes after processing files, never matches
    Final,
    /// specific index
    Location(usize),
    /// /regex/ matching the line
    Regex(crate::Regex),
    /// addr! negates the addr match
    Negate(Box<Address>),
    /// addr1 - addr2
    Between(Between),
    /// start ~ step
    Nth(usize, usize),
    /// addr + window
    Extend(Extend),
    /// addr1 | addr2, ...
    Set(Set),
    /// addr1 & addr2 & ...
    And(And),
    /// ?
    Maybe,
}

impl Address {
    pub(crate) fn is_match(&self, memory: &Memory) -> bool {
        use Address::*;
        match self {
            Always => memory.index != 0,
            Never => false,
            Final => memory.end,
            Location(idx) => memory.index == *idx,
            Regex(regex) => memory.index != 0 && regex.0.is_match(&memory.this),
            Negate(addr) => !addr.is_match(memory) && memory.index != 0,
            Between(between) => between.is_match(memory),
            Nth(start, step) => {
                if memory.index < *start {
                    false
                } else {
                    (memory.index - *start).is_multiple_of(*step)
                }
            }
            Extend(extend) => extend.is_match(memory),
            Set(set) => set.is_match(memory),
            And(and) => and.is_match(memory),
            Maybe => unreachable!(),
        }
    }

    pub(crate) fn is_final(&self) -> bool {
        use Address::*;
        match self {
            Final => true,
            Extend(extend) => extend.start.is_final(),
            Set(set) => set.is_final(),
            And(and) => and.addresses.iter().any(|a| a.is_final()),
            Between(between) => between.start.is_final(),
            _ => false,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
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

    pub(crate) fn is_match(&self, memory: &Memory) -> bool {
        if self.inside.get() {
            if self.end.is_match(memory) {
                self.inside.set(false)
            }
            true
        } else {
            if self.start.is_match(memory) {
                if !self.end.is_match(memory) {
                    self.inside.set(true)
                }
                return true;
            }
            false
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
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

    pub(crate) fn is_match(&self, memory: &Memory) -> bool {
        match self.count.get() {
            0 => {
                if self.start.is_match(memory) {
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

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Set {
    pub(crate) addresses: Vec<Address>,
    seen: Cell<bool>,
}

impl Set {
    pub(crate) fn is_match(&self, memory: &Memory) -> bool {
        let mut ok = false;
        for addr in self.addresses.iter() {
            if addr.is_match(memory) {
                ok = true;
                break;
            }
        }
        self.seen.set(ok);
        ok
    }

    fn is_final(&self) -> bool {
        // if it was already used, it doesn't matter
        // that it contains the final address
        self.addresses.iter().any(|a| a.is_final()) && !self.seen.get()
    }
}

impl From<Vec<Address>> for Set {
    fn from(value: Vec<Address>) -> Self {
        Set {
            addresses: value,
            seen: Cell::new(false),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct And {
    pub(crate) addresses: Vec<Address>,
}

impl And {
    pub(crate) fn is_match(&self, memory: &Memory) -> bool {
        for addr in self.addresses.iter() {
            if !addr.is_match(memory) {
                return false;
            }
        }
        true
    }
}

impl From<Vec<Address>> for And {
    fn from(value: Vec<Address>) -> Self {
        And { addresses: value }
    }
}

impl std::ops::Not for Address {
    type Output = Address;

    fn not(self) -> Self::Output {
        use Address::*;
        match self {
            Always => Never,
            Never => Always,
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
            Never => write!(f, "!"),
            Final => write!(f, "$"),
            Location(idx) => write!(f, "{}", idx),
            Regex(regex) => write!(f, "/{}/", regex),
            Negate(addr) => {
                if matches!(
                    addr.as_ref(),
                    Between(_) | Nth(_, _) | Extend(_) | And(_) | Set(_)
                ) {
                    write!(f, "({})!", addr)
                } else {
                    write!(f, "{}!", addr)
                }
            }
            Between(this) => write!(f, "{}-{}", this.start, this.end),
            Nth(start, step) => write!(f, "{}~{}", start, step),
            Extend(window) => write!(f, "{}+{}", window.start, window.size),
            Set(set) => {
                let list = set
                    .addresses
                    .iter()
                    .map(|a| a.to_string())
                    .collect::<Vec<String>>()
                    .join(" | ");
                write!(f, "{}", list)
            }
            And(and) => {
                let list = and
                    .addresses
                    .iter()
                    .map(|a| a.to_string())
                    .collect::<Vec<String>>()
                    .join(" & ");
                write!(f, "{}", list)
            }
            Maybe => write!(f, "?"),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        Line, Memory,
        address::Address::{self, *},
        parser::StringReader,
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
        Set(vec![Location(1), Location(2), Location(3)].into()),
        Line(1, "".to_string()),
        true;
        "set 1,2,3 matches line 1"
    )]
    #[test_case(
        Set(vec![Location(1), Location(2), Location(3)].into()),
        Line(279, "".to_string()),
        false;
        "set 1,2,3 does not match line 279"
    )]
    fn matches(addr: Address, line: Line, expected: bool) {
        let mut memory = Memory::default();
        memory.read(line);
        assert_eq!(addr.is_match(&memory), expected)
    }

    #[test_case(
        "!",
        vec![false, false, false, false, false, false, false, false, false, false];
        "never"
    )]
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
        "2|5|9",
        vec![false, true, false, false, true, false, false, false, true, false];
        "set of indexes"
    )]
    #[test_case(
        "2-7",
        vec![false, true, true, true, true, true, true, false, false, false];
        "range"
    )]
    #[test_case(
        "(3-5)!",
        vec![true, true, false, false, false, true, true, true, true, true];
        "negated range"
    )]
    #[test_case(
        "6+2",
        vec![false, false, false, false, false, true, true, true, false, false];
        "extended"
    )]
    #[test_case(
        "(2+3)!",
        vec![true, false, false, false, false, true, true, true, true, true];
        "negated extended"
    )]
    #[test_case(
        "1~2",
        vec![true, false, true, false, true, false, true, false, true, false];
        "nth for odd"
    )]
    #[test_case(
        "(1~2)!",
        vec![false, true, false, true, false, true, false, true, false, true];
        "negated nth for odd"
    )]
    #[test_case(
        "(2|3)-(7|8)",
        vec![false, true, true, true, true, true, true, false, false, false];
        "range containing brackets"
    )]
    #[test_case(
        "1-1",
        vec![true, false, false, false, false, false, false, false, false, false];
        "range of indexes 1 to 1"
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
                    addr.is_match(&memory)
                })
                .collect::<Vec<bool>>(),
            expected
        )
    }
}
