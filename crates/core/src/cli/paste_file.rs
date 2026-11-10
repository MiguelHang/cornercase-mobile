use std::path::Path;

use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use serde::{Deserialize, Serialize};

use crate::error::Error;

pub const MAX_IMAGE_BYTES: usize = 32 << 20;

const FALLBACK_STEM: &str = "pasted";

const IMAGE_EXTENSIONS: [&str; 9] = ["png", "jpg", "jpeg", "heic", "heif", "gif", "webp", "tif", "tiff"];

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

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct Picked {
    pub path: String,
    #[serde(default)]
    pub name: Option<String>,
    pub kind: Kind,
    #[serde(default, rename = "thumbnailPath")]
    pub thumbnail_path: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(untagged)]
pub enum Picking {
    Picked(Picked),
    Cancelled { cancelled: bool },
}

impl Picking {
    pub fn picked(self) -> Option<Picked> {
        match self {
            Self::Picked(picked) => Some(picked),
            Self::Cancelled { .. } => None,
        }
    }
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

pub fn thumbnail_url(bytes: &[u8]) -> String {
    format!("data:image/jpeg;base64,{}", STANDARD.encode(bytes))
}

pub fn output_name(original: Option<&str>, kind: Kind) -> String {
    let stem = original
        .and_then(|name| Path::new(name.trim()).file_name())
        .and_then(|name| name.to_str())
        .map(without_image_extension)
        .filter(|stem| !stem.is_empty())
        .unwrap_or(FALLBACK_STEM);
    format!("{stem}.{}", kind.extension())
}

fn without_image_extension(name: &str) -> &str {
    match name.rsplit_once('.') {
        Some((stem, extension)) if IMAGE_EXTENSIONS.iter().any(|known| known.eq_ignore_ascii_case(extension)) => stem,
        _ => name,
    }
}

#[cfg(test)]
mod tests;
