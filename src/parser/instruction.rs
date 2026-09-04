use super::{address, command, reader::Reader, skip_whitespace};
use crate::{Action, address::Address, command::Command};
use anyhow::{Result, bail};

pub(crate) fn parse_instruction<R: Reader>(
    reader: &mut R,
    actions: &mut Vec<Action>,
    finally: &mut Vec<Command>,
) -> Result<()> {
    // [address][commands]
    skip_whitespace(reader);
    let mut address = address::parse(reader)?;
    skip_whitespace(reader);
    let commands = command::parse(reader)?;

    if address.is_final() {
        for cmd in &commands {
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

impl Address {
    fn is_final(&self) -> bool {
        use Address::*;
        match self {
            Final => true,
            Extend(extend) => extend.start.is_final(),
            Set(set) => set.iter().any(|a| a.is_final()),
            _ => false,
        }
    }

    fn is_regular(&self) -> bool {
        use Address::*;
        match self {
            Final => false,
            Extend(extend) => extend.start.is_regular(),
            Set(set) => set.iter().any(|a| a.is_regular()),
            And(and) => !and.iter().any(|a| a.is_final()),
            Negate(not) => !matches!(not.as_ref(), Always),
            _ => true,
        }
    }
}
