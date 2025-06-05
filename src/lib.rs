mod address;
mod command;
mod lines;
mod parser;
mod program;

use anyhow::Result;
pub use {
    command::Status,
    lines::{FilesReader, Line, StdinReader},
    program::Program,
};

#[derive(Debug, Clone)]
pub(crate) struct Regex(regex::Regex);

#[derive(Debug, PartialEq)]
pub(crate) enum Action {
    Condition(address::Address, usize),
    Command(command::Command),
}

impl std::fmt::Display for Action {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use Action::*;
        match self {
            Condition(a, _) => a.fmt(f),
            Command(c) => c.fmt(f),
        }
    }
}

impl std::str::FromStr for Regex {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> std::result::Result<Regex, Self::Err> {
        let regex = regex::Regex::new(s)?;
        Ok(Regex(regex))
    }
}

impl PartialEq for Regex {
    fn eq(&self, other: &Self) -> bool {
        self.0.as_str() == other.0.as_str()
    }
}

impl std::fmt::Display for Regex {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

fn run<R: Iterator<Item = Result<Line>>>(
    actions: &[Action],
    pattern: &mut Line,
    hold: &mut String,
    reader: &mut R,
) -> Result<Option<Status>> {
    let mut status = None;
    let mut pos = 0;
    while pos < actions.len() {
        match &actions[pos] {
            Action::Condition(cond, jump) => {
                if cond.matches(pattern) {
                    status = Some(Status::Normal);
                } else {
                    pos += jump;
                }
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
