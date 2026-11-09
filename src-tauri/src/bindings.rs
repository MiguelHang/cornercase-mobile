use tauri::{Runtime, Wry};
use tauri_specta::{Builder, collect_commands, collect_events};

use crate::commands::{connection, identity, machines};
use crate::events::{AgentsChanged, ConnectionChanged, Toast};

pub(crate) fn specta<R: Runtime>() -> Builder<R> {
    Builder::<R>::new()
        .commands(collect_commands![
            machines::machines_list,
            machines::machine_add,
            machines::machine_remove::<Wry>,
            connection::connect::<Wry>,
            connection::connection_state,
            connection::agents,
            identity::identity_fingerprint,
        ])
        .events(collect_events![ConnectionChanged, AgentsChanged, Toast])
}

#[cfg(test)]
mod tests;
