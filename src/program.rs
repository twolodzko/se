use crate::{
    Action, Memory, Reader, Result, Status,
    command::{self, Command},
};
use std::io::Write;

#[derive(Debug, PartialEq)]
pub struct Program {
    actions: Vec<Action>,
    finally: Vec<command::Command>,
    memory: Memory,
}

impl Program {
    pub(crate) fn new(actions: Vec<Action>, finally: Vec<command::Command>) -> Program {
        Program {
            actions,
            finally,
            memory: Memory::default(),
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

        while let Some(line) = reader.next() {
            self.memory.read(line?);
            status = Normal;

            if let Some(s) = self.process_line(reader, out)? {
                status = s;
                matches += 1;
            }

            if status == NoPrint {
                continue;
            }
            if print_all {
                writeln!(out, "{}", self.memory.this)?;
            }
            if let Quit(_) = status {
                break;
            }
        }

        for cmd in self.finally.iter() {
            let s = cmd.run(&mut self.memory, reader, out)?;
            if s != Status::Normal {
                status = s;
                break;
            }
        }

        Ok((status, matches))
    }

    fn process_line(&mut self, reader: &mut Reader, out: &mut dyn Write) -> Result<Option<Status>> {
        let mut status = None;
        let mut pos = 0;
        while pos < self.actions.len() {
            match &self.actions[pos] {
                Action::Condition(cond, jump) => {
                    if cond.matches(&self.memory) {
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
        if !self.finally.is_empty() {
            if !self.actions.is_empty() {
                write!(f, "; ")?;
            }
            write!(f, "$ ")?;
            for c in &self.finally {
                write!(f, "{} ", c)?;
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
        prog.memory.read(Line(0, "123456789".to_string()));
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
        prog.memory.read(Line(0, input.to_string()));
        prog.process_line(&mut Reader::empty(), &mut std::io::stdout().lock())
            .unwrap();
        assert_eq!(prog.memory.this, expected)
    }
}
