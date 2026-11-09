use tauri::{Runtime, Wry};
use tauri_specta::{Builder, collect_commands, collect_events};

use crate::commands::{chat, connection, identity, machines, probe, routes, start, terminal};
use crate::events::{AgentsChanged, ConnectionChanged, PaneStatusChanged, TerminalChanged, Toast};

pub(crate) fn specta<R: Runtime>() -> Builder<R> {
    Builder::<R>::new()
        .commands(collect_commands![
            machines::machines_list,
            machines::machine_add,
            machines::machine_remove::<Wry>,
            machines::machine_rename,
            routes::machine_route_add::<Wry>,
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
            terminal::terminal_open::<Wry>,
            terminal::terminal_close,
            start::start_projects,
            start::installed_agents,
            start::start_agent,
            start::new_workspace,
        ])
        .events(collect_events![ConnectionChanged, AgentsChanged, PaneStatusChanged, TerminalChanged, Toast])
}

#[cfg(test)]
mod tests;
