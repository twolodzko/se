use std::io::StdoutLock;

use crate::{command, run, Action, Line, Status};
use anyhow::Result;
use std::io::Write;

#[derive(Debug, PartialEq)]
pub struct Program {
    pub(crate) actions: Vec<Action>,
    pub(crate) finally: Vec<command::Command>,
}

impl Program {
    pub fn run<R: Iterator<Item = Result<Line>>>(
        &self,
        reader: &mut R,
        print_all: bool,
        out: &mut StdoutLock,
    ) -> Result<(Status, usize)> {
        use Status::*;

        let mut matches = 0;
        let mut status = Normal;
        let mut hold = String::new();
        let mut pattern: Line = Line::default();

        while let Some(line) = reader.next() {
            pattern = line?;
            status = Normal;

            if let Some(s) = run(&self.actions, &mut pattern, &mut hold, reader, out)? {
                status = s;
                matches += 1;
            }

            if status == NoPrint {
                continue;
            }
            if print_all {
                writeln!(out, "{}", pattern.1)?;
            }
            if let Quit(_) = status {
                break;
            }
        }

        for cmd in self.finally.iter() {
            let s = cmd.run(&mut pattern, &mut hold, reader, out)?;
            if s != Status::Normal {
                status = s;
                break;
            }
        }

        Ok((status, matches))
    }
}

impl From<Vec<Action>> for Program {
    fn from(value: Vec<Action>) -> Self {
        Program {
            actions: value,
            finally: Vec::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{lines::MockReader, run, Line, Program};
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
        run(
            &func.actions,
            pattern,
            &mut String::new(),
            &mut MockReader {},
            &mut std::io::stdout().lock(),
        )
        .unwrap();
        assert_eq!(pattern.1, expected)
    }
}
