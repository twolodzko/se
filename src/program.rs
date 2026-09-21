use crate::{Action, Memory, Reader, Result, Status, command::Command};
use std::io::Write;

#[derive(Debug, PartialEq)]
pub struct Program {
    actions: Vec<Action>,
    memory: Memory,
    check_final: bool,
}

impl Program {
    pub(crate) fn new(actions: Vec<Action>) -> Program {
        let check_final = actions.iter().any(|a| {
            if let Action::Condition(c, _) = a {
                c.contains_final()
            } else {
                false
            }
        });
        Program {
            actions,
            memory: Memory::default(),
            check_final,
        }
    }

    pub fn run(
        &mut self,
        reader: &mut Reader,
        print_all: bool,
        out: &mut dyn Write,
    ) -> Result<(Status, usize)> {
        use Status::*;

        let mut matches = 0;
        let mut status = Normal;

        // before processing lines
        if self.check_final {
            self.memory.end = reader.peek().is_none();
        }
        if let Some(s) = self.process_line(reader, out)? {
            status = s;
        }

        // process lines
        while self.read_line(reader)? && !matches!(status, Quit(_)) {
            status = Normal;
            if let Some(s) = self.process_line(reader, out)? {
                status = s;
                matches += 1;
            }
            if status != NoPrint && print_all {
                writeln!(out, "{}", self.memory.this)?;
            }
        }

        Ok((status, matches))
    }

    /// Process line and return status. `None` for no match.
    fn process_line(&mut self, reader: &mut Reader, out: &mut dyn Write) -> Result<Option<Status>> {
        let mut status = None;
        let mut pos = 0;
        while pos < self.actions.len() {
            match &self.actions[pos] {
                Action::Condition(cond, jump) => {
                    if cond.is_match(&self.memory) {
                        status = Some(Status::Normal);
                    } else {
                        pos += jump;
                    }
                }
                Action::Command(cmd) => {
                    let s = cmd.run(&mut self.memory, reader, out)?;
                    if let Status::GoTo(idx) = s {
                        pos = idx;
                    } else if s != Status::Normal {
                        status = Some(s);
                        break;
                    }
                }
            }
            pos += 1;
        }
        Ok(status)
    }

    fn read_line(&mut self, reader: &mut Reader) -> Result<bool> {
        if let Some(line) = reader.next() {
            self.memory.read(line?);
            if self.check_final {
                self.memory.end = reader.peek().is_none();
            }
            return Ok(true);
        }
        Ok(false)
    }
}

impl std::fmt::Display for Program {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut iter = self.actions.iter().peekable();
        while let Some(a) = iter.next() {
            write!(f, "{} ", a)?;
            if let Action::Command(c) = a
                && matches!(c, Command::Break)
            {
                continue;
            }
            if let Some(Action::Condition(_, _)) = iter.peek() {
                write!(f, "; ")?;
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::{Line, Program, Reader};
    use std::str::FromStr;
    use test_case::test_case;

    #[test_case(
        "c3-5",
        "345";
        "range"
    )]
    #[test_case(
        "c-5",
        "12345";
        "left-open range"
    )]
    #[test_case(
        "c5",
        "5";
        "n-th chars"
    )]
    #[test_case(
        "c3-",
        "3456789";
        "right-open range"
    )]
    #[test_case(
        "c1-1",
        "1";
        "single item range"
    )]
    #[test_case(
        "c1",
        "1";
        "first item"
    )]
    fn keep(command: &str, expected: &str) {
        let mut prog = Program::from_str(command).unwrap();
        prog.memory.read(Line(1, "123456789".to_string()));
        prog.process_line(&mut Reader::empty(), &mut std::io::stdout().lock())
            .unwrap();
        assert_eq!(prog.memory.this, expected)
    }

    #[test_case(
        r"i'>'*3",
        "foo",
        ">>>foo";
        "prepend"
    )]
    #[test_case(
        r"a'='*2",
        "foo",
        "foo==";
        "append"
    )]
    #[test_case(
        r"s/[^x]*(?P<n>x+)[^x]*/\{n}/",
        "fooxxxbaz",
        "xxx";
        "substitute with named group"
    )]
    #[test_case(
        r"s/a/#/3",
        "fata morgana",
        "f#t# morg#na";
        "substitute with limit"
    )]
    #[test_case(
        r"s/a/#/ s/b/#/; s/c/#/. s/d/#/",
        "abracadabra",
        "##r###d##r#";
        "sequential substitutions"
    )]
    #[test_case(
        r"s/foo/WRONG/",
        "this is fine",
        "this is fine";
        "substitute for no match"
    )]
    #[test_case(
        r"z",
        "clear me!",
        "";
        "clear memory"
    )]
    #[test_case(
        "z'hello, world!'",
        "replace me",
        "hello, world!";
        "replace string"
    )]
    fn run(command: &str, input: &str, expected: &str) {
        let mut prog = Program::from_str(command).unwrap();
        prog.memory.read(Line(1, input.to_string()));
        prog.process_line(&mut Reader::empty(), &mut std::io::stdout().lock())
            .unwrap();
        assert_eq!(prog.memory.this, expected)
    }
}
