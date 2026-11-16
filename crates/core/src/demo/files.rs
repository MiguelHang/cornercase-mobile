use std::collections::BTreeMap;
use std::time::Duration;

use serde::Deserialize;

use super::clock::iso;
use super::dispatch::{self, Answer, json};
use super::world::World;
use crate::cli::changes::Status;
use crate::cli::files::{Entry, EntryKind, Folder, Found, Listing, Name, Read, ReadKind, SearchMode};

const TOUCHED: Duration = Duration::from_mins(5);
const KEPT: Duration = Duration::from_hours(72);

#[derive(Deserialize)]
pub(super) struct Tree {
    pub(super) workspace: u64,
    dir: String,
    files: Vec<Written>,
}

#[derive(Deserialize)]
pub(super) struct Written {
    pub(super) path: String,
    pub(super) lines: Vec<String>,
}

impl Written {
    fn text(&self) -> String {
        self.lines.iter().flat_map(|line| [line.as_str(), "\n"]).collect()
    }
}

impl Tree {
    pub(super) fn written(&self, path: &str) -> Option<&Written> {
        self.files.iter().find(|file| file.path == path)
    }

    fn within<'a>(&'a self, folder: &'a str) -> impl Iterator<Item = (&'a str, &'a Written)> {
        self.files.iter().filter_map(move |file| Some((inside(&file.path, folder)?, file)))
    }
}

impl World {
    fn tree(&self, workspace: u64) -> Option<&Tree> {
        self.trees.iter().find(|tree| tree.workspace == workspace)
    }
}

pub(super) fn answer(world: &World, argv: &[String]) -> Answer {
    let tree = dispatch::number(argv, "--workspace").and_then(|workspace| world.tree(workspace));
    let operands: Vec<&str> = dispatch::operands(argv).collect();
    let shown = match (argv.get(1).map(String::as_str), tree) {
        (Some("list"), Some(tree)) => Some(json(&listing(world, tree, &operands))),
        (Some("read"), Some(tree)) => {
            operands.first().and_then(|path| read(world, tree, relative(path))).map(|read| json(&read))
        }
        (Some("search"), Some(tree)) => operands.first().map(|query| json(&found(tree, query))),
        _ => None,
    };
    shown.map_or_else(|| Answer::unknown(argv), Answer::printed)
}

fn listing(world: &World, tree: &Tree, paths: &[&str]) -> Listing {
    Listing { folders: paths.iter().map(|path| folder(world, tree, relative(path))).collect() }
}

fn folder(world: &World, tree: &Tree, path: &str) -> Folder {
    let mut entries = BTreeMap::new();
    for (rest, file) in tree.within(path) {
        let (name, kind) = rest.split_once('/').map_or((rest, EntryKind::File), |(name, _)| (name, EntryKind::Folder));
        let shown = joined(path, name);
        let entry = match kind {
            EntryKind::Folder => Entry { status: world.touched(tree.workspace, &shown), ..named(name, kind) },
            _ => file_entry(world, tree.workspace, name, file),
        };
        entries.insert((kind != EntryKind::Folder, name.to_owned()), entry);
    }
    let error = (entries.is_empty() && !path.is_empty()).then(|| "no such folder".to_owned());
    let entries = entries.into_values().collect();
    Folder { path: path.to_owned(), dir: joined(&tree.dir, path), entries, more: false, error }
}

fn file_entry(world: &World, workspace: u64, name: &str, file: &Written) -> Entry {
    let status = world.status_of(workspace, &file.path);
    let bytes = u64::try_from(file.text().len()).ok();
    Entry { bytes, modified: Some(modified(world, status)), status, ..named(name, EntryKind::File) }
}

fn named(name: &str, kind: EntryKind) -> Entry {
    Entry { name: name.to_owned(), kind, hidden: name.starts_with('.'), ..Entry::default() }
}

fn read(world: &World, tree: &Tree, path: &str) -> Option<Read> {
    let file = joined(&tree.dir, path);
    let Some(written) = tree.written(path) else {
        let folder = path.is_empty() || tree.within(path).next().is_some();
        return folder.then(|| Read { path: path.to_owned(), file, kind: ReadKind::Folder, ..Read::default() });
    };
    let text = written.text();
    let status = world.status_of(tree.workspace, path);
    Some(Read {
        path: path.to_owned(),
        file,
        kind: ReadKind::Text,
        bytes: u64::try_from(text.len()).ok(),
        modified: Some(modified(world, status)),
        text: Some(text),
        ..Read::default()
    })
}

fn found(tree: &Tree, query: &str) -> Found {
    let wanted = query.to_lowercase();
    let names: Vec<Name> = tree.files.iter().filter_map(|file| matched(&file.path, &wanted)).collect();
    let matches = u64::try_from(names.len()).unwrap_or_default();
    Found { query: query.to_owned(), mode: SearchMode::Names, names, files: Vec::new(), matches, capped: false }
}

fn matched(path: &str, wanted: &str) -> Option<Name> {
    let lower = path.to_lowercase();
    let at = lower.find(wanted)?;
    let start = lower[..at].chars().count();
    let indices = (start..start + wanted.chars().count()).filter_map(|index| u32::try_from(index).ok()).collect();
    Some(Name { path: path.to_owned(), indices })
}

fn modified(world: &World, status: Option<Status>) -> String {
    let ago = if status.is_some() { TOUCHED } else { KEPT };
    iso(world.now().checked_sub(ago).unwrap_or_else(|| world.now()))
}

fn inside<'a>(path: &'a str, folder: &str) -> Option<&'a str> {
    if folder.is_empty() {
        return Some(path);
    }
    path.strip_prefix(folder)?.strip_prefix('/')
}

fn relative(path: &str) -> &str {
    let path = path.strip_prefix("./").unwrap_or(path);
    if path == "." { "" } else { path }
}

fn joined(folder: &str, name: &str) -> String {
    match (folder.is_empty(), name.is_empty()) {
        (true, _) => name.to_owned(),
        (_, true) => folder.to_owned(),
        _ => format!("{folder}/{name}"),
    }
}

#[cfg(test)]
mod tests;
