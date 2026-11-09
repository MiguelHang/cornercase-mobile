use tauri::Wry;
use tauri_specta::{Builder, collect_commands, collect_events};

use crate::commands::{connection, identity, machines};
use crate::events::{ConnectionChanged, SessionChanged, Toast};

pub(crate) fn specta() -> Builder<Wry> {
    Builder::<Wry>::new()
        .commands(collect_commands![
            machines::machines_list,
            machines::machine_add,
            machines::machine_remove,
            connection::connect,
            connection::connection_state,
            connection::session,
            identity::identity_fingerprint,
        ])
        .events(collect_events![ConnectionChanged, SessionChanged, Toast])
        .dangerously_cast_bigints_to_number()
}

#[cfg(test)]
mod tests;
