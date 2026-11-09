use super::{Machine, MachineId};
use crate::error::Error;

pub(crate) const LONGEST_NAME: usize = 64;

pub(crate) fn checked_name(name: &str, machines: &[Machine], own: Option<MachineId>) -> Result<String, Error> {
    let name = name.trim();
    if name.is_empty() {
        return Err(Error::NameEmpty);
    }
    if name.chars().count() > LONGEST_NAME {
        return Err(Error::NameTooLong);
    }
    let wanted = folded(name);
    if machines.iter().any(|machine| Some(machine.id) != own && folded(&machine.name) == wanted) {
        return Err(Error::NameTaken);
    }
    Ok(name.to_owned())
}

fn folded(name: &str) -> String {
    name.trim().to_lowercase()
}

#[cfg(test)]
mod tests;
