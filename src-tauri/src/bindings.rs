use tauri::{Runtime, Wry};
use tauri_specta::{Builder, collect_commands, collect_events};

use crate::commands::{changes, chat, connection, identity, machines, probe, routes, start, terminal};
use crate::events::{AgentsChanged, ChangesChanged, ConnectionChanged, PaneStatusChanged, TerminalChanged, Toast};

pub(crate) fn specta<R: Runtime>() -> Builder<R> {
    with_events(Builder::<R>::new().commands(collect_commands![
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
        chat::attach::attach::<Wry>,
        terminal::terminal_open::<Wry>,
        terminal::terminal_close,
        terminal::input::terminal_input,
        terminal::input::terminal_scroll,
        start::start_projects,
        start::installed_agents,
        start::start_agent,
        start::new_workspace,
        changes::changes_file,
        changes::changes_viewed,
        changes::changes_follow::<Wry>,
        changes::changes_unfollow,
    ]))
}

fn with_events<R: Runtime>(builder: Builder<R>) -> Builder<R> {
    builder.events(collect_events![
        ConnectionChanged,
        AgentsChanged,
        PaneStatusChanged,
        TerminalChanged,
        ChangesChanged,
        Toast
    ])
}

#[cfg(test)]
mod tests;
