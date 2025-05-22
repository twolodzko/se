use std::collections::HashMap;

use crate::{
    address::Address,
    command::{Command, Status},
    Line,
};

#[derive(Debug, PartialEq)]
pub struct Editor {
    pub(crate) instructions: Vec<Instruction>,
    pub(crate) counter: usize,
    hold: String,
    pub(crate) labels: Labels,
}

type Labels = HashMap<String, usize>;

#[derive(Debug, PartialEq)]
pub(crate) struct Instruction {
    pub(crate) address: Address,
    pub(crate) commands: Vec<Command>,
}

impl Editor {
    pub(crate) fn new(instructions: Vec<Instruction>, labels: Labels) -> Self {
        Self {
            instructions,
            counter: 0,
            hold: String::new(),
            labels,
        }
    }

    pub fn process(&mut self, line: &str) -> Option<(String, Status)> {
        use Status::*;

        self.counter += 1;
        let mut matched = false;
        let mut pattern = Line(self.counter, line.to_string());
        let mut print = String::new();
        let mut i = 0;

        'it: while i < self.instructions.len() {
            unsafe {
                let instruction = self.instructions.get_unchecked_mut(i);
                if instruction.address.matches(&pattern) {
                    for cmd in instruction.commands.iter() {
                        let status = cmd.run(&mut pattern, &mut self.hold, &mut print);
                        if let GoTo(label) = status {
                            match self.labels.get(&label) {
                                Some(index) => {
                                    i = *index;
                                    print!("{}", print);
                                    print.clear();
                                    continue 'it;
                                }
                                None => unimplemented!(),
                            }
                        } else if status != Normal {
                            print!("{}", print);
                            return Some((pattern.1, status));
                        }
                    }
                    matched = true;
                }
            }
            i += 1;
        }
        print!("{}", print);

        if matched {
            Some((pattern.1, Normal))
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

#[cfg(test)]
mod tests {
    use crate::Editor;
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
        "12345";
        "first n chars"
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
        let mut editor = Editor::try_from(command.to_string()).unwrap();
        let (result, _) = editor.process("123456789").unwrap();
        assert_eq!(result, expected)
    }
}
