use std::collections::HashMap;

use crate::{command::Status, function::Function, Line};

#[derive(Debug, PartialEq)]
pub struct Program {
    pub(crate) main: Function,
    pub(crate) func: HashMap<String, Function>,
}

impl Program {
    pub(crate) fn call(&mut self, pattern: &mut Line, hold: &mut String) -> Option<Status> {
        self.main.call(pattern, hold, self.func)
    }
}

pub fn run<R: Iterator<Item = std::io::Result<Line>>>(
    reader: &mut R,
    program: &mut Program,
    print_all: bool,
) -> std::io::Result<(Status, usize)> {
    use Status::*;

    let mut matches = 0;
    let mut status = Normal;
    let mut hold = String::new();

    for line in reader {
        let pattern = &mut line?;
        status = Normal;

        if let Some(s) = program.call(pattern, &mut hold) {
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
