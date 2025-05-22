use std::collections::HashMap;

use super::{
    address, command,
    reader::{FileReader, Reader, StringReader},
    utils::{read_label, skip_whitespace},
};
use crate::{editor::Instruction, Editor, Error};

impl TryFrom<std::path::PathBuf> for Editor {
    type Error = Error;

    fn try_from(value: std::path::PathBuf) -> Result<Self, Self::Error> {
        let reader = &mut FileReader::try_from(value)?;
        parse(reader)
    }
}

impl TryFrom<String> for Editor {
    type Error = Error;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        let reader = &mut StringReader::from(value);
        parse(reader)
    }
}

fn parse<R: Reader>(reader: &mut R) -> Result<Editor, Error> {
    let mut instructions = Vec::new();
    let mut labels = HashMap::new();
    let mut i = 0;
    loop {
        let (label, instruction) = parse_instruction(reader)?;
        instructions.push(instruction);
        if let Some(label) = label {
            labels.insert(label, i);
        }
        if reader.peek()?.is_none() {
            break;
        }
        i += 1;
    }
    Ok(Editor::new(instructions, labels))
}

fn parse_instruction<R: Reader>(reader: &mut R) -> Result<(Option<String>, Instruction), Error> {
    // [:label][address][commands]
    skip_whitespace(reader);
    let label = parse_label(reader)?;

    skip_whitespace(reader);
    let address = address::parse(reader)?;

    skip_whitespace(reader);
    let commands = command::parse(reader)?;

    Ok((label, Instruction { address, commands }))
}

fn parse_label<R: Reader>(reader: &mut R) -> Result<Option<String>, Error> {
    if let Some(':') = reader.peek()? {
        reader.next()?;
        skip_whitespace(reader);
        Ok(Some(read_label(reader)?))
    } else {
        Ok(None)
    }
}

#[cfg(test)]
mod tests {
    use crate::{address::Address::*, command::Command::*, editor::Instruction, Editor};
    use std::{collections::HashMap, str::FromStr};
    use test_case::test_case;

    #[test_case("", Editor::new(vec![Instruction{
        address: Always,
        commands: Vec::new(),
    }], HashMap::new()); "empty")]
    #[test_case("*", Editor::new(vec![Instruction{
        address: Always,
        commands: Vec::new()
    }], HashMap::new()); "any")]
    #[test_case("p", Editor::new(vec![Instruction{
        address: Always,
        commands: vec![Println]
    }], HashMap::new()); "print all")]
    #[test_case(r"=\np", Editor::new(vec![Instruction{
        address: Always,
        commands: vec![LineNumber, Insert("\n".to_string()), Println]
    }], HashMap::new()); "print with newlines")]
    #[test_case(r"   = \n  p  ", Editor::new(vec![Instruction{
        address: Always,
        commands: vec![LineNumber, Insert("\n".to_string()), Println]
    }], HashMap::new()); "commands with spaces")]
    #[test_case("-", Editor::new(vec![Instruction{
        address: Between(Box::new(Location(1)), Box::new(Never), false),
        commands: Vec::new()
    }], HashMap::new()); "infinite range")]
    #[test_case("-5", Editor::new(vec![Instruction{
        address: Between(Box::new(Location(1)), Box::new(Location(5)), false),
        commands: Vec::new(),
    }], HashMap::new()); "right bound range")]
    #[test_case("3-", Editor::new(vec![Instruction{
        address: Between(Box::new(Location(3)), Box::new(Never), false),
        commands: Vec::new(),
    }], HashMap::new()); "left bound range")]
    #[test_case("13-72", Editor::new(vec![Instruction{
        address: Between(Box::new(Location(13)), Box::new(Location(72)), false),
        commands: Vec::new(),
    }], HashMap::new()); "range")]
    #[test_case(" 13  -   72 ", Editor::new(vec![Instruction{
        address: Between(Box::new(Location(13)), Box::new(Location(72)), false),
        commands: Vec::new(),
    }], HashMap::new()); "range with spaces")]
    #[test_case("13-72!", Editor::new(vec![Instruction{
        address: Negate(Box::new(Between(Box::new(Location(13)), Box::new(Location(72)), false))),
        commands: Vec::new(),
    }], HashMap::new()); "range negated")]
    #[test_case("/abc/", Editor::new(vec![Instruction{
        address: Regex(crate::Regex::from_str("abc").unwrap()),
        commands: Vec::new(),
    }], HashMap::new()); "regex match")]
    #[test_case(r"/abc\//", Editor::new(vec![Instruction{
        address: Regex(crate::Regex::from_str("abc/").unwrap()),
        commands: Vec::new(),
    }], HashMap::new()); "regex match with escape")]
    #[test_case("^abc$", Editor::new(vec![Instruction{
        address: Regex(crate::Regex::from_str("^abc$").unwrap()),
        commands: Vec::new(),
    }], HashMap::new()); "whole line regex match")]
    #[test_case(r"^\$abc$", Editor::new(vec![Instruction{
        address: Regex(crate::Regex::from_str(r"^\$abc$").unwrap()),
        commands: Vec::new(),
    }], HashMap::new()); "whole line regex match with escape")]
    #[test_case(r"^\$$", Editor::new(vec![Instruction{
        address: Regex(crate::Regex::from_str(r"^\$$").unwrap()),
        commands: Vec::new(),
    }], HashMap::new()); "whole line only dollar")]
    #[test_case("/abc/-/def/", Editor::new(vec![Instruction{
        address: Between(
            Box::new(Regex(crate::Regex::from_str("abc").unwrap())),
            Box::new(Regex(crate::Regex::from_str("def").unwrap())),
            false
        ),
        commands: Vec::new(),
    }], HashMap::new()); "regex range")]
    #[test_case("(1!)!", Editor::new(vec![Instruction{
        address: Location(1),
        commands: Vec::new(),
    }], HashMap::new()); "double negation")]
    #[test_case(" 666    ! ", Editor::new(vec![Instruction{
        address: Negate(Box::new(Location(666))),
        commands: Vec::new(),
    }], HashMap::new()); "negation with space")]
    #[test_case("5,6,10", Editor::new(vec![Instruction{
        address: Set(vec![Location(5), Location(6), Location(10)]),
        commands: Vec::new(),
    }], HashMap::new()); "set")]
    #[test_case("((5),((6),10))", Editor::new(vec![Instruction{
        address: Set(vec![Location(5), Location(6), Location(10)]),
        commands: Vec::new(),
    }], HashMap::new()); "set with brackets")]
    #[test_case("  5, 6  ,10   ", Editor::new(vec![Instruction{
        address: Set(vec![Location(5), Location(6), Location(10)]),
        commands: Vec::new(),
    }], HashMap::new()); "set with spaces")]
    #[test_case("5,6,10!", Editor::new(vec![Instruction{
        address: Set(vec![Location(5), Location(6), Negate(Box::new(Location(10)))]),
        commands: Vec::new(),
    }], HashMap::new()); "set negated")]
    #[test_case("(((42)))", Editor::new(vec![Instruction{
        address: Location(42),
        commands: Vec::new(),
    }], HashMap::new()); "brackets")]
    #[test_case(r"/abc\/123/", Editor::new(vec![Instruction{
        address: Regex(crate::Regex::from_str("abc/123").unwrap()),
        commands: Vec::new(),
    }], HashMap::new()); "regex")]
    #[test_case(r"s/abc/def/", Editor::new(vec![Instruction{
        address: Always,
        commands: vec![Substitute(
                crate::Regex::from_str("abc").unwrap(),
                "def".to_string(),
                0,
            )],
    }], HashMap::new()); "substitute")]
    #[test_case(r"s/abc/def/5", Editor::new(vec![Instruction{
        address: Always,
        commands: vec![Substitute(
                crate::Regex::from_str("abc").unwrap(),
                "def".to_string(),
                5,
            )],
    }], HashMap::new()); "substitute with count")]
    #[test_case(r"s/abc/def/g", Editor::new(vec![Instruction{
        address: Always,
        commands: vec![Substitute(
                crate::Regex::from_str("abc").unwrap(),
                "def".to_string(),
                0,
            )],
    }], HashMap::new()); "substitute with global count")]
    #[test_case(r"/abc/s/def/ghi/g", Editor::new(vec![Instruction{
        address: Regex(crate::Regex::from_str("abc").unwrap()),
        commands: vec![Substitute(
                crate::Regex::from_str("def").unwrap(),
                "ghi".to_string(),
                0,
            )],
    }], HashMap::new()); "condense match and substitute")]
    #[test_case(r"s/(abc)/__$123__/", Editor::new(vec![Instruction{
        address: Always,
        commands: vec![Substitute(
                crate::Regex::from_str("(abc)").unwrap(),
                "__${123}__".to_string(),
                0,
            )],
    }], HashMap::new()); "substitute with numbered group")]
    #[test_case(r"1d;3d;7d", Editor::new(vec![
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
    ], HashMap::new()); "multiple instructions")]
    fn parse(input: &str, expected: Editor) {
        let result = Editor::try_from(input.to_string()).unwrap();
        assert_eq!(result, expected)
    }
}
