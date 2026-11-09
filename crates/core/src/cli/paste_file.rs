use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::error::Error;

pub const MAX_IMAGE_BYTES: usize = 32 << 20;

const FALLBACK_STEM: &str = "pasted";

#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize, Serialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(default)]
pub struct Pasted {
    #[cfg_attr(feature = "specta", specta(type = u32))]
    pub pane: u64,
    pub path: String,
    #[cfg_attr(feature = "specta", specta(type = u32))]
    pub bytes: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Kind {
    Png,
    Jpeg,
}

impl Kind {
    fn extension(self) -> &'static str {
        match self {
            Self::Png => "png",
            Self::Jpeg => "jpg",
        }
    }
}

pub fn args(pane: u64, name: &str) -> Vec<String> {
    ["paste-file", "--pane", &pane.to_string(), &format!("--name={name}"), "--json", "-"].map(str::to_owned).into()
}

pub fn checked(bytes: Vec<u8>) -> Result<Vec<u8>, Error> {
    if bytes.is_empty() {
        return Err(Error::ImageEmpty);
    }
    if bytes.len() > MAX_IMAGE_BYTES {
        return Err(Error::ImageTooLarge);
    }
    Ok(bytes)
}

pub fn output_name(original: Option<&str>, kind: Kind) -> String {
    let stem = original
        .and_then(|name| Path::new(name.trim()).file_stem())
        .and_then(|stem| stem.to_str())
        .filter(|stem| !stem.is_empty())
        .unwrap_or(FALLBACK_STEM);
    format!("{stem}.{}", kind.extension())
}

#[cfg(test)]
mod tests;
