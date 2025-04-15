use super::{
    address, command,
    reader::{FileReader, Reader, StringReader},
    utils::{self, skip_whitespace},
};
use crate::{
    function::{Action, Function},
    Error,
};
use std::{collections::HashMap, str::FromStr};

impl TryFrom<&std::path::PathBuf> for Function {
    type Error = Error;

    fn try_from(value: &std::path::PathBuf) -> Result<Self, Self::Error> {
        let reader = &mut FileReader::try_from(value)?;
        let (instructions, functions) = parse(reader)?;
        Ok(Function(instructions.into(), functions))
    }
}

impl FromStr for Function {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let reader = &mut StringReader::from(s);
        let (instructions, functions) = parse(reader)?;
        Ok(Function(instructions.into(), functions))
    }
}

fn parse<R: Reader>(reader: &mut R) -> Result<(Vec<Action>, HashMap<String, Vec<Action>>), Error> {
    let mut instructions = Vec::new();
    let mut functions = HashMap::new();
    while let Some(c) = reader.peek()? {
        match c {
            '@' => {
                reader.next()?;
                skip_whitespace(reader);
                let (name, body) = parse_function(reader)?;
                functions.insert(name, body);
            }
            _ => instructions.append(&mut parse_instruction(reader)?),
        }

        skip_whitespace(reader);
    }
    Ok((instructions, functions))
}

fn parse_instruction<R: Reader>(reader: &mut R) -> Result<Vec<Action>, Error> {
    // [address][commands]
    utils::skip_whitespace(reader);
    let address = address::parse(reader)?;
    utils::skip_whitespace(reader);
    let commands = command::parse(reader)?;

    let mut instruction = Vec::new();
    instruction.push(Action::Condition(address, commands.len()));
    for cmd in commands.into_iter() {
        instruction.push(Action::Command(cmd));
    }
    Ok(instruction)
}

fn parse_function<R: Reader>(reader: &mut R) -> Result<(String, Vec<Action>), Error> {
    let mut name = String::new();
    while let Some(c) = reader.peek()? {
        if c.is_alphanumeric() {
            reader.next()?;
            name.push(c);
        } else {
            break;
        }
    }
    if name.is_empty() {
        return Err(Error::Custom("function name cannot be empty".to_string()));
    }

    skip_whitespace(reader);
    let Some('{') = reader.next()? else {
        return Err(Error::Missing('{'));
    };

    let mut instructions = Vec::new();
    loop {
        instructions.append(&mut parse_instruction(reader)?);

        skip_whitespace(reader);
        match reader.peek()? {
            None => return Err(Error::Missing('}')),
            Some('}') => {
                reader.next()?;
                break;
            }
            _ => (),
        }
    }

    Ok((name, instructions))
}

#[cfg(test)]
mod tests {
    use super::Function;
    use crate::{
        address::{self, Address::*},
        command::Command::*,
        function::Action,
    };
    use std::str::FromStr;
    use test_case::test_case;

    #[test_case("", Function::from(Vec::new()); "empty")]
    #[test_case("*", Function::from(vec![
        Action::Condition(Always, 0),
    ]); "any")]
    #[test_case("p", Function::from(vec![
        Action::Condition(Always, 1),
        Action::Command(Println),
    ]); "print all")]
    #[test_case(r"=\np", Function::from(vec![
        Action::Condition(Always, 3),
        Action::Command(LineNumber),
        Action::Command(Insert("\n".to_string())),
        Action::Command(Println),
    ]); "print with newlines")]
    #[test_case(r"   = \n  p  ", Function::from(vec![
        Action::Condition(Always, 3),
        Action::Command(LineNumber),
        Action::Command(Insert("\n".to_string())),
        Action::Command(Println),
    ]); "commands with spaces")]
    #[test_case("-", Function::from(vec![
        Action::Condition(Between(address::Between::new(Location(1), Never)), 0),
    ]); "infinite range")]
    #[test_case("-5", Function::from(vec![
        Action::Condition(Between(address::Between::new(Location(1), Location(5))), 0),
    ]); "right bound range")]
    #[test_case("3-", Function::from(vec![
        Action::Condition(Between(address::Between::new(Location(3), Never)), 0),
    ]); "left bound range")]
    #[test_case("13-72", Function::from(vec![
        Action::Condition(Between(address::Between::new(Location(13), Location(72))), 0),
    ]); "range")]
    #[test_case(" 13  -   72 ", Function::from(vec![
        Action::Condition(Between(address::Between::new(Location(13), Location(72))), 0),
    ]); "range with spaces")]
    #[test_case("13-72!", Function::from(vec![
        Action::Condition(Negate(Box::new(Between(address::Between::new(Location(13), Location(72))))), 0),
    ]); "range negated")]
    #[test_case("/abc/", Function::from(vec![
        Action::Condition(Regex(crate::Regex::from_str("abc").unwrap()), 0)
    ]); "regex match")]
    #[test_case(r"/abc\//", Function::from(vec![
        Action::Condition(Regex(crate::Regex::from_str("abc/").unwrap()), 0)
    ]); "regex match with escape")]
    #[test_case("^abc$", Function::from(vec![
        Action::Condition(Regex(crate::Regex::from_str("^abc$").unwrap()), 0)
    ]); "whole line regex match")]
    #[test_case(r"^\$abc$", Function::from(vec![
        Action::Condition(Regex(crate::Regex::from_str(r"^\$abc$").unwrap()), 0)
    ]); "whole line regex match with escape")]
    #[test_case(r"^\$$", Function::from(vec![
        Action::Condition(Regex(crate::Regex::from_str(r"^\$$").unwrap()), 0)
    ]); "whole line only dollar")]
    #[test_case("/abc/-/def/", Function::from(vec![
        Action::Condition(Between(address::Between::new(
            Regex(crate::Regex::from_str("abc").unwrap()),
            Regex(crate::Regex::from_str("def").unwrap()),
        )), 0),
    ]); "regex range")]
    #[test_case("(1!)!", Function::from(vec![
        Action::Condition(Location(1), 0),
    ]); "double negation")]
    #[test_case(" 666    ! ", Function::from(vec![
        Action::Condition(Negate(Box::new(Location(666))), 0)
    ]); "negation with space")]
    #[test_case("5,6,10", Function::from(vec![
        Action::Condition(Set(vec![Location(5), Location(6), Location(10)]), 0),
    ]); "set")]
    #[test_case("((5),((6),10))", Function::from(vec![
        Action::Condition(Set(vec![Location(5), Location(6), Location(10)]), 0),
    ]); "set with brackets")]
    #[test_case("  5, 6  ,10   ", Function::from(vec![
        Action::Condition(Set(vec![Location(5), Location(6), Location(10)]), 0),
    ]); "set with spaces")]
    #[test_case("5,6,10!", Function::from(vec![
        Action::Condition(Set(vec![Location(5), Location(6), Negate(Box::new(Location(10)))]), 0),
    ]); "set negated")]
    #[test_case("(((42)))", Function::from(vec![
        Action::Condition(Location(42), 0)
    ]); "brackets")]
    #[test_case(r"/abc\/123/", Function::from(vec![
        Action::Condition(Regex(crate::Regex::from_str("abc/123").unwrap()), 0),
    ]); "regex")]
    #[test_case(r"s/abc/def/", Function::from(vec![
        Action::Condition(Always, 1),
        Action::Command(Substitute(
                crate::Regex::from_str("abc").unwrap(),
                "def".to_string(),
                0,
            )),
    ]); "substitute")]
    #[test_case(r"s/abc/def/5", Function::from(vec![
        Action::Condition(Always, 1),
        Action::Command(Substitute(
                crate::Regex::from_str("abc").unwrap(),
                "def".to_string(),
                5,
            )),
    ]); "substitute with count")]
    #[test_case(r"s/abc/def/g", Function::from(vec![
        Action::Condition(Always, 1),
        Action::Command(Substitute(
                crate::Regex::from_str("abc").unwrap(),
                "def".to_string(),
                0,
            )),
    ]); "substitute with global count")]
    #[test_case(r"/abc/s/def/ghi/g", Function::from(vec![
        Action::Condition(Regex(crate::Regex::from_str("abc").unwrap()), 1),
        Action::Command(Substitute(
                crate::Regex::from_str("def").unwrap(),
                "ghi".to_string(),
                0,
            )),
    ]); "condense match and substitute")]
    #[test_case(r"s/(abc)/__$123__/", Function::from(vec![
        Action::Condition(Always, 1),
        Action::Command(Substitute(
                crate::Regex::from_str("(abc)").unwrap(),
                "__${123}__".to_string(),
                0,
            )),
    ]); "substitute with numbered group")]
    #[test_case(r"1d;3d;7d", Function::from(vec![
        Action::Condition(Location(1), 1),
        Action::Command(Delete),
        Action::Condition(Location(3), 1),
        Action::Command(Delete),
        Action::Condition(Location(7), 1),
        Action::Command(Delete),
    ]); "multiple instructions")]
    fn parse(input: &str, expected: Function) {
        let result = Function::from_str(input).unwrap();
        assert_eq!(result, expected)
    }
}
