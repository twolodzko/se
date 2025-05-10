use crate::{
    address::Address,
    command::{Action, Command},
    Line,
};

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

    pub fn apply(&mut self, line: &str) -> Option<(String, Action)> {
        self.counter += 1;
        let mut matched = false;
        let mut buffer = Line(self.counter, line.to_string());

        for instruction in self.instructions.iter_mut() {
            if instruction.address.matches(&buffer) {
                for cmd in instruction.commands.iter() {
                    use Action::*;
                    if let act @ (Quit(_) | Skip | End) = cmd.apply(&mut buffer) {
                        return Some((buffer.1, act));
                    }
                }
                matched = true;
            }
        }

        if matched {
            Some((buffer.1, Action::None))
        } else {
            None
        }
    }
}
