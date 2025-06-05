use super::{address, command, reader::Reader, utils};
use crate::{address::Address, command::Command, Action};
use anyhow::{bail, Result};

pub(crate) fn parse_instruction<R: Reader>(
    reader: &mut R,
    actions: &mut Vec<Action>,
    finally: &mut Vec<Command>,
) -> Result<()> {
    // [address][commands]
    utils::skip_whitespace(reader);
    let mut address = address::parse(reader)?;
    utils::skip_whitespace(reader);
    let commands = command::parse(reader)?;

    if address == Address::Final {
        for cmd in commands.into_iter() {
            finally.push(cmd);
        }
    } else {
        address.replace_maybe(commands.first())?;
        actions.push(Action::Condition(address, commands.len()));
        for cmd in commands.into_iter() {
            actions.push(Action::Command(cmd));
        }
    }
    Ok(())
}

impl Address {
    fn replace_maybe(&mut self, subst: Option<&Command>) -> Result<()> {
        match self {
            Address::Maybe => {
                let Some(Command::Substitute(regex, _, _)) = subst else {
                    bail!("_ must be followed by a substitution")
                };
                *self = Address::Regex(regex.clone());
            }
            Address::Between(between) => {
                between.lhs.replace_maybe(subst)?;
                between.rhs.replace_maybe(subst)?;
            }
            Address::Set(addrs) => addrs.iter_mut().try_for_each(|a| a.replace_maybe(subst))?,
            _ => (),
        }
        Ok(())
    }
}
