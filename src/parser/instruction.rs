use super::{address, command, reader::Reader, utils};
use crate::{Action, address::Address, command::Command};
use anyhow::{Result, bail};

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
        finally.extend(commands);
    } else {
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
                    bail!("{} must be followed by a substitution", self)
                };
                *self = Address::Regex(regex.clone());
            }
            Address::Between(between) => {
                between.lhs.replace_maybe(subst)?;
                between.rhs.replace_maybe(subst)?;
            }
            Address::Set(addrs) => addrs.iter_mut().try_for_each(|a| a.replace_maybe(subst))?,
            Address::And(addrs) => addrs.iter_mut().try_for_each(|a| a.replace_maybe(subst))?,
            _ => (),
        }
        Ok(())
    }
}
