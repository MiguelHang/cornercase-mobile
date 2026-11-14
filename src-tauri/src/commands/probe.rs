use cornercase_core::cli::contract::Descriptor;
use cornercase_core::error::Error;
use cornercase_core::machines::code::MachineCode;
use cornercase_core::machines::pick::{PickConfig, PickError, Picked, pick_route};
use cornercase_core::machines::registry::Machines;
use cornercase_core::machines::route::Route;
use cornercase_core::machines::same::{Same, Why};
use cornercase_core::machines::{Machine, MachineId};
use tauri::State;

use crate::commands::connection::Launcher;
use crate::commands::error::CommandError;
use crate::state::AppState;
use crate::view::{MatchFound, MatchView, ProbeView, RouteFailure, SameWhy};

#[tauri::command]
#[specta::specta]
pub(crate) async fn machine_probe(state: State<'_, AppState>, id: u64) -> Result<ProbeView, CommandError> {
    probe(&state, &Launcher::default(), MachineId(id)).await
}

#[tauri::command]
#[specta::specta]
pub(crate) async fn machine_match(
    state: State<'_, AppState>,
    code: String,
    name: String,
) -> Result<MatchView, CommandError> {
    matching(&state, &Launcher::default(), &code, &name).await
}

pub(crate) async fn probe(state: &AppState, launcher: &Launcher, id: MachineId) -> Result<ProbeView, CommandError> {
    let machine = state.machines(|machines| machines.machine(id).cloned().ok_or(Error::UnknownMachine.into()))?;
    let picked = pick_route(&machine, |route: &Route| launcher.transport(route), PickConfig::default()).await;
    Ok(match picked {
        Ok(picked) => answered(state, id, picked),
        Err(error) => failed(error),
    })
}

pub(crate) async fn matching(
    state: &AppState,
    launcher: &Launcher,
    code: &str,
    name: &str,
) -> Result<MatchView, CommandError> {
    let routes = paired_routes(state, launcher, code).await?;
    if let Some(found) = compare(state, &routes, name, None, |same| same.why != Why::Name)? {
        return Ok(MatchView { found, checked: true });
    }
    let unsaved = Machine { id: MachineId(0), name: name.to_owned(), routes: routes.clone(), last_route: None };
    let picked = pick_route(&unsaved, |route: &Route| launcher.transport(route), PickConfig::default()).await.ok();
    let checked = picked.is_some();
    let boot = picked.and_then(|picked| Descriptor::from_value(picked.contract).ok()).map(|found| found.boot().0);
    let found = compare(state, &routes, name, boot.as_deref(), |_| true)?.unwrap_or(MatchFound::None);
    Ok(MatchView { found, checked })
}

pub(crate) async fn paired_routes(
    state: &AppState,
    launcher: &Launcher,
    text: &str,
) -> Result<Vec<Route>, CommandError> {
    let code = MachineCode::parse(text)?;
    if !code.needs_pairing() {
        return Ok(code.routes());
    }
    if let Some(secret) = state.secret_of(text) {
        return Ok(code.cli_routes(&secret));
    }
    let secret = launcher.pair(&code).await?;
    let routes = code.cli_routes(&secret);
    state.redeemed(text, secret);
    Ok(routes)
}

fn compare(
    state: &AppState,
    routes: &[Route],
    name: &str,
    boot: Option<&str>,
    keep: impl FnOnce(&Same) -> bool,
) -> Result<Option<MatchFound>, CommandError> {
    let seen = state.seen_boots();
    state.machines(|machines| {
        Ok(machines.same(routes, name, boot, &seen).filter(keep).map(|same| found(machines, same)))
    })
}

fn found(machines: &Machines, same: Same) -> MatchFound {
    let machine = same.machine.0;
    let name = machines.machine(same.machine).map(|saved| saved.name.clone()).unwrap_or_default();
    if same.why.merges() {
        return MatchFound::Merge { machine, name };
    }
    let why = match same.why {
        Why::Boot => SameWhy::Boot,
        Why::Name => SameWhy::Name,
        _ => SameWhy::Address,
    };
    MatchFound::Same { machine, name, why }
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
    if failures.iter().any(|(_, error)| *error == Error::NotPaired) {
        return ProbeView::NotPaired;
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
