use serde::Deserialize;
use serde_json::json;

use super::dispatch::{self, Answer};
use super::files::Tree;
use super::world::World;
use crate::cli::changes::{ChangedFile, ChangedWorkspace, Hunk, Line, LineKind, Mode, Patch, State, Status, Summary};

const STREAM: u64 = 1;
const SCOPE: &str = "workspace";

#[derive(Deserialize)]
pub(super) struct Changed {
    project: u64,
    workspace: u64,
    base: String,
    files: Vec<Diffed>,
}

#[derive(Deserialize)]
struct Diffed {
    path: String,
    status: Status,
    #[serde(default)]
    hunks: Vec<Hunked>,
}

#[derive(Default, Deserialize)]
struct Hunked {
    old_start: u64,
    new_start: u64,
    #[serde(default)]
    context: String,
    lines: Vec<String>,
}

pub(super) struct Diffs {
    project: u64,
    workspace: u64,
    base: String,
    patches: Vec<Patch>,
}

pub(super) fn diffs(changed: Vec<Changed>, trees: &[Tree]) -> Vec<Diffs> {
    changed
        .into_iter()
        .map(|changed| {
            let tree = trees.iter().find(|tree| tree.workspace == changed.workspace);
            let patches =
                changed.files.into_iter().enumerate().map(|(at, diffed)| patch(changed.workspace, at, diffed, tree));
            Diffs {
                project: changed.project,
                workspace: changed.workspace,
                base: changed.base,
                patches: patches.collect(),
            }
        })
        .collect()
}

fn patch(workspace: u64, at: usize, diffed: Diffed, tree: Option<&Tree>) -> Patch {
    let hunked = if diffed.hunks.is_empty() { whole(tree, &diffed.path) } else { diffed.hunks };
    let hunks: Vec<Hunk> = hunked.into_iter().map(hunk).collect();
    let counted =
        |kind| u64::try_from(hunks.iter().flat_map(|hunk| &hunk.lines).filter(|line| line.kind == kind).count());
    let file = ChangedFile {
        path: diffed.path,
        status: diffed.status,
        added: counted(LineKind::Added).unwrap_or_default(),
        removed: counted(LineKind::Removed).unwrap_or_default(),
        digest: format!("{workspace:08x}{at:08x}"),
        ..ChangedFile::default()
    };
    Patch { workspace, mode: Mode::default(), file, hunks }
}

fn whole(tree: Option<&Tree>, path: &str) -> Vec<Hunked> {
    let lines = tree.and_then(|tree| tree.written(path)).map(|written| &written.lines).into_iter().flatten();
    vec![Hunked { new_start: 1, lines: lines.map(|line| format!("+{line}")).collect(), ..Hunked::default() }]
}

fn hunk(hunked: Hunked) -> Hunk {
    let (mut old, mut new) = (hunked.old_start, hunked.new_start);
    let lines = hunked
        .lines
        .iter()
        .map(|marked| {
            let (kind, text) = split(marked);
            let line = Line {
                kind,
                old: (kind != LineKind::Added).then_some(old),
                new: (kind != LineKind::Removed).then_some(new),
                text: text.to_owned(),
                ..Line::default()
            };
            old += u64::from(kind != LineKind::Added);
            new += u64::from(kind != LineKind::Removed);
            line
        })
        .collect();
    Hunk { old_start: hunked.old_start, new_start: hunked.new_start, context: hunked.context, lines }
}

fn split(marked: &str) -> (LineKind, &str) {
    let mut chars = marked.chars();
    let kind = match chars.next() {
        Some('-') => LineKind::Removed,
        Some('+') => LineKind::Added,
        _ => LineKind::Context,
    };
    (kind, chars.as_str())
}

pub(super) struct Listed {
    workspace: u64,
    mode: Mode,
    revision: u64,
    shown: Option<Vec<ChangedWorkspace>>,
}

impl Listed {
    pub(super) fn new(workspace: u64, mode: Mode) -> Self {
        Self { workspace, mode, revision: 0, shown: None }
    }

    pub(super) fn fresh(&mut self, world: &World) -> Vec<String> {
        let workspaces: Vec<ChangedWorkspace> = world.summed(self.workspace).into_iter().collect();
        if self.shown.as_ref() == Some(&workspaces) {
            return Vec::new();
        }
        self.revision += 1;
        let summary = Summary {
            boot: world.session.boot.clone(),
            stream: STREAM,
            revision: self.revision,
            scope: SCOPE.to_owned(),
            mode: self.mode,
            state: State::Ready,
            error: None,
            workspaces: workspaces.clone(),
        };
        self.shown = Some(workspaces);
        vec![dispatch::json(&summary)]
    }
}

pub(super) fn answer(world: &mut World, argv: &[String]) -> Answer {
    let place = dispatch::number(argv, "--workspace").zip(dispatch::operands(argv).next());
    let shown = match (argv.get(1).map(String::as_str), place) {
        (Some("file"), Some((workspace, path))) => {
            world.patch(workspace, path).map(|patch| dispatch::json(&Patch { mode: mode(argv), ..patch }))
        }
        (Some("viewed"), Some((workspace, path))) => world.marked(workspace, path, !dispatch::flagged(argv, "--clear")),
        _ => None,
    };
    shown.map_or_else(|| Answer::unknown(argv), Answer::printed)
}

pub(super) fn mode(argv: &[String]) -> Mode {
    dispatch::valued(argv, "--mode").and_then(|word| serde_json::from_value(json!(word)).ok()).unwrap_or_default()
}

impl World {
    pub(super) fn status_of(&self, workspace: u64, path: &str) -> Option<Status> {
        self.found(workspace, path).map(|found| found.file.status)
    }

    pub(super) fn touched(&self, workspace: u64, folder: &str) -> Option<Status> {
        let prefix = format!("{folder}/");
        self.patches(workspace).any(|patch| patch.file.path.starts_with(&prefix)).then_some(Status::Modified)
    }

    fn summed(&self, workspace: u64) -> Option<ChangedWorkspace> {
        let diffs = self.diffs.iter().find(|diffs| diffs.workspace == workspace)?;
        let files: Vec<ChangedFile> =
            self.patches(workspace).map(|patch| self.viewed(workspace, &patch.file)).collect();
        Some(ChangedWorkspace {
            project: diffs.project,
            workspace,
            base: Some(diffs.base.clone()),
            added: files.iter().map(|file| file.added).sum(),
            removed: files.iter().map(|file| file.removed).sum(),
            files,
        })
    }

    fn patch(&self, workspace: u64, path: &str) -> Option<Patch> {
        let found = self.found(workspace, path)?;
        Some(Patch { file: self.viewed(workspace, &found.file), ..found.clone() })
    }

    fn found(&self, workspace: u64, path: &str) -> Option<&Patch> {
        self.patches(workspace).find(|found| found.file.path == path)
    }

    fn marked(&mut self, workspace: u64, path: &str, viewed: bool) -> Option<String> {
        self.status_of(workspace, path)?;
        let key = (workspace, path.to_owned());
        if viewed {
            self.seen.insert(key);
        } else {
            self.seen.remove(&key);
        }
        self.told.send_replace(self.session.revision);
        Some(String::new())
    }

    fn viewed(&self, workspace: u64, file: &ChangedFile) -> ChangedFile {
        ChangedFile { viewed: self.seen.contains(&(workspace, file.path.clone())), ..file.clone() }
    }

    fn patches(&self, workspace: u64) -> impl Iterator<Item = &Patch> {
        self.diffs.iter().filter(move |diffs| diffs.workspace == workspace).flat_map(|diffs| &diffs.patches)
    }
}

#[cfg(test)]
mod tests;
