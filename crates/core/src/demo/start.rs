use super::chat::{DONE, WORKING};
use super::data;
use super::dispatch::{Answer, flagged, json, number};
use super::surface::{Tone, row};
use super::world::World;
use crate::cli::start::Started;
use crate::cli::status::{Pane, Tab, Workspace};

struct Opened<'a> {
    project: u64,
    workspace: u64,
    agent: &'a str,
    title: &'a str,
}

pub(super) fn accounts(argv: &[String]) -> Answer {
    if argv.get(1).map(String::as_str) == Some("list") && flagged(argv, "--json") {
        Answer::printed(data::accounts().to_string())
    } else {
        Answer::unknown(argv)
    }
}

pub(super) fn answer(world: &mut World, argv: &[String]) -> Answer {
    let agent = argv.get(1).filter(|_| flagged(argv, "--json")).map(String::as_str);
    let placed = agent.and_then(|agent| Some((agent, world.placed(argv)?)));
    let Some((agent, (project, workspace))) = placed else { return Answer::unknown(argv) };
    let title = given(argv, "--prompt=").or_else(|| given(argv, "--issue=")).unwrap_or(agent);
    let (tab, pane) = world.opened(&Opened { project, workspace, agent, title });
    Answer::printed(json(&Started { project, workspace, tab, pane, warning: None }))
}

fn given<'a>(argv: &'a [String], name: &str) -> Option<&'a str> {
    argv.iter().find_map(|word| word.strip_prefix(name)).filter(|value| !value.is_empty())
}

impl World {
    fn placed(&mut self, argv: &[String]) -> Option<(u64, u64)> {
        if let Some(workspace) = number(argv, "--workspace") {
            let mut projects = self.session.projects.iter();
            let project = projects.find(|project| project.workspaces.iter().any(|seen| seen.id == workspace))?;
            return Some((project.id, workspace));
        }
        let project = number(argv, "--project")?;
        let workspace = match given(argv, "--worktree=") {
            Some(branch) => self.worktree(project, branch),
            None => {
                self.session.projects.iter().find(|seen| seen.id == project)?.workspaces.first().map(|seen| seen.id)
            }
        };
        Some((project, workspace?))
    }

    fn worktree(&mut self, project: u64, branch: &str) -> Option<u64> {
        let id = self.issue();
        let project = self.session.projects.iter_mut().find(|seen| seen.id == project)?;
        let path = project.path.as_ref().map(|path| format!("{path}-{branch}"));
        let branch = Some(branch.to_owned());
        let name = branch.clone().unwrap_or_default();
        project.workspaces.push(Workspace { id, name, path, branch, worktree: true, ..Workspace::default() });
        Some(id)
    }

    fn opened(&mut self, opened: &Opened) -> (u64, u64) {
        let (tab, pane) = (self.issue(), self.issue());
        let projects = self.session.projects.iter_mut().filter(|project| project.id == opened.project);
        let workspace = projects
            .flat_map(|project| project.workspaces.iter_mut())
            .find(|workspace| workspace.id == opened.workspace)
            .expect("the agent starts in a workspace the demo has");
        let (agent, status) = (Some(opened.agent.to_owned()), Some(WORKING.to_owned()));
        let started = Pane {
            id: pane,
            agent: agent.clone(),
            status: status.clone(),
            program: agent,
            title: Some(opened.title.to_owned()),
            path: workspace.path.clone(),
            active: true,
            ..Pane::default()
        };
        let name = opened.agent.to_owned();
        workspace.tabs.push(Tab { id: tab, name, status, panes: vec![started], ..Tab::default() });
        self.printed.insert(pane, vec![row(Tone::Accent, format!("> {}", opened.title))]);
        self.worked(pane, DONE);
        (tab, pane)
    }
}

#[cfg(test)]
mod tests;
