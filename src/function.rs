use std::{cell::RefCell, collections::HashMap, rc::Rc};

use crate::{
    address::Address,
    command::{Command, Status},
    Line,
};

#[derive(Debug, PartialEq)]
pub struct Function(pub(crate) Vec<Instruction>, pub(crate) Library);

pub(crate) type Library = Rc<RefCell<HashMap<String, Function>>>;

#[derive(Debug, PartialEq)]
pub(crate) struct Instruction {
    pub(crate) address: Address,
    pub(crate) commands: Vec<Command>,
}

impl Function {
    /// Call the function with `pattern` buffer and `hold` buffer as arguments,
    /// modify them if relevant, return the status. On no match, return `None`.
    pub(crate) fn call(&self, pattern: &mut Line, hold: &mut String) -> Option<Status> {
        let mut matched = false;
        for instruction in self.0.iter() {
            if instruction.address.matches(pattern) {
                for cmd in instruction.commands.iter() {
                    if let Command::Call(name) = cmd {
                        if let Some(func) = self.1.borrow().get(name) {
                            if let Some(status) = func.call(pattern, hold) {
                                if status != Status::Normal {
                                    return Some(status);
                                }
                            }
                        }
                    } else {
                        let status = cmd.run(pattern, hold);
                        if status != Status::Normal {
                            return Some(status);
                        }
                    }
                }
                matched = true;
            }
        }
        if matched {
            Some(Status::Normal)
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

impl std::fmt::Display for Function {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            self.0
                .iter()
                .map(|c| c.to_string())
                .collect::<Vec<String>>()
                .join("\n")
        )
    }
}
