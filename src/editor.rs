use crate::{address::Address, command::Command, Line};

#[derive(Debug, PartialEq)]
pub struct Editor {
    pub(crate) instructions: Vec<Instruction>,
    pub(crate) counter: usize,
    hold: String,
}

#[derive(Debug, PartialEq)]
pub(crate) struct Instruction {
    pub(crate) address: Address,
    pub(crate) commands: Vec<Command>,
}

impl Editor {
    pub(crate) fn new(instructions: Vec<Instruction>) -> Self {
        Self {
            instructions,
            counter: 0,
            hold: String::new(),
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
                        Copy => {
                            self.hold = buffer.1.to_string();
                        }
                        Paste => {
                            buffer.1 = self.hold.to_string();
                        }
                        Exchange => {
                            let tmp = self.hold.to_string();
                            self.hold = buffer.1.to_string();
                            buffer.1 = tmp;
                        }
                        _ => cmd.apply(&mut buffer),
                    }
                }
                matched = true;
            }
        }

        if matched {
            Some((buffer.1, Nothing))
        } else {
            None
        }
    }
}

impl std::fmt::Display for Instruction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut commands = self
            .commands
            .iter()
            .map(|c| c.to_string())
            .collect::<Vec<String>>()
            .join(" ");
        if let Some(c) = commands.chars().last() {
            if c != '.' {
                commands.push(' ');
                commands.push(';');
            }
        }
        write!(f, "{} {}", self.address, commands,)
    }
}

impl std::fmt::Display for Editor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            self.instructions
                .iter()
                .map(|c| c.to_string())
                .collect::<Vec<String>>()
                .join("\n")
        )
    }
}
