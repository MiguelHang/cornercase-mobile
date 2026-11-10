use serde::{Deserialize, Serialize};

use crate::cli::changes::Status;

#[cfg(test)]
pub(crate) mod fixtures;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize, Serialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "lowercase")]
pub enum EntryKind {
    #[default]
    File,
    Folder,
    #[serde(other)]
    Other,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize, Serialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "kebab-case")]
pub enum ReadKind {
    #[default]
    Text,
    Image,
    Binary,
    TooLarge,
    Folder,
    Missing,
    #[serde(other)]
    Other,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize, Serialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "lowercase")]
pub enum SearchMode {
    #[default]
    Text,
    Names,
    #[serde(other)]
    Other,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize, Serialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(default)]
pub struct Entry {
    pub name: String,
    pub kind: EntryKind,
    pub link: bool,
    pub hidden: bool,
    #[cfg_attr(feature = "specta", specta(type = Option<u32>))]
    pub bytes: Option<u64>,
    pub modified: Option<String>,
    pub status: Option<Status>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize, Serialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(default)]
pub struct Folder {
    pub path: String,
    pub dir: String,
    pub entries: Vec<Entry>,
    pub more: bool,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize, Serialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(default)]
pub struct Listing {
    pub folders: Vec<Folder>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize, Serialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(default)]
pub struct Image {
    pub format: String,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub data: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize, Serialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(default)]
pub struct Read {
    pub path: String,
    pub file: String,
    pub kind: ReadKind,
    #[cfg_attr(feature = "specta", specta(type = Option<u32>))]
    pub bytes: Option<u64>,
    pub modified: Option<String>,
    pub language: Option<String>,
    pub text: Option<String>,
    pub image: Option<Image>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize, Serialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(default)]
pub struct Name {
    pub path: String,
    pub indices: Vec<u32>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize, Serialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(default)]
pub struct FoundLine {
    pub number: u32,
    pub text: String,
    #[cfg_attr(feature = "specta", specta(type = Vec<[u32; 2]>))]
    pub ranges: Vec<[u64; 2]>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize, Serialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(default)]
pub struct FoundFile {
    pub path: String,
    pub lines: Vec<FoundLine>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize, Serialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(default)]
pub struct Found {
    pub query: String,
    pub mode: SearchMode,
    pub names: Vec<Name>,
    pub files: Vec<FoundFile>,
    #[cfg_attr(feature = "specta", specta(type = u32))]
    pub matches: u64,
    pub capped: bool,
}

#[cfg(test)]
mod tests;
