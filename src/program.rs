use std::io::StdoutLock;

use crate::{command, Action, Line, Status};
use anyhow::Result;
use std::io::Write;

#[derive(Debug, PartialEq)]
pub struct Program {
    actions: Vec<Action>,
    finally: Vec<command::Command>,
    memory: Memory,
}

#[derive(Debug, PartialEq, Default)]
pub(crate) struct Memory {
    pub(crate) line: Line,
    pub(crate) this: String,
    pub(crate) hold: String,
}

impl Memory {
    pub(crate) fn read(&mut self, line: Line) {
        self.this = line.1.clone();
        self.line = line;
    }
}

impl Program {
    pub(crate) fn new(actions: Vec<Action>, finally: Vec<command::Command>) -> Program {
        Program {
            actions,
            finally,
            memory: Memory::default(),
        }
    }

    pub fn run<R: Iterator<Item = Result<Line>>>(
        &mut self,
        reader: &mut R,
        print_all: bool,
        out: &mut StdoutLock,
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

    fn process_line<R: Iterator<Item = Result<Line>>>(
        &mut self,
        reader: &mut R,
        out: &mut StdoutLock,
    ) -> Result<Option<Status>> {
        let mut status = None;
        let mut pos = 0;
        while pos < self.actions.len() {
            match &self.actions[pos] {
                Action::Condition(cond, jump) => {
                    if cond.matches(&self.memory.line) {
                        status = Some(Status::Normal);
                    } else {
                        pos += jump;
                    }
                }
                Action::Command(cmd) => {
                    let s = cmd.run(&mut self.memory, reader, out)?;
                    if s != Status::Normal {
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

impl From<Vec<Action>> for Program {
    fn from(value: Vec<Action>) -> Self {
        Program {
            actions: value,
            finally: Vec::new(),
            memory: Memory::default(),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{lines::MockReader, Line, Program};
    use std::str::FromStr;
    use test_case::test_case;

    #[test_case(
        "k3-5",
        "345";
        "range"
    )]
    #[test_case(
        "k-5",
        "12345";
        "left-open range"
    )]
    #[test_case(
        "k5",
        "5";
        "n-th chars"
    )]
    #[test_case(
        "k3-",
        "3456789";
        "right-open range"
    )]
    #[test_case(
        "k1-1",
        "1";
        "single item range"
    )]
    #[test_case(
        "k1",
        "1";
        "first item"
    )]
    fn keep(command: &str, expected: &str) {
        let mut prog = Program::from_str(command).unwrap();
        prog.memory.read(Line(0, "123456789".to_string()));
        prog.process_line(&mut MockReader {}, &mut std::io::stdout().lock())
            .unwrap();
        assert_eq!(prog.memory.this, expected)
    }
}
