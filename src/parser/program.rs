use super::{
    address, command,
    reader::{FileReader, Reader, StringReader},
    skip_whitespace,
};
use crate::{Action, Error, Result, command::Command, error, program::Program};
use std::{collections::HashMap, str::FromStr};

impl TryFrom<&std::path::PathBuf> for Program {
    type Error = Error;

    fn try_from(value: &std::path::PathBuf) -> Result<Self> {
        let reader = &mut FileReader::try_from(value)?;
        let (actions, finally) = parse(reader)?;
        Ok(Program::new(actions, finally))
    }
}

impl FromStr for Program {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self> {
        let reader = &mut StringReader::from(s);
        let (actions, finally) = parse(reader)?;
        Ok(Program::new(actions, finally))
    }
}

fn parse<R: Reader>(reader: &mut R) -> Result<(Vec<Action>, Vec<Command>)> {
    let mut actions = Vec::new();
    let mut finally = Vec::new();
    let mut labels = HashMap::new();
    while reader.peek()?.is_some() {
        parse_instruction(reader, &mut actions, &mut finally, &mut labels)?;
        skip_whitespace(reader);
    }
    for action in actions.iter_mut() {
        if let Action::Command(Command::Branch(label, pos)) = action {
            if let Some(idx) = labels.get(label) {
                *pos = *idx;
            } else {
                return error!("unknown label: {}", label);
            }
        }
    }
    Ok((actions, finally))
}

fn parse_instruction<R: Reader>(
    reader: &mut R,
    actions: &mut Vec<Action>,
    finally: &mut Vec<Command>,
    lables: &mut HashMap<String, usize>,
) -> Result<()> {
    // [address][commands]
    skip_whitespace(reader);
    let mut address = address::parse(reader)?;
    skip_whitespace(reader);
    let commands = command::parse(reader)?;

    if address.is_final() {
        for cmd in &commands {
            if matches!(cmd, Command::Branch(_, _) | Command::Label(_)) {
                return error!("branching is not supported the final block");
            }
            finally.push(cmd.clone());
        }
    }

    if address.is_regular() {
        let subst = commands.iter().find_map(|c| {
            if let Command::Substitute(regex, _, _) = c {
                Some(regex)
            } else {
                None
            }
        });
        address.replace_maybe(subst)?;
        address.simplify()?;
        actions.push(Action::Condition(address, commands.len()));

        for cmd in commands.into_iter() {
            if let Command::Label(label) = &cmd
                && lables.insert(label.to_owned(), actions.len()).is_some()
            {
                return error!("duplicated label: {}", label);
            }
            actions.push(Action::Command(cmd));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::Program;
    use crate::{
        Action,
        address::{self, Address::*},
        command::Command::*,
    };
    use std::str::FromStr;
    use test_case::test_case;

    impl From<Vec<Action>> for Program {
        fn from(value: Vec<Action>) -> Self {
            Program::new(value, Vec::new())
        }
    }

    #[test_case("", Program::from(Vec::new()); "empty")]
    #[test_case("p", Program::from(vec![
        Action::Condition(Always, 1),
        Action::Command(Println(None)),
    ]); "print all")]
    #[test_case(r"=a'\n'p", Program::from(vec![
        Action::Condition(Always, 3),
        Action::Command(LineNumber),
        Action::Command(Append("\n".to_string())),
        Action::Command(Println(None)),
    ]); "print with newlines")]
    #[test_case(r"   = a\n  p  ", Program::from(vec![
        Action::Condition(Always, 3),
        Action::Command(LineNumber),
        Action::Command(Append("\n".to_string())),
        Action::Command(Println(None)),
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
    #[test_case("!13-72", Program::from(vec![
        Action::Condition(Between(address::Between::new(Negate(Box::new(Location(13))), Location(72))), 0),
    ]); "range with negated")]
    #[test_case("/abc/", Program::from(vec![
        Action::Condition(Regex(crate::Regex::from_str("abc").unwrap()), 0)
    ]); "regex match")]
    #[test_case(r"/abc\//", Program::from(vec![
        Action::Condition(Regex(crate::Regex::from_str("abc/").unwrap()), 0)
    ]); "regex match with escape")]
    #[test_case(r"\\/\", Program::from(vec![
        Action::Condition(Regex(crate::Regex::from_str("/").unwrap()), 0)
    ]); "regex match with custom deliminator")]
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
    #[test_case("!(!1)", Program::from(vec![
        Action::Condition(Location(1), 0),
    ]); "double negation")]
    #[test_case(" !   666   ", Program::from(vec![
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
    #[test_case("5,6,!10", Program::from(vec![
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
    #[test_case(r"s/abc/def/", Program::from(vec![
        Action::Condition(Always, 1),
        Action::Command(Substitute(
                crate::Regex::from_str("abc").unwrap(),
                "def".to_string(),
                0,
            )),
    ]); "substitute with global count")]
    #[test_case(r"s\/\//\", Program::from(vec![
        Action::Condition(Always, 1),
        Action::Command(Substitute(
                crate::Regex::from_str("/").unwrap(),
                "//".to_string(),
                0,
            )),
    ]); "substitute with custom delimiter")]
    #[test_case(r"/abc/s/def/ghi/", Program::from(vec![
        Action::Condition(Regex(crate::Regex::from_str("abc").unwrap()), 1),
        Action::Command(Substitute(
                crate::Regex::from_str("def").unwrap(),
                "ghi".to_string(),
                0,
            )),
    ]); "condense match and substitute")]
    #[test_case(r"s/(abc)/__\123__/", Program::from(vec![
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
    #[test_case(r"?s/abc/def/5", Program::from(vec![
        Action::Condition(Regex(crate::Regex::from_str("abc").unwrap()), 1),
        Action::Command(Substitute(
                crate::Regex::from_str("abc").unwrap(),
                "def".to_string(),
                5,
            )),
    ]); "maybe")]
    #[test_case(r"1-?s/abc/def/5", Program::from(vec![
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
    #[test_case(r"1,?s/abc/def/5", Program::from(vec![
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
    #[test_case("1,$ p'ok'; !q", Program::new(vec![
        Action::Condition(Location(1), 1),
        Action::Command(Println(Some("ok".to_string()))),
        Action::Condition(Never, 1),
        Action::Command(Quit(0)),
    ], vec![
        Println(Some("ok".to_string())),
    ]); "remove final blocks")]
    #[test_case("1,((!//)+1),2 p'ok'", Program::from(vec![
        Action::Condition(Set(vec![Location(1), Location(2)]), 1),
        Action::Command(Println(Some("ok".to_string()))),
    ]); "remove never blocks")]
    #[test_case("/a/,/b/,1,/c|d/ q", Program::from(vec![
        Action::Condition(Set(vec![
            Regex(crate::Regex::from_str("a|b|c|d").unwrap()),
            Location(1),
        ]), 1),
        Action::Command(Quit(0)),
    ]); "simplify set of regular expressions")]
    fn parse(input: &str, expected: Program) {
        let result = Program::from_str(input).unwrap();
        assert_eq!(result, expected)
    }

    #[test_case("bx"; "undeclared label")]
    #[test_case(":x p'hi!' :x p'hola!' b x"; "duplicated label")]
    #[test_case("bx ; $ :x p'the end'"; "label in final block")]
    fn parsing_invalid_branching(input: &str) {
        let result = Program::from_str(input);
        assert!(result.is_err())
    }
}
