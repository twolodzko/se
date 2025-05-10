use crate::Line;

#[derive(Debug)]
pub(crate) enum Address {
    // always matches
    Always,
    // never matches
    Never,
    // specific index
    Location(usize),
    // /regex/ matching the line
    Regex(regex::Regex),
    // addr! negates the addr match
    Negate(Box<Address>),
    // // addr1 - addr2 (at least one is an index)
    // Range(Address, Address),
    // // addr1 - addr2
    Between(Box<Address>, Box<Address>, bool),
    // addr1, addr2, ...
    Set(Vec<Address>),
}

impl Address {
    pub(crate) fn matches(&mut self, line: &Line) -> bool {
        use Address::*;
        match self {
            Always => true,
            Never => false,
            Location(idx) => *idx == line.0,
            Regex(ref regex) => regex.is_match(&line.1),
            Negate(addr) => !addr.matches(line),
            Between(lhs, rhs, inside) => {
                if *inside {
                    if rhs.matches(line) {
                        *inside = false;
                    }
                    true
                } else {
                    if lhs.matches(line) {
                        if !rhs.matches(line) {
                            *inside = true;
                        }
                        return true;
                    }
                    false
                }
            }
            Set(addrs) => {
                let mut ok = false;
                for addr in addrs.iter_mut() {
                    if ok {
                        // Between's always need to be evaluated
                        // so we don't miss the bounds
                        if let Negate(inner) = addr {
                            if !matches!(inner.as_ref(), Between(_, _, _)) {
                                continue;
                            }
                        }
                        if !matches!(addr, Between(_, _, _)) {
                            continue;
                        }
                    }
                    if addr.matches(line) {
                        ok = true;
                    }
                }
                ok
            }
        }
    }
}

impl std::ops::Not for Address {
    type Output = Address;

    fn not(self) -> Self::Output {
        use Address::*;
        match self {
            Negate(inner) => *inner,
            Always => Never,
            Never => Always,
            _ => Negate(Box::new(self)),
        }
    }
}

impl std::fmt::Display for Address {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use Address::*;
        match self {
            Always => write!(f, "*"),
            Never => write!(f, "$"),
            Location(idx) => write!(f, "{}", idx),
            Regex(regex) => write!(f, "/{}/", regex),
            Negate(addr) => write!(f, "{}!", addr),
            Between(lhs, rhs, _) => write!(f, "{}-{}", lhs, rhs),
            Set(addrs) => write!(
                f,
                "{}",
                addrs
                    .iter()
                    .map(|a| a.to_string())
                    .collect::<Vec<String>>()
                    .join(", ")
            ),
        }
    }
}

impl PartialEq for Address {
    fn eq(&self, other: &Self) -> bool {
        use Address::*;
        match (self, other) {
            (Always, Always) => true,
            (Never, Never) => true,
            (Location(lhs), Location(rhs)) => lhs == rhs,
            (Regex(lhs), Regex(rhs)) => lhs.as_str() == rhs.as_str(),
            (Negate(lhs), Negate(rhs)) => lhs == rhs,
            (Between(lhs_lo, lhs_hi, _), Between(rhs_lo, rhs_hi, _)) => {
                lhs_lo == rhs_lo && lhs_hi == rhs_hi
            }
            (Set(lhs), Set(rhs)) => std::iter::zip(lhs, rhs).all(|(a, b)| a == b),
            _ => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        address::Address::{self, *},
        Line,
    };
    use test_case::test_case;

    #[test_case(Always, Line(1, "".to_string()), true; "any matches line 1")]
    #[test_case(Always, Line(279, "".to_string()), true; "any matches line 279")]
    #[test_case(Negate(Box::new(Always)), Line(1, "".to_string()), false; "negated any does not match line 1")]
    #[test_case(Negate(Box::new(Always)), Line(279, "".to_string()), false; "negated any does not match line 279")]
    #[test_case(Location(1), Line(1, "".to_string()), true; "index 1 matches line 1")]
    #[test_case(Location(1), Line(279, "".to_string()), false; "index 1 does not match line 279")]
    #[test_case(
        Regex(regex::Regex::new("abc").unwrap()),
        Line(1, "abc".to_string()),
        true;
        "regex abc matches line abc"
    )]
    #[test_case(
        Regex(regex::Regex::new("abc").unwrap()),
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
        let mut addr = addr;
        assert_eq!(addr.matches(&line), expected)
    }

    #[test_case(
        Always,
        vec![true, true, true, true, true, true, true, true, true, true];
        "any"
    )]
    #[test_case(
        Negate(Box::new(Always)),
        vec![false, false, false, false, false, false, false, false, false, false];
        "any negated"
    )]
    #[test_case(
        Location(7),
        vec![false, false, false, false, false, false, true, false, false, false];
        "index 7"
    )]
    #[test_case(
        Location(89),
        vec![false, false, false, false, false, false, false, false, false, false];
        "index 89"
    )]
    #[test_case(
        Set(vec![Location(2), Location(5), Location(9)]),
        vec![false, true, false, false, true, false, false, false, true, false];
        "set of indexes"
    )]
    #[test_case(
        Between(
            Box::new(Location(2)),
            Box::new(Location(7)),
            false,
        ),
        vec![false, true, true, true, true, true, true, false, false, false];
        "range of indexes 2-7"
    )]
    #[test_case(
        Between(
            Box::new(Location(1)),
            Box::new(Location(1)),
            false,
        ),
        vec![true, false, false, false, false, false, false, false, false, false];
        "range of indexes 1-1"
    )]
    #[test_case(
        Regex(regex::Regex::new("aa").unwrap()),
        vec![false, false, true, false, true, true, false, false, false, false];
        "regex aa"
    )]
    #[test_case(
        Between(
            Box::new(Regex(regex::Regex::new("start").unwrap())),
            Box::new(Regex(regex::Regex::new("end").unwrap())),
            false,
        ),
        vec![false, true, true, true, false, true, true, false, false, false];
        "regex range matches twice"
    )]
    #[test_case(
        Between(
            Box::new(Location(5)),
            Box::new(Regex(regex::Regex::new("123").unwrap())),
            false,
        ),
        vec![false, false, false, false, true, true, true, true, true, false];
        "mixed range"
    )]
    #[test_case(
        Between(
            Box::new(Location(6)),
            Box::new(Never),
            false,
        ),
        vec![false, false, false, false, false, true, true, true, true, true];
        "half-open range"
    )]
    fn multiline_example(addr: Address, expected: Vec<bool>) {
        let example = r"
            start
            aaa
            end
            zzz aa bb c
            start aabcd
            def end

            123
        ";
        let mut addr = addr;
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
