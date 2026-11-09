use tauri::{Runtime, Wry};
use tauri_specta::{Builder, collect_commands, collect_events};

use crate::commands::{connection, identity, machines, terminal};
use crate::events::{ConnectionChanged, SessionChanged, TerminalChanged, Toast};

pub(crate) fn specta<R: Runtime>() -> Builder<R> {
    Builder::<R>::new()
        .commands(collect_commands![
            machines::machines_list,
            machines::machine_add,
            machines::machine_remove::<Wry>,
            connection::connect::<Wry>,
            connection::connection_state,
            connection::session,
            identity::identity_fingerprint,
            terminal::terminal_open::<Wry>,
            terminal::terminal_close,
        ])
        .events(collect_events![ConnectionChanged, SessionChanged, TerminalChanged, Toast])
}

#[cfg(test)]
mod tests;
