use std::collections::{HashMap, VecDeque};

use crate::{address, command, Line, Status};

#[derive(Debug, PartialEq, Clone)]
pub struct Function(
    pub(crate) VecDeque<Action>,
    pub(crate) HashMap<String, Vec<Action>>,
);

#[derive(Debug, PartialEq, Clone)]
pub(crate) enum Action {
    Condition(address::Address, usize),
    Command(command::Command),
}

impl Function {
    pub(crate) fn call(&self, pattern: &mut Line, hold: &mut String) -> Option<Status> {
        let mut status = None;
        let mut body = self.0.clone();
        while let Some(node) = body.pop_front() {
            match node {
                Action::Command(command::Command::Call(ref name)) => {
                    if let Some(func) = self.1.get(name) {
                        for node in func.iter().rev() {
                            body.push_front(node.clone());
                        }
                    } else {
                        eprintln!("Error: unknown function: {}", name);
                        std::process::exit(1);
                    }
                }
                Action::Condition(cond, jump) => {
                    if cond.matches(pattern) {
                        status = Some(Status::Normal);
                    } else {
                        for _ in 0..jump {
                            if body.pop_front().is_none() {
                                break;
                            }
                        }
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
        }
        status
    }

    pub fn process<R: Iterator<Item = std::io::Result<Line>>>(
        &self,
        reader: &mut R,
        print_all: bool,
    ) -> std::io::Result<(Status, usize)> {
        use Status::*;

        let mut matches = 0;
        let mut status = Normal;
        let mut hold = String::new();

        for line in reader {
            let pattern = &mut line?;
            status = Normal;

            if let Some(s) = self.call(pattern, &mut hold) {
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

        Ok((status, matches))
    }
}

impl From<Vec<Action>> for Function {
    fn from(value: Vec<Action>) -> Self {
        Function(value.into(), HashMap::new())
    }
}
