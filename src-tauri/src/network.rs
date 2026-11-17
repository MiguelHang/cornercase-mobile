use cornercase_core::network::{Debounce, Decision, Kind, Network};
use tauri::async_runtime;
use tauri::{AppHandle, Manager, Runtime};
use tokio::sync::mpsc::{self, UnboundedReceiver};

use crate::state::AppState;

const DOWN: Network = Network { kind: Kind::None, expensive: false };

pub(crate) fn listen() -> (impl Fn(Network) + Send + Sync + 'static, UnboundedReceiver<Network>) {
    let (heard, paths) = mpsc::unbounded_channel();
    let hear = move |network| {
        heard.send(network).ok();
    };
    (hear, paths)
}

pub(crate) fn follow_in_background<R: Runtime>(app: &AppHandle<R>, paths: UnboundedReceiver<Network>) {
    async_runtime::spawn(follow(app.clone(), paths));
}

async fn follow<R: Runtime>(app: AppHandle<R>, mut paths: UnboundedReceiver<Network>) {
    let mut debounce = Debounce::default();
    loop {
        tokio::select! {
            path = paths.recv() => match path {
                Some(network) => debounce.observe(network),
                None => return,
            },
            decision = debounce.decided() => decided(&app, decision),
        }
    }
}

fn line(network: Network) -> String {
    let kind = match network.kind {
        Kind::Wifi => "wifi",
        Kind::Cellular => "cellular",
        Kind::Other => "other",
        Kind::None => "none",
    };
    if network.expensive { format!("network {kind} expensive") } else { format!("network {kind}") }
}

fn decided<R: Runtime>(app: &AppHandle<R>, decision: Decision) {
    if let Some(state) = app.try_state::<AppState>() {
        apply(&state, decision);
    }
}

fn apply(state: &AppState, decision: Decision) {
    let network = applied(decision);
    let was_down = state.replace_network(network).is_some_and(Network::is_down);
    eprintln!("cornercase-mobile: {}", line(network));
    if let Some(active) = state.active().as_ref().filter(|active| !active.is_demo()) {
        let store = active.store();
        match decision {
            Decision::Baseline(_) => {}
            Decision::Offline => store.offline(),
            Decision::Changed(_) if was_down => store.online(),
            Decision::Changed(_) => store.network_changed(),
        }
    }
}

fn applied(decision: Decision) -> Network {
    match decision {
        Decision::Baseline(network) | Decision::Changed(network) => network,
        Decision::Offline => DOWN,
    }
}

#[cfg(test)]
mod tests;
