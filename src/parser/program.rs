use super::{
    address, command,
    reader::{FileReader, Reader, StringReader},
    utils::{self, skip_whitespace},
};
use crate::{
    address::Address,
    command::Command,
    program::{Action, Program},
};
use anyhow::{bail, Result};
use std::str::FromStr;

impl TryFrom<&std::path::PathBuf> for Program {
    type Error = anyhow::Error;

    fn try_from(value: &std::path::PathBuf) -> Result<Self, Self::Error> {
        let reader = &mut FileReader::try_from(value)?;
        let (actions, finally) = parse(reader)?;
        Ok(Program(actions, finally))
    }
}

impl FromStr for Program {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let reader = &mut StringReader::from(s);
        let (actions, finally) = parse(reader)?;
        Ok(Program(actions, finally))
    }
}

fn parse<R: Reader>(reader: &mut R) -> Result<(Vec<Action>, Vec<Command>)> {
    let mut actions = Vec::new();
    let mut finally = Vec::new();
    while reader.peek()?.is_some() {
        parse_instruction(reader, &mut actions, &mut finally)?;
        skip_whitespace(reader);
    }
    Ok((actions, finally))
}

fn parse_instruction<R: Reader>(
    reader: &mut R,
    actions: &mut Vec<Action>,
    finally: &mut Vec<Command>,
) -> Result<()> {
    // [address][commands]
    utils::skip_whitespace(reader);
    let mut address = address::parse(reader)?;
    utils::skip_whitespace(reader);
    let commands = command::parse(reader)?;

    if address == Address::Final {
        for cmd in commands.into_iter() {
            finally.push(cmd);
        }
    } else {
        address.replace_maybe(commands.first())?;
        actions.push(Action::Condition(address, commands.len()));
        for cmd in commands.into_iter() {
            actions.push(Action::Command(cmd));
        }
    }
    Ok(())
}

impl Address {
    fn replace_maybe(&mut self, subst: Option<&Command>) -> Result<()> {
        match self {
            Address::Maybe => {
                let Some(Command::Substitute(regex, _, _)) = subst else {
                    bail!("? must be followed by a substitution")
                };
                *self = Address::Regex(regex.clone());
            }
            Address::Between(between) => {
                between.lhs.replace_maybe(subst)?;
                between.rhs.replace_maybe(subst)?;
            }
            Address::Set(addrs) => addrs.iter_mut().try_for_each(|a| a.replace_maybe(subst))?,
            _ => (),
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::Program;
    use crate::{
        address::{self, Address::*},
        command::Command::*,
        program::Action,
    };
    use std::str::FromStr;
    use test_case::test_case;

    #[test_case("", Program::from(Vec::new()); "empty")]
    #[test_case("p", Program::from(vec![
        Action::Condition(Always, 1),
        Action::Command(Println),
    ]); "print all")]
    #[test_case(r"='\n'p", Program::from(vec![
        Action::Condition(Always, 3),
        Action::Command(LineNumber),
        Action::Command(Insert("\n".to_string())),
        Action::Command(Println),
    ]); "print with newlines")]
    #[test_case(r"   = '\n'  p  ", Program::from(vec![
        Action::Condition(Always, 3),
        Action::Command(LineNumber),
        Action::Command(Insert("\n".to_string())),
        Action::Command(Println),
    ]); "commands with spaces")]
    #[test_case("-", Program::from(vec![
        Action::Condition(Between(address::Between::new(Location(1), Final)), 0),
    ]); "infinite range")]
    #[test_case("-5", Program::from(vec![
        Action::Condition(Between(address::Between::new(Location(1), Location(5))), 0),
    ]); "right bound range")]
    #[test_case("3-", Program::from(vec![
        Action::Condition(Between(address::Between::new(Location(3), Final)), 0),
    ]); "left bound range")]
    #[test_case("13-72", Program::from(vec![
        Action::Condition(Between(address::Between::new(Location(13), Location(72))), 0),
    ]); "range")]
    #[test_case(" 13  -   72 ", Program::from(vec![
        Action::Condition(Between(address::Between::new(Location(13), Location(72))), 0),
    ]); "range with spaces")]
    #[test_case("13-72!", Program::from(vec![
        Action::Condition(Negate(Box::new(Between(address::Between::new(Location(13), Location(72))))), 0),
    ]); "range negated")]
    #[test_case("/abc/", Program::from(vec![
        Action::Condition(Regex(crate::Regex::from_str("abc").unwrap()), 0)
    ]); "regex match")]
    #[test_case(r"/abc\//", Program::from(vec![
        Action::Condition(Regex(crate::Regex::from_str("abc/").unwrap()), 0)
    ]); "regex match with escape")]
    #[test_case("^abc$", Program::from(vec![
        Action::Condition(Regex(crate::Regex::from_str("^abc$").unwrap()), 0)
    ]); "whole line regex match")]
    #[test_case(r"^\$abc$", Program::from(vec![
        Action::Condition(Regex(crate::Regex::from_str(r"^\$abc$").unwrap()), 0)
    ]); "whole line regex match with escape")]
    #[test_case(r"^\$$", Program::from(vec![
        Action::Condition(Regex(crate::Regex::from_str(r"^\$$").unwrap()), 0)
    ]); "whole line only dollar")]
    #[test_case("/abc/-/def/", Program::from(vec![
        Action::Condition(Between(address::Between::new(
            Regex(crate::Regex::from_str("abc").unwrap()),
            Regex(crate::Regex::from_str("def").unwrap()),
        )), 0),
    ]); "regex range")]
    #[test_case("(1!)!", Program::from(vec![
        Action::Condition(Location(1), 0),
    ]); "double negation")]
    #[test_case(" 666    ! ", Program::from(vec![
        Action::Condition(Negate(Box::new(Location(666))), 0)
    ]); "negation with space")]
    #[test_case("5,6,10", Program::from(vec![
        Action::Condition(Set(vec![Location(5), Location(6), Location(10)]), 0),
    ]); "set")]
    #[test_case("((5),((6),10))", Program::from(vec![
        Action::Condition(Set(vec![Location(5), Location(6), Location(10)]), 0),
    ]); "set with brackets")]
    #[test_case("  5, 6  ,10   ", Program::from(vec![
        Action::Condition(Set(vec![Location(5), Location(6), Location(10)]), 0),
    ]); "set with spaces")]
    #[test_case("5,6,10!", Program::from(vec![
        Action::Condition(Set(vec![Location(5), Location(6), Negate(Box::new(Location(10)))]), 0),
    ]); "set negated")]
    #[test_case("(((42)))", Program::from(vec![
        Action::Condition(Location(42), 0)
    ]); "brackets")]
    #[test_case(r"/abc\/123/", Program::from(vec![
        Action::Condition(Regex(crate::Regex::from_str("abc/123").unwrap()), 0),
    ]); "regex")]
    #[test_case(r"s/abc/def/", Program::from(vec![
        Action::Condition(Always, 1),
        Action::Command(Substitute(
                crate::Regex::from_str("abc").unwrap(),
                "def".to_string(),
                0,
            )),
    ]); "substitute")]
    #[test_case(r"s/abc/def/5", Program::from(vec![
        Action::Condition(Always, 1),
        Action::Command(Substitute(
                crate::Regex::from_str("abc").unwrap(),
                "def".to_string(),
                5,
            )),
    ]); "substitute with count")]
    #[test_case(r"s/abc/def/g", Program::from(vec![
        Action::Condition(Always, 1),
        Action::Command(Substitute(
                crate::Regex::from_str("abc").unwrap(),
                "def".to_string(),
                0,
            )),
    ]); "substitute with global count")]
    #[test_case(r"/abc/s/def/ghi/g", Program::from(vec![
        Action::Condition(Regex(crate::Regex::from_str("abc").unwrap()), 1),
        Action::Command(Substitute(
                crate::Regex::from_str("def").unwrap(),
                "ghi".to_string(),
                0,
            )),
    ]); "condense match and substitute")]
    #[test_case(r"s/(abc)/__$123__/", Program::from(vec![
        Action::Condition(Always, 1),
        Action::Command(Substitute(
                crate::Regex::from_str("(abc)").unwrap(),
                "__${123}__".to_string(),
                0,
            )),
    ]); "substitute with numbered group")]
    #[test_case(r"1d;3d;7d", Program::from(vec![
        Action::Condition(Location(1), 1),
        Action::Command(Delete),
        Action::Condition(Location(3), 1),
        Action::Command(Delete),
        Action::Condition(Location(7), 1),
        Action::Command(Delete),
    ]); "multiple instructions")]
    #[test_case(r"? s/abc/def/5", Program::from(vec![
        Action::Condition(Regex(crate::Regex::from_str("abc").unwrap()), 1),
        Action::Command(Substitute(
                crate::Regex::from_str("abc").unwrap(),
                "def".to_string(),
                5,
            )),
    ]); "maybe")]
    #[test_case(r"1-? s/abc/def/5", Program::from(vec![
        Action::Condition(
            Between(address::Between::new(
                Location(1),
                Regex(crate::Regex::from_str("abc").unwrap())
            )),
            1,
        ),
        Action::Command(Substitute(
                crate::Regex::from_str("abc").unwrap(),
                "def".to_string(),
                5,
            )),
    ]); "maybe in range")]
    #[test_case(r"1,? s/abc/def/5", Program::from(vec![
        Action::Condition(
            Set(vec![
                Location(1),
                Regex(crate::Regex::from_str("abc").unwrap())
            ]),
            1,
        ),
        Action::Command(Substitute(
                crate::Regex::from_str("abc").unwrap(),
                "def".to_string(),
                5,
            )),
    ]); "maybe in set")]
    fn parse(input: &str, expected: Program) {
        let result = Program::from_str(input).unwrap();
        assert_eq!(result, expected)
    }
}
