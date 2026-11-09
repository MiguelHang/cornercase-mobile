use cornercase_core::cli::contract::Descriptor;
use cornercase_core::error::Error;
use cornercase_core::machines::MachineId;
use cornercase_core::machines::pick::{PickConfig, PickError, Picked, pick_route};
use cornercase_core::machines::route::Route;
use tauri::State;

use crate::commands::connection::Launcher;
use crate::commands::error::CommandError;
use crate::state::AppState;
use crate::view::{ProbeView, RouteFailure};

#[tauri::command]
#[specta::specta]
pub(crate) async fn machine_probe(state: State<'_, AppState>, id: u64) -> Result<ProbeView, CommandError> {
    probe(&state, &Launcher::bridge(), MachineId(id)).await
}

pub(crate) async fn probe(state: &AppState, launcher: &Launcher, id: MachineId) -> Result<ProbeView, CommandError> {
    let machine = state.machines(|machines| machines.machine(id).cloned().ok_or(Error::UnknownMachine.into()))?;
    let picked = pick_route(&machine, |route: &Route| launcher.transport(route), PickConfig::default()).await;
    Ok(match picked {
        Ok(picked) => answered(state, id, picked),
        Err(error) => failed(error),
    })
}

fn answered(state: &AppState, id: MachineId, picked: Picked) -> ProbeView {
    if let Ok(descriptor) = Descriptor::from_value(picked.contract) {
        state.saw_boot(id, descriptor.boot().0);
    }
    ProbeView::Answers { route: picked.route.address().to_string() }
}

fn failed(error: PickError) -> ProbeView {
    let PickError::AllFailed(failures) = error else {
        return ProbeView::NoAnswer { failures: Vec::new() };
    };
    if failures.iter().any(|(_, error)| *error == Error::Unauthorized) {
        return ProbeView::Unauthorized;
    }
    if failures.iter().all(|(_, error)| *error == Error::NoToken) {
        return ProbeView::NoToken;
    }
    let failures = failures
        .iter()
        .map(|(address, error)| RouteFailure { route: address.to_string(), error: error.into() })
        .collect();
    ProbeView::NoAnswer { failures }
}

#[cfg(test)]
mod tests;
