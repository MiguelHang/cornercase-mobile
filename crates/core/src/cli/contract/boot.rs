use super::Descriptor;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Boot(pub String);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Observed {
    Same,
    Restarted,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct BootWatch {
    current: Option<Boot>,
}

impl Descriptor {
    pub fn boot(&self) -> Boot {
        Boot(self.boot.clone())
    }
}

impl BootWatch {
    pub fn observe(&mut self, boot: Boot) -> Observed {
        let restarted = self.current.as_ref().is_some_and(|current| *current != boot);
        self.current = Some(boot);
        if restarted { Observed::Restarted } else { Observed::Same }
    }
}

#[cfg(test)]
mod tests;
