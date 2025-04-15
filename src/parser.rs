use crate::{
    address::Address::{self, *},
    command::{
        self,
        Command::{self, *},
    },
    editor::Instruction,
    Editor, Error,
};
use std::{iter::Peekable, str::Chars};

pub fn parse(input: &str) -> Result<Editor, Error> {
    let chars = &mut input.chars().peekable();
    let mut instructions = Vec::new();
    loop {
        instructions.push(parse_instruction(chars)?);
        if chars.peek().is_none() {
            break;
        }
    }
    Ok(Editor {
        instructions,
        counter: 0,
    })
}

fn parse_instruction(chars: &mut Peekable<Chars>) -> Result<Instruction, Error> {
    skip_whitespace(chars);
    let address = parse_addrs(chars)?;
    skip_whitespace(chars);
    let commands = parse_cmds(chars)?;
    Ok(Instruction { address, commands })
}

fn parse_addrs(chars: &mut Peekable<Chars>) -> Result<Address, Error> {
    let mut addrs = Vec::new();
    let mut has_any = false;
    loop {
        let mut addr = parse_brackets(chars)?;
        match addr {
            Always => has_any = true,
            Set(ref mut rhs) => addrs.append(rhs),
            _ => addrs.push(addr),
        }

        skip_whitespace(chars);
        if let Some(',') = chars.peek() {
            chars.next();
            skip_whitespace(chars);
        } else {
            break;
        }
    }

    // optimizations
    if has_any {
        return Ok(Always);
    }
    if addrs.len() == 1 {
        return Ok(addrs.remove(0));
    }
    Ok(Set(addrs))
}

fn parse_brackets(chars: &mut Peekable<Chars>) -> Result<Address, Error> {
    if let Some('(') = chars.peek() {
        chars.next();
        skip_whitespace(chars);
        let addr = parse_addrs(chars)?;
        skip_whitespace(chars);
        if chars.next() != Some(')') {
            return Err(Error::Missing(')'));
        }
        Ok(maybe_negate(addr, chars))
    } else {
        let addr = parse_range(chars)?;
        skip_whitespace(chars);
        Ok(maybe_negate(addr, chars))
    }
}

fn parse_range(chars: &mut Peekable<Chars>) -> Result<Address, Error> {
    let lhs = parse_simple_addr(chars)?.unwrap_or(Always);
    skip_whitespace(chars);
    if let Some('-') = chars.peek() {
        chars.next();
        let rhs = parse_simple_addr(chars)?.unwrap_or(Never);
        if let (Location(lo), Location(hi)) = (&lhs, &rhs) {
            if lo > hi {
                return Err(Error::InvalidAddr(format!(
                    "{} > {} in {}-{}",
                    lo, hi, lo, hi
                )));
            }
        }
        return Ok(Between(Box::new(lhs), Box::new(rhs), false));
    }
    Ok(lhs)
}

fn parse_simple_addr(chars: &mut Peekable<Chars>) -> Result<Option<Address>, Error> {
    if let Some(&c) = chars.peek() {
        match c {
            '/' => {
                chars.next();
                return Ok(Some(Regex(parse_regex(chars)?)));
            }
            c if c.is_ascii_digit() => {
                let s = read_integer(chars);
                return match s.parse() {
                    Ok(num) => {
                        if num == 0 {
                            return Err(Error::InvalidAddr(s));
                        }
                        Ok(Some(Location(num)))
                    }
                    Err(err) => Err(Error::ParseInt(err)),
                };
            }
            '*' => {
                // the "any" match is default, no need to specify
                chars.next();
            }
            '$' => {
                chars.next();
                return Ok(Some(Never));
            }
            _ => (),
        }
    }
    Ok(None)
}

fn maybe_negate(addr: Address, chars: &mut Peekable<Chars>) -> Address {
    match chars.peek() {
        Some('!') => {
            chars.next();
            !addr
        }
        _ => addr,
    }
}

fn parse_cmds(chars: &mut Peekable<Chars>) -> Result<Vec<Command>, Error> {
    let mut cmds = Vec::new();
    while let Some(c) = chars.next() {
        let cmd = match c {
            ';' => break,
            'p' => Print,
            'l' => Escape,
            's' => {
                skip_whitespace(chars);
                parse_substitute(chars)?
            }
            '=' => LineNumber,
            'n' => Newline,
            'd' => Delete,
            'q' => {
                skip_whitespace(chars);
                let s = read_integer(chars);
                let code = if s.is_empty() {
                    0
                } else {
                    s.parse().map_err(Error::ParseInt)?
                };
                Quit(code)
            }
            '\'' | '"' => {
                let msg = unescape(read_until(chars, c)?)?;
                Insert(msg)
            }
            c if c.is_whitespace() => continue,
            _ => return Err(Error::Unexpected(c)),
        };
        cmds.push(cmd);
    }
    Ok(cmds)
}

fn parse_substitute(chars: &mut Peekable<Chars>) -> Result<Command, Error> {
    if chars.next() != Some('/') {
        return Err(Error::Missing('/'));
    }

    // Parse: s/src/dst/[limit]
    let src = parse_regex(chars)?;
    let dst = unescape(read_until(chars, '/')?)?;
    skip_whitespace(chars);

    let mut limit = 0;
    if let Some(c) = chars.peek() {
        if *c == 'g' {
            chars.next();
            // g is default, no need to update the limit
        } else if c.is_ascii_digit() {
            limit = read_integer(chars).parse().map_err(Error::ParseInt)?;
        }
    }

    Ok(Substitute(command::Replacer {
        regex: src,
        template: dst,
        limit,
    }))
}

fn read_until(chars: &mut Peekable<Chars>, delim: char) -> Result<String, Error> {
    let mut acc = String::new();
    while let Some(c) = chars.next() {
        match c {
            c if c == delim => return Ok(acc),
            '\\' => {
                if let Some(e) = chars.next() {
                    if e != delim {
                        acc.push(c);
                    }
                    acc.push(e);
                } else {
                    acc.push(c);
                    return Err(Error::InvalidAddr(acc));
                }
            }
            _ => acc.push(c),
        }
    }
    Err(Error::Missing('/'))
}

fn skip_whitespace(chars: &mut Peekable<Chars>) {
    while chars.peek().is_some_and(|c| c.is_whitespace()) {
        chars.next();
    }
}

fn read_integer(chars: &mut Peekable<Chars>) -> String {
    let mut num = String::new();
    loop {
        match chars.peek() {
            Some(c) if c.is_ascii_digit() => num.push(*c),
            _ => break,
        }
        chars.next();
    }
    num
}

fn unescape(s: String) -> Result<String, Error> {
    unescape::unescape(&s).ok_or(Error::ParsingError(s))
}

fn parse_regex(chars: &mut Peekable<Chars>) -> Result<regex::Regex, Error> {
    let regex = read_until(chars, '/')?;
    regex::Regex::new(&regex).map_err(Error::Regex)
}

#[cfg(test)]
mod tests {
    use crate::{
        address::Address::*,
        command::{Command::*, Replacer},
        editor::Instruction,
        Editor,
    };
    use test_case::test_case;

    #[test_case("", Editor::new(vec![Instruction{
        address: Always,
        commands: Vec::new(),
    }]); "empty")]
    #[test_case("*", Editor::new(vec![Instruction{
        address: Always,
        commands: Vec::new()
    }]); "any")]
    #[test_case("p", Editor::new(vec![Instruction{
        address: Always,
        commands: vec![Print]
    }]); "print all")]
    #[test_case("=np", Editor::new(vec![Instruction{
        address: Always,
        commands: vec![LineNumber, Newline, Print]
    }]); "print with newlines")]
    #[test_case("   = n  p  ", Editor::new(vec![Instruction{
        address: Always,
        commands: vec![LineNumber, Newline, Print]
    }]); "commands with spaces")]
    #[test_case("-", Editor::new(vec![Instruction{
        address: Between(Box::new(Always), Box::new(Never), false),
        commands: Vec::new()
    }]); "infinite range")]
    #[test_case("-5", Editor::new(vec![Instruction{
        address: Between(Box::new(Always), Box::new(Location(5)), false),
        commands: Vec::new(),
    }]); "right bound range")]
    #[test_case("3-", Editor::new(vec![Instruction{
        address: Between(Box::new(Location(3)), Box::new(Never), false),
        commands: Vec::new(),
    }]); "left bound range")]
    #[test_case("13-72", Editor::new(vec![Instruction{
        address: Between(Box::new(Location(13)), Box::new(Location(72)), false),
        commands: Vec::new(),
    }]); "range")]
    #[test_case("13-72!", Editor::new(vec![Instruction{
        address: Negate(Box::new(Between(Box::new(Location(13)), Box::new(Location(72)), false))),
        commands: Vec::new(),
    }]); "range negated")]
    #[test_case("/abc/-/def/", Editor::new(vec![Instruction{
        address: Between(
            Box::new(Regex(regex::Regex::new("abc").unwrap())),
            Box::new(Regex(regex::Regex::new("def").unwrap())),
            false
        ),
        commands: Vec::new(),
    }]); "regex range")]
    #[test_case("(1!)!", Editor::new(vec![Instruction{
        address: Location(1),
        commands: Vec::new(),
    }]); "double negation")]
    #[test_case(" 666    ! ", Editor::new(vec![Instruction{
        address: Negate(Box::new(Location(666))),
        commands: Vec::new(),
    }]); "negation with space")]
    #[test_case("5,6,10", Editor::new(vec![Instruction{
        address: Set(vec![Location(5), Location(6), Location(10)]),
        commands: Vec::new(),
    }]); "set")]
    #[test_case("((5),((6),10))", Editor::new(vec![Instruction{
        address: Set(vec![Location(5), Location(6), Location(10)]),
        commands: Vec::new(),
    }]); "set with brackets")]
    #[test_case("  5, 6  ,10   ", Editor::new(vec![Instruction{
        address: Set(vec![Location(5), Location(6), Location(10)]),
        commands: Vec::new(),
    }]); "set with spaces")]
    #[test_case("5,6,10!", Editor::new(vec![Instruction{
        address: Set(vec![Location(5), Location(6), Negate(Box::new(Location(10)))]),
        commands: Vec::new(),
    }]); "set negated")]
    #[test_case("(((42)))", Editor::new(vec![Instruction{
        address: Location(42),
        commands: Vec::new(),
    }]); "brackets")]
    #[test_case(r"/abc\/123/", Editor::new(vec![Instruction{
        address: Regex(regex::Regex::new("abc/123").unwrap()),
        commands: Vec::new(),
    }]); "regex")]
    #[test_case(r"s/abc/def/", Editor::new(vec![Instruction{
        address: Always,
        commands: vec![Substitute(Replacer{
                regex: regex::Regex::new("abc").unwrap(),
                template: "def".to_string(),
                limit: 0,
            })],
    }]); "substitute")]
    #[test_case(r"s/abc/def/5", Editor::new(vec![Instruction{
        address: Always,
        commands: vec![Substitute(Replacer{
                regex: regex::Regex::new("abc").unwrap(),
                template: "def".to_string(),
                limit: 5,
            })],
    }]); "substitute with count")]
    #[test_case(r"s/abc/def/g", Editor::new(vec![Instruction{
        address: Always,
        commands: vec![Substitute(Replacer{
                regex: regex::Regex::new("abc").unwrap(),
                template: "def".to_string(),
                limit: 0,
            })],
    }]); "substitute with global count")]
    #[test_case(r"s   /abc/def/   5", Editor::new(vec![Instruction{
        address: Always,
        commands: vec![Substitute(Replacer{
                regex: regex::Regex::new("abc").unwrap(),
                template: "def".to_string(),
                limit: 5,
            })],
    }]); "substitute with count after spaces")]
    #[test_case(r"/abc/s/def/ghi/g", Editor::new(vec![Instruction{
        address: Regex(regex::Regex::new("abc").unwrap()),
        commands: vec![Substitute(Replacer{
                regex: regex::Regex::new("def").unwrap(),
                template: "ghi".to_string(),
                limit: 0,
            })],
    }]); "condense match and substitute")]
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
    ]); "multiple instructions")]
    fn parse(input: &str, expected: Editor) {
        let result = crate::parse(input).unwrap();
        assert_eq!(result, expected)
    }
}
