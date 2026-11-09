use tauri::{Runtime, Wry};
use tauri_specta::{Builder, collect_commands, collect_events};

use crate::commands::{connection, identity, machines, probe, routes, start, terminal};
use crate::events::{AgentsChanged, ConnectionChanged, TerminalChanged, Toast};

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
            terminal::terminal_open::<Wry>,
            terminal::terminal_close,
            start::start_projects,
            start::installed_agents,
        ])
        .events(collect_events![ConnectionChanged, AgentsChanged, TerminalChanged, Toast])
}

#[cfg(test)]
mod tests;
