use crate::{address, command, Line, Status};
use anyhow::Result;

#[derive(Debug, PartialEq)]
pub struct Program(pub(crate) Vec<Action>, pub(crate) Vec<command::Command>);

#[derive(Debug, PartialEq)]
pub(crate) enum Action {
    Condition(address::Address, usize),
    Command(command::Command),
}

impl Program {
    pub fn run<R: Iterator<Item = Result<Line>>>(
        &self,
        reader: &mut R,
        print_all: bool,
    ) -> Result<(Status, usize)> {
        use Status::*;

        let mut matches = 0;
        let mut status = Normal;
        let mut hold = String::new();
        let mut pattern: Line = Line::default();

        while let Some(line) = reader.next() {
            pattern = line?;
            status = Normal;

            if let Some(s) = self.process(&mut pattern, &mut hold, reader)? {
                status = s;
                matches += 1;
            }

            if status == NoPrint {
                continue;
            }
            if print_all {
                println!("{}", pattern.1)
            }
            if let Quit(_) = status {
                break;
            }
        }

        for cmd in self.1.iter() {
            let s = cmd.run(&mut pattern, &mut hold, reader)?;
            if s != Status::Normal {
                status = s;
                break;
            }
        }

        Ok((status, matches))
    }

    fn process<R: Iterator<Item = Result<Line>>>(
        &self,
        pattern: &mut Line,
        hold: &mut String,
        reader: &mut R,
    ) -> Result<Option<Status>> {
        let mut status = None;
        let mut pos = 0;
        while pos < self.0.len() {
            match &self.0[pos] {
                Action::Condition(cond, jump) => {
                    if cond.matches(pattern) {
                        status = Some(Status::Normal);
                    } else {
                        pos += jump;
                    }
                }
                Action::Command(command::Command::GoTo(_, n)) => {
                    pos = *n;
                }
                Action::Command(cmd) => {
                    let s = cmd.run(pattern, hold, reader)?;
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
        Program(value, Vec::new())
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
        let func = Program::from_str(command).unwrap();
        let pattern = &mut Line(0, "123456789".to_string());
        func.process(pattern, &mut String::new(), &mut MockReader {})
            .unwrap();
        assert_eq!(pattern.1, expected)
    }
}
