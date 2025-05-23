use super::{
    address, command,
    reader::{FileReader, Reader, StringReader},
    utils,
};
use crate::{
    function::{Function, Instruction},
    Error,
};
use std::str::FromStr;

impl TryFrom<&std::path::PathBuf> for Function {
    type Error = Error;

    fn try_from(value: &std::path::PathBuf) -> Result<Self, Self::Error> {
        let reader = &mut FileReader::try_from(value)?;
        parse(reader)
    }
}

impl FromStr for Function {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let reader = &mut StringReader::from(s);
        parse(reader)
    }
}

fn parse<R: Reader>(reader: &mut R) -> Result<Function, Error> {
    let mut instructions = Vec::new();
    loop {
        instructions.push(parse_instruction(reader)?);
        if reader.peek()?.is_none() {
            break;
        }
    }
    Ok(Function(instructions))
}

fn parse_instruction<R: Reader>(reader: &mut R) -> Result<Instruction, Error> {
    // [address][commands]
    utils::skip_whitespace(reader);
    let address = address::parse(reader)?;
    utils::skip_whitespace(reader);
    let commands = command::parse(reader)?;
    Ok(Instruction { address, commands })
}

#[cfg(test)]
mod tests {
    use crate::{
        address::{Address::*, Boundary},
        command::Command::*,
        function::{Function, Instruction},
    };
    use std::str::FromStr;
    use test_case::test_case;

    #[test_case("", Function(vec![Instruction{
        address: Always,
        commands: Vec::new(),
    }]); "empty")]
    #[test_case("*", Function(vec![Instruction{
        address: Always,
        commands: Vec::new()
    }]); "any")]
    #[test_case("p", Function(vec![Instruction{
        address: Always,
        commands: vec![Println]
    }]); "print all")]
    #[test_case(r"=\np", Function(vec![Instruction{
        address: Always,
        commands: vec![LineNumber, Insert("\n".to_string()), Println]
    }]); "print with newlines")]
    #[test_case(r"   = \n  p  ", Function(vec![Instruction{
        address: Always,
        commands: vec![LineNumber, Insert("\n".to_string()), Println]
    }]); "commands with spaces")]
    #[test_case("-", Function(vec![Instruction{
        address: Between(Boundary::Location(1), Boundary::from(Never)),
        commands: Vec::new()
    }]); "infinite range")]
    #[test_case("-5", Function(vec![Instruction{
        address: Between(Boundary::Location(1), Boundary::Location(5)),
        commands: Vec::new(),
    }]); "right bound range")]
    #[test_case("3-", Function(vec![Instruction{
        address: Between(Boundary::Location(3), Boundary::from(Never)),
        commands: Vec::new(),
    }]); "left bound range")]
    #[test_case("13-72", Function(vec![Instruction{
        address: Between(Boundary::Location(13), Boundary::Location(72)),
        commands: Vec::new(),
    }]); "range")]
    #[test_case(" 13  -   72 ", Function(vec![Instruction{
        address: Between(Boundary::Location(13), Boundary::Location(72)),
        commands: Vec::new(),
    }]); "range with spaces")]
    #[test_case("13-72!", Function(vec![Instruction{
        address: Negate(Box::new(Between(Boundary::Location(13), Boundary::Location(72)))),
        commands: Vec::new(),
    }]); "range negated")]
    #[test_case("/abc/", Function(vec![Instruction{
        address: Regex(crate::Regex::from_str("abc").unwrap()),
        commands: Vec::new(),
    }]); "regex match")]
    #[test_case(r"/abc\//", Function(vec![Instruction{
        address: Regex(crate::Regex::from_str("abc/").unwrap()),
        commands: Vec::new(),
    }]); "regex match with escape")]
    #[test_case("^abc$", Function(vec![Instruction{
        address: Regex(crate::Regex::from_str("^abc$").unwrap()),
        commands: Vec::new(),
    }]); "whole line regex match")]
    #[test_case(r"^\$abc$", Function(vec![Instruction{
        address: Regex(crate::Regex::from_str(r"^\$abc$").unwrap()),
        commands: Vec::new(),
    }]); "whole line regex match with escape")]
    #[test_case(r"^\$$", Function(vec![Instruction{
        address: Regex(crate::Regex::from_str(r"^\$$").unwrap()),
        commands: Vec::new(),
    }]); "whole line only dollar")]
    #[test_case("/abc/-/def/", Function(vec![Instruction{
        address: Between(
            Boundary::from(Regex(crate::Regex::from_str("abc").unwrap())),
            Boundary::from(Regex(crate::Regex::from_str("def").unwrap())),
        ),
        commands: Vec::new(),
    }]); "regex range")]
    #[test_case("(1!)!", Function(vec![Instruction{
        address: Location(1),
        commands: Vec::new(),
    }]); "double negation")]
    #[test_case(" 666    ! ", Function(vec![Instruction{
        address: Negate(Box::new(Location(666))),
        commands: Vec::new(),
    }]); "negation with space")]
    #[test_case("5,6,10", Function(vec![Instruction{
        address: Set(vec![Location(5), Location(6), Location(10)]),
        commands: Vec::new(),
    }]); "set")]
    #[test_case("((5),((6),10))", Function(vec![Instruction{
        address: Set(vec![Location(5), Location(6), Location(10)]),
        commands: Vec::new(),
    }]); "set with brackets")]
    #[test_case("  5, 6  ,10   ", Function(vec![Instruction{
        address: Set(vec![Location(5), Location(6), Location(10)]),
        commands: Vec::new(),
    }]); "set with spaces")]
    #[test_case("5,6,10!", Function(vec![Instruction{
        address: Set(vec![Location(5), Location(6), Negate(Box::new(Location(10)))]),
        commands: Vec::new(),
    }]); "set negated")]
    #[test_case("(((42)))", Function(vec![Instruction{
        address: Location(42),
        commands: Vec::new(),
    }]); "brackets")]
    #[test_case(r"/abc\/123/", Function(vec![Instruction{
        address: Regex(crate::Regex::from_str("abc/123").unwrap()),
        commands: Vec::new(),
    }]); "regex")]
    #[test_case(r"s/abc/def/", Function(vec![Instruction{
        address: Always,
        commands: vec![Substitute(
                crate::Regex::from_str("abc").unwrap(),
                "def".to_string(),
                0,
            )],
    }]); "substitute")]
    #[test_case(r"s/abc/def/5", Function(vec![Instruction{
        address: Always,
        commands: vec![Substitute(
                crate::Regex::from_str("abc").unwrap(),
                "def".to_string(),
                5,
            )],
    }]); "substitute with count")]
    #[test_case(r"s/abc/def/g", Function(vec![Instruction{
        address: Always,
        commands: vec![Substitute(
                crate::Regex::from_str("abc").unwrap(),
                "def".to_string(),
                0,
            )],
    }]); "substitute with global count")]
    #[test_case(r"/abc/s/def/ghi/g", Function(vec![Instruction{
        address: Regex(crate::Regex::from_str("abc").unwrap()),
        commands: vec![Substitute(
                crate::Regex::from_str("def").unwrap(),
                "ghi".to_string(),
                0,
            )],
    }]); "condense match and substitute")]
    #[test_case(r"s/(abc)/__$123__/", Function(vec![Instruction{
        address: Always,
        commands: vec![Substitute(
                crate::Regex::from_str("(abc)").unwrap(),
                "__${123}__".to_string(),
                0,
            )],
    }]); "substitute with numbered group")]
    #[test_case(r"1d;3d;7d", Function(vec![
        Instruction{
            address: Location(1),
            commands: vec![Delete],
        },
        Instruction{
            address: Location(3),
            commands: vec![Delete],
        },
        Instruction{
            address: Location(7),
            commands: vec![Delete],
        },
    ]); "multiple instructions")]
    fn parse(input: &str, expected: Function) {
        let result = Function::from_str(input).unwrap();
        assert_eq!(result, expected)
    }
}
