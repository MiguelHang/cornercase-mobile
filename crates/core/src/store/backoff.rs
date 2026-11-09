use std::time::Duration;

const WAITS: [Duration; 5] = [
    Duration::from_secs(1),
    Duration::from_secs(2),
    Duration::from_secs(4),
    Duration::from_secs(8),
    Duration::from_secs(15),
];

#[derive(Debug, Default)]
pub(crate) struct Backoff {
    failures: usize,
}

impl Backoff {
    pub(crate) fn failed(&mut self) -> Duration {
        let wait = WAITS[self.failures];
        self.failures = (self.failures + 1).min(WAITS.len() - 1);
        wait
    }
}

#[cfg(test)]
mod tests;
