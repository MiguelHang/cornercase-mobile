use tauri::{Runtime, Wry};
use tauri_specta::{Builder, Commands, collect_commands, collect_events};

use crate::commands::{changes, chat, connection, files, identity, issues, machines, probe, routes, start, terminal};
use crate::events::{
    AgentsChanged, ChangesChanged, ConnectionChanged, PaneStatusChanged, QuestionChanged, TerminalChanged, Toast,
};

pub(crate) fn specta<R: Runtime>() -> Builder<R> {
    Builder::<R>::new().commands(commands()).events(collect_events![
        ConnectionChanged,
        AgentsChanged,
        PaneStatusChanged,
        TerminalChanged,
        QuestionChanged,
        ChangesChanged,
        Toast
    ])
}

macro_rules! app_commands {
    () => {
        collect_commands![
            machines::machines_list,
            machines::machine_add,
            machines::machine_remove::<Wry>,
            machines::machine_rename,
            routes::machine_merge::<Wry>,
            routes::machine_route_remove::<Wry>,
            routes::machine_route_move,
            probe::machine_probe,
            probe::machine_match,
            connection::connect::<Wry>,
            connection::connection_state,
            connection::agents,
            identity::identity_fingerprint,
            chat::last_message,
            chat::send,
            chat::keys,
            chat::mark_reviewed,
            chat::attach::attach::<Wry>,
            chat::question::question_open::<Wry>,
            chat::question::question_close,
            terminal::terminal_open::<Wry>,
            terminal::terminal_close,
            terminal::input::terminal_input,
            terminal::input::terminal_scroll,
            start::start_projects,
            start::installed_agents,
            start::start_agent,
            start::start_issue,
            start::new_workspace,
            changes::changes_file,
            changes::changes_viewed,
            changes::changes_follow::<Wry>,
            changes::changes_unfollow,
            files::files_list,
            files::files_read,
            files::files_search,
            files::files_search_stop,
            issues::issue_sources,
            issues::issue_list,
            issues::issue_read,
        ]
    };
}

fn commands<R: Runtime>() -> Commands<R> {
    app_commands!()
}

#[cfg(test)]
mod tests;
