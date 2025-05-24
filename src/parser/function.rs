use super::{
    address, command,
    reader::{FileReader, Reader, StringReader},
    utils::{self, skip_whitespace},
};
use crate::{
    function::{Function, Instruction, Library},
    Error,
};
use std::{cell::RefCell, collections::HashMap, rc::Rc, str::FromStr};

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
    let library = Rc::new(RefCell::new(HashMap::new()));
    while let Some(c) = reader.peek()? {
        match c {
            '@' => {
                reader.next()?;
                let (name, func) = parse_function(reader, &library)?;
                library.borrow_mut().insert(name, func);
            }
            _ => {
                instructions.push(parse_instruction(reader)?);
            }
        }
        skip_whitespace(reader);
    }
    Ok(Function(instructions, library))
}

fn parse_instruction<R: Reader>(reader: &mut R) -> Result<Instruction, Error> {
    // [address][commands]
    utils::skip_whitespace(reader);
    let address = address::parse(reader)?;
    utils::skip_whitespace(reader);
    let commands = command::parse(reader)?;
    Ok(Instruction { address, commands })
}

fn parse_function<R: Reader>(reader: &mut R, lib: &Library) -> Result<(String, Function), Error> {
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
        instructions.push(parse_instruction(reader)?);

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

    Ok((name, Function(instructions, lib.clone())))
}

#[cfg(test)]
mod tests {
    use crate::{
        address::{Address::*, Bool},
        command::Command::*,
        function::{Function, Instruction},
    };
    use std::{cell::RefCell, collections::HashMap, rc::Rc, str::FromStr};
    use test_case::test_case;

    #[test_case("", Function(Vec::new(), Rc::new(RefCell::new(HashMap::new()))); "empty")]
    #[test_case("*", Function(vec![Instruction{
        address: Always,
        commands: Vec::new()
    }], Rc::new(RefCell::new(HashMap::new()))); "any")]
    #[test_case("p", Function(vec![Instruction{
        address: Always,
        commands: vec![Println]
    }], Rc::new(RefCell::new(HashMap::new()))); "print all")]
    #[test_case(r"=\np", Function(vec![Instruction{
        address: Always,
        commands: vec![LineNumber, Insert("\n".to_string()), Println]
    }], Rc::new(RefCell::new(HashMap::new()))); "print with newlines")]
    #[test_case(r"   = \n  p  ", Function(vec![Instruction{
        address: Always,
        commands: vec![LineNumber, Insert("\n".to_string()), Println]
    }], Rc::new(RefCell::new(HashMap::new()))); "commands with spaces")]
    #[test_case("-", Function(vec![Instruction{
        address: Between(Box::new(Location(1)), Box::new(Never), Bool::new(false)),
        commands: Vec::new()
    }], Rc::new(RefCell::new(HashMap::new()))); "infinite range")]
    #[test_case("-5", Function(vec![Instruction{
        address: Between(Box::new(Location(1)), Box::new(Location(5)), Bool::new(false)),
        commands: Vec::new(),
    }], Rc::new(RefCell::new(HashMap::new()))); "right bound range")]
    #[test_case("3-", Function(vec![Instruction{
        address: Between(Box::new(Location(3)), Box::new(Never), Bool::new(false)),
        commands: Vec::new(),
    }], Rc::new(RefCell::new(HashMap::new()))); "left bound range")]
    #[test_case("13-72", Function(vec![Instruction{
        address: Between(Box::new(Location(13)), Box::new(Location(72)), Bool::new(false)),
        commands: Vec::new(),
    }], Rc::new(RefCell::new(HashMap::new()))); "range")]
    #[test_case(" 13  -   72 ", Function(vec![Instruction{
        address: Between(Box::new(Location(13)), Box::new(Location(72)), Bool::new(false)),
        commands: Vec::new(),
    }], Rc::new(RefCell::new(HashMap::new()))); "range with spaces")]
    #[test_case("13-72!", Function(vec![Instruction{
        address: Negate(Box::new(Between(Box::new(Location(13)), Box::new(Location(72)), Bool::new(false)))),
        commands: Vec::new(),
    }], Rc::new(RefCell::new(HashMap::new()))); "range negated")]
    #[test_case("/abc/", Function(vec![Instruction{
        address: Regex(crate::Regex::from_str("abc").unwrap()),
        commands: Vec::new(),
    }], Rc::new(RefCell::new(HashMap::new()))); "regex match")]
    #[test_case(r"/abc\//", Function(vec![Instruction{
        address: Regex(crate::Regex::from_str("abc/").unwrap()),
        commands: Vec::new(),
    }], Rc::new(RefCell::new(HashMap::new()))); "regex match with escape")]
    #[test_case("^abc$", Function(vec![Instruction{
        address: Regex(crate::Regex::from_str("^abc$").unwrap()),
        commands: Vec::new(),
    }], Rc::new(RefCell::new(HashMap::new()))); "whole line regex match")]
    #[test_case(r"^\$abc$", Function(vec![Instruction{
        address: Regex(crate::Regex::from_str(r"^\$abc$").unwrap()),
        commands: Vec::new(),
    }], Rc::new(RefCell::new(HashMap::new()))); "whole line regex match with escape")]
    #[test_case(r"^\$$", Function(vec![Instruction{
        address: Regex(crate::Regex::from_str(r"^\$$").unwrap()),
        commands: Vec::new(),
    }], Rc::new(RefCell::new(HashMap::new()))); "whole line only dollar")]
    #[test_case("/abc/-/def/", Function(vec![Instruction{
        address: Between(
            Box::new(Regex(crate::Regex::from_str("abc").unwrap())),
            Box::new(Regex(crate::Regex::from_str("def").unwrap())),
            Bool::new(false)
        ),
        commands: Vec::new(),
    }], Rc::new(RefCell::new(HashMap::new()))); "regex range")]
    #[test_case("(1!)!", Function(vec![Instruction{
        address: Location(1),
        commands: Vec::new(),
    }], Rc::new(RefCell::new(HashMap::new()))); "double negation")]
    #[test_case(" 666    ! ", Function(vec![Instruction{
        address: Negate(Box::new(Location(666))),
        commands: Vec::new(),
    }], Rc::new(RefCell::new(HashMap::new()))); "negation with space")]
    #[test_case("5,6,10", Function(vec![Instruction{
        address: Set(vec![Location(5), Location(6), Location(10)]),
        commands: Vec::new(),
    }], Rc::new(RefCell::new(HashMap::new()))); "set")]
    #[test_case("((5),((6),10))", Function(vec![Instruction{
        address: Set(vec![Location(5), Location(6), Location(10)]),
        commands: Vec::new(),
    }], Rc::new(RefCell::new(HashMap::new()))); "set with brackets")]
    #[test_case("  5, 6  ,10   ", Function(vec![Instruction{
        address: Set(vec![Location(5), Location(6), Location(10)]),
        commands: Vec::new(),
    }], Rc::new(RefCell::new(HashMap::new()))); "set with spaces")]
    #[test_case("5,6,10!", Function(vec![Instruction{
        address: Set(vec![Location(5), Location(6), Negate(Box::new(Location(10)))]),
        commands: Vec::new(),
    }], Rc::new(RefCell::new(HashMap::new()))); "set negated")]
    #[test_case("(((42)))", Function(vec![Instruction{
        address: Location(42),
        commands: Vec::new(),
    }], Rc::new(RefCell::new(HashMap::new()))); "brackets")]
    #[test_case(r"/abc\/123/", Function(vec![Instruction{
        address: Regex(crate::Regex::from_str("abc/123").unwrap()),
        commands: Vec::new(),
    }], Rc::new(RefCell::new(HashMap::new()))); "regex")]
    #[test_case(r"s/abc/def/", Function(vec![Instruction{
        address: Always,
        commands: vec![Substitute(
                crate::Regex::from_str("abc").unwrap(),
                "def".to_string(),
                0,
            )],
    }], Rc::new(RefCell::new(HashMap::new()))); "substitute")]
    #[test_case(r"s/abc/def/5", Function(vec![Instruction{
        address: Always,
        commands: vec![Substitute(
                crate::Regex::from_str("abc").unwrap(),
                "def".to_string(),
                5,
            )],
    }], Rc::new(RefCell::new(HashMap::new()))); "substitute with count")]
    #[test_case(r"s/abc/def/g", Function(vec![Instruction{
        address: Always,
        commands: vec![Substitute(
                crate::Regex::from_str("abc").unwrap(),
                "def".to_string(),
                0,
            )],
    }], Rc::new(RefCell::new(HashMap::new()))); "substitute with global count")]
    #[test_case(r"/abc/s/def/ghi/g", Function(vec![Instruction{
        address: Regex(crate::Regex::from_str("abc").unwrap()),
        commands: vec![Substitute(
                crate::Regex::from_str("def").unwrap(),
                "ghi".to_string(),
                0,
            )],
    }], Rc::new(RefCell::new(HashMap::new()))); "condense match and substitute")]
    #[test_case(r"s/(abc)/__$123__/", Function(vec![Instruction{
        address: Always,
        commands: vec![Substitute(
                crate::Regex::from_str("(abc)").unwrap(),
                "__${123}__".to_string(),
                0,
            )],
    }], Rc::new(RefCell::new(HashMap::new()))); "substitute with numbered group")]
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
    ], Rc::new(RefCell::new(HashMap::new()))); "multiple instructions")]
    fn parse(input: &str, expected: Function) {
        let result = Function::from_str(input).unwrap();
        assert_eq!(result, expected)
    }
}
