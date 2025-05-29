use crate::{address, command, Line, Status};

#[derive(Debug, PartialEq)]
pub struct Program(pub(crate) Vec<Action>, pub(crate) Vec<command::Command>);

#[derive(Debug, PartialEq)]
pub(crate) enum Action {
    Condition(address::Address, usize),
    Command(command::Command),
}

impl Program {
    pub fn run<R: Iterator<Item = std::io::Result<Line>>>(
        &self,
        reader: &mut R,
        print_all: bool,
    ) -> std::io::Result<(Status, usize)> {
        use Status::*;

        let mut matches = 0;
        let mut status = Normal;
        let mut hold = String::new();
        let mut pattern: Line = Line::default();

        for line in reader {
            pattern = line?;
            status = Normal;

            if let Some(s) = self.process(&mut pattern, &mut hold) {
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
            let s = cmd.run(&mut pattern, &mut hold);
            if s != Status::Normal {
                status = s;
                break;
            }
        }

        Ok((status, matches))
    }

    pub(crate) fn process(&self, pattern: &mut Line, hold: &mut String) -> Option<Status> {
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
                Action::Command(cmd) => {
                    let s = cmd.run(pattern, hold);
                    if s != Status::Normal {
                        status = Some(s);
                        break;
                    }
                }
            }
            pos += 1;
        }
        status
    }
}

impl From<Vec<Action>> for Program {
    fn from(value: Vec<Action>) -> Self {
        Program(value, Vec::new())
    }
}
