use std::future;
use std::time::Duration;

use serde::Deserialize;
use tokio::time::{self, Instant};

const QUIET: Duration = Duration::from_secs(1);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Kind {
    Wifi,
    Cellular,
    Other,
    None,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
pub struct Network {
    pub kind: Kind,
    pub expensive: bool,
}

impl Network {
    pub fn is_down(self) -> bool {
        self.kind == Kind::None
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Decision {
    Baseline(Network),
    Offline,
    Changed(Network),
}

#[derive(Debug)]
pub struct Debounce {
    quiet: Duration,
    applied: Option<Network>,
    pending: Option<(Network, Instant)>,
}

impl Default for Debounce {
    fn default() -> Self {
        Self::new(QUIET)
    }
}

impl Debounce {
    pub fn new(quiet: Duration) -> Self {
        Self { quiet, applied: None, pending: None }
    }

    pub fn observe(&mut self, network: Network) {
        self.pending = Some((network, Instant::now() + self.quiet));
    }

    pub async fn decided(&mut self) -> Decision {
        loop {
            let Some((_, calm)) = self.pending else {
                return future::pending().await;
            };
            time::sleep_until(calm).await;
            if let Some(decision) = self.settle() {
                return decision;
            }
        }
    }

    fn settle(&mut self) -> Option<Decision> {
        let (network, _) = self.pending.take()?;
        let previous = self.applied.replace(network);
        match previous {
            Some(previous) if previous.is_down() && network.is_down() => None,
            _ if network.is_down() => Some(Decision::Offline),
            None => Some(Decision::Baseline(network)),
            Some(_) => Some(Decision::Changed(network)),
        }
    }
}

#[cfg(test)]
mod tests;
