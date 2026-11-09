use tauri::{Runtime, Wry};
use tauri_specta::{Builder, collect_commands, collect_events};

use crate::commands::{connection, identity, machines, probe, routes};
use crate::events::{ConnectionChanged, SessionChanged, Toast};

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
            connection::session,
            identity::identity_fingerprint,
        ])
        .events(collect_events![ConnectionChanged, SessionChanged, Toast])
}

#[cfg(test)]
mod tests;
