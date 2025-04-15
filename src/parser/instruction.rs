use super::{address, command, reader::Reader, skip_whitespace};
use crate::{Action, Error, Result, address::Address, command::Command, error};
use std::collections::HashMap;

pub(crate) fn parse_instruction<R: Reader>(
    reader: &mut R,
    actions: &mut Vec<Action>,
    finally: &mut Vec<Command>,
    lables: &mut HashMap<String, usize>,
) -> Result<()> {
    // [address][commands]
    skip_whitespace(reader);
    let mut address = address::parse(reader)?;
    skip_whitespace(reader);
    let commands = command::parse(reader)?;

    if address.is_final() {
        for cmd in &commands {
            if !matches!(address, Address::Set(_))
                && let Command::Label(label) = cmd
            {
                return error!("label {} declared in the final block", label);
            }
            finally.push(cmd.clone());
        }
    }

    if address.is_regular() {
        let subst = commands.iter().find_map(|c| {
            if let Command::Substitute(regex, _, _) = c {
                Some(regex)
            } else {
                None
            }
        });
        address.replace_maybe(subst)?;
        actions.push(Action::Condition(address, commands.len()));

        for cmd in commands.into_iter() {
            if let Command::Label(label) = &cmd
                && lables.insert(label.to_owned(), actions.len()).is_some()
            {
                return error!("duplicated label: {}", label);
            }
            actions.push(Action::Command(cmd));
        }
    }
    Ok(())
}

impl Address {
    fn replace_maybe(&mut self, subst: Option<&crate::Regex>) -> Result<()> {
        match self {
            Address::Maybe => {
                let Some(regex) = subst else {
                    return error!("{} must be followed by a substitution", self);
                };
                *self = Address::Regex(regex.clone());
            }
            Address::Between(between) => {
                between.start.replace_maybe(subst)?;
                between.end.replace_maybe(subst)?;
            }
            Address::Set(set) => set.iter_mut().try_for_each(|a| a.replace_maybe(subst))?,
            Address::And(and) => and.iter_mut().try_for_each(|a| a.replace_maybe(subst))?,
            _ => (),
        }
        Ok(())
    }
}
