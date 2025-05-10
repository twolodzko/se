use crate::{address::Address, command::Command, Line};

#[derive(Debug, PartialEq)]
pub struct Editor {
    pub(crate) instructions: Vec<Instruction>,
    pub(crate) counter: usize,
}

#[derive(Debug, PartialEq)]
pub(crate) struct Instruction {
    pub(crate) address: Address,
    pub(crate) commands: Vec<Command>,
}

impl Editor {
    #[cfg(test)]
    pub(crate) fn new(instructions: Vec<Instruction>) -> Self {
        Self {
            instructions,
            counter: 0,
        }
    }

    pub fn apply(&mut self, line: &str) -> Option<(String, Command)> {
        use Command::*;

        self.counter += 1;
        let mut matched = false;
        let mut buffer = Line(self.counter, line.to_string());

        for instruction in self.instructions.iter_mut() {
            if instruction.address.matches(&buffer) {
                for cmd in instruction.commands.iter() {
                    match &cmd {
                        Delete | Stop | Quit(_) => return Some((buffer.1, cmd.clone())),
                        _ => cmd.apply(&mut buffer),
                    }
                }
                matched = true;
            }
        }

        if matched {
            Some((buffer.1, NoOp))
        } else {
            None
        }
    }
}
