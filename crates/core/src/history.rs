use serde::{Deserialize, Serialize};

use crate::error::Error;
use crate::machines::store::versioned;

const VERSION: u32 = 1;
const MOST_ENTRIES: usize = 200;
const MOST_BYTES: usize = 1 << 20;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum HistoryEntry {
    You {
        #[cfg_attr(feature = "specta", specta(type = u32))]
        id: u64,
        text: String,
        chips: Vec<String>,
        state: SendState,
        reason: Option<String>,
        #[cfg_attr(feature = "specta", specta(type = f64))]
        seen: u64,
    },
    Agent {
        #[cfg_attr(feature = "specta", specta(type = u32))]
        id: u64,
        text: String,
        written: Option<String>,
        turn_over: Option<bool>,
        #[cfg_attr(feature = "specta", specta(type = f64))]
        seen: u64,
    },
    Gap {
        #[cfg_attr(feature = "specta", specta(type = u32))]
        id: u64,
        #[cfg_attr(feature = "specta", specta(type = f64))]
        seen: u64,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "snake_case")]
pub enum SendState {
    Sending,
    Sent,
    Failed,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PaneFile {
    pub boot: String,
    pub entries: Vec<HistoryEntry>,
}

#[derive(Serialize)]
struct Written<'a> {
    version: u32,
    boot: &'a str,
    entries: &'a [HistoryEntry],
}

#[derive(Deserialize)]
struct Read {
    boot: String,
    entries: Vec<HistoryEntry>,
}

impl HistoryEntry {
    fn gap_after(&self) -> Self {
        let (Self::You { id, seen, .. } | Self::Agent { id, seen, .. } | Self::Gap { id, seen }) = self;
        Self::Gap { id: *id, seen: *seen }
    }
}

pub fn written(boot: &str, entries: &[HistoryEntry]) -> Result<Vec<u8>, Error> {
    let envelope = serialized(&Written { version: VERSION, boot, entries: &[] })?.len();
    let kept = capped(entries, MOST_BYTES.saturating_sub(envelope))?;
    serialized(&Written { version: VERSION, boot, entries: &kept })
}

pub fn parsed(text: &[u8]) -> Result<PaneFile, Error> {
    let Read { boot, entries } = versioned(text, VERSION)?;
    Ok(PaneFile { boot, entries })
}

fn capped(entries: &[HistoryEntry], budget: usize) -> Result<Vec<HistoryEntry>, Error> {
    let sizes = entries.iter().map(size).collect::<Result<Vec<_>, _>>()?;
    let gap = size(&HistoryEntry::Gap { id: u64::MAX, seen: u64::MAX })?;
    if fits(entries.len(), sizes.iter().sum(), budget) {
        return Ok(entries.to_vec());
    }
    let start = (1..entries.len())
        .find(|&start| {
            let gap_needed = needs_gap(&entries[start..]);
            let count = entries.len() - start + usize::from(gap_needed);
            let bytes = sizes[start..].iter().sum::<usize>() + if gap_needed { gap } else { 0 };
            fits(count, bytes, budget)
        })
        .unwrap_or(entries.len());
    Ok(from(entries, start))
}

fn from(entries: &[HistoryEntry], start: usize) -> Vec<HistoryEntry> {
    let rest = &entries[start..];
    if needs_gap(rest) {
        std::iter::once(entries[start - 1].gap_after()).chain(rest.iter().cloned()).collect()
    } else {
        rest.to_vec()
    }
}

fn needs_gap(rest: &[HistoryEntry]) -> bool {
    !matches!(rest.first(), Some(HistoryEntry::Gap { .. }))
}

fn fits(count: usize, bytes: usize, budget: usize) -> bool {
    count <= MOST_ENTRIES && bytes <= budget
}

fn size(entry: &HistoryEntry) -> Result<usize, Error> {
    Ok(serialized(entry)?.len() + 1)
}

fn serialized(value: &impl Serialize) -> Result<Vec<u8>, Error> {
    serde_json::to_vec(value).map_err(|error| Error::Store(format!("cannot write the history: {error}")))
}

#[cfg(test)]
mod tests;
