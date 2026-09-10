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
            if matches!(cmd, Command::Branch(_, _) | Command::Label(_)) {
                return error!("branching is not supported the final block");
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
        address.simplify()?;
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
    fn is_final(&self) -> bool {
        use Address::*;
        match self {
            Final => true,
            Extend(extend) => extend.start.is_final(),
            Set(set) => set.iter().any(|a| a.is_final()),
            Between(between) => between.start.is_final(),
            _ => false,
        }
    }

    fn is_regular(&self) -> bool {
        if let Address::Set(set) = self {
            return set.iter().any(|a| a.is_regular());
        }
        !self.is_final()
    }

    pub(super) fn is_impossible(&self) -> bool {
        use Address::*;
        match self {
            And(and) => and.iter().any(|a| a.is_final() || a.is_impossible()),
            Negate(not) => not.is_final(),
            Extend(extend) => extend.start.is_impossible(),
            Between(between) => between.start.is_impossible(),
            _ => false,
        }
    }

    fn replace_maybe(&mut self, subst: Option<&crate::Regex>) -> Result<()> {
        use Address::*;
        match self {
            Maybe => {
                let Some(regex) = subst else {
                    return error!("{} must be followed by a substitution", self);
                };
                *self = Regex(regex.clone());
            }
            Between(between) => {
                between.start.replace_maybe(subst)?;
                between.end.replace_maybe(subst)?;
            }
            Set(set) => set.iter_mut().try_for_each(|a| a.replace_maybe(subst))?,
            And(and) => and.iter_mut().try_for_each(|a| a.replace_maybe(subst))?,
            Extend(extend) => extend.start.replace_maybe(subst)?,
            _ => {}
        }
        Ok(())
    }

    /// Simplify addresses that never match
    fn simplify(&mut self) -> Result<()> {
        use Address::*;
        match self {
            Final => *self = Never,
            Set(set) => {
                let mut i = 0;
                while i < set.len() {
                    set[i].simplify()?;
                    if set[i] == Never {
                        set.remove(i);
                    } else {
                        i += 1;
                    }
                }
                match set.len() {
                    0 => *self = Never,
                    1 => *self = set.remove(0),
                    _ => {}
                }
            }
            And(and) => {
                for addr in and.iter_mut() {
                    addr.simplify()?;
                    if *addr == Never {
                        *self = Never;
                        break;
                    }
                }
            }
            Extend(extend) => {
                extend.start.simplify()?;
                if *extend.start == Never {
                    *self = Never
                }
            }
            Between(between) => {
                between.start.simplify()?;
                if *between.start == Never {
                    *self = Never
                }
            }
            _ => {}
        }
        Ok(())
    }
}
