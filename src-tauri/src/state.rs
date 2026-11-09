use std::collections::HashMap;
use std::fmt::Display;
use std::path::PathBuf;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use cornercase_core::error::Error;
use cornercase_core::identity::Identity;
use cornercase_core::machines::MachineId;
use cornercase_core::machines::registry::Machines;
use cornercase_core::machines::store::JsonFileStore;
use cornercase_core::network::Network;
use cornercase_core::secrets::Secrets;
use cornercase_core::store::Store;
use tauri::async_runtime::JoinHandle;

use crate::commands::error::CommandError;
use crate::identity;

const MACHINES_FILE: &str = "machines.json";

pub(crate) struct AppState {
    machines: SharedMachines,
    identity: Result<Identity, Error>,
    active: Mutex<Option<Active>>,
    network: Mutex<Option<Network>>,
    boots: Mutex<HashMap<MachineId, String>>,
}

#[derive(Clone)]
pub(crate) struct SharedMachines(Arc<Mutex<Result<Machines, CommandError>>>);

pub(crate) struct Active {
    machine: MachineId,
    store: Store,
    driver: JoinHandle<()>,
    forwarder: JoinHandle<()>,
    terminal: Slot,
    changes: Slot,
}

#[derive(Default)]
struct Slot(Option<(u64, JoinHandle<()>)>);

impl AppState {
    pub(crate) fn load(data_dir: Result<PathBuf, impl Display>, secrets: Arc<dyn Secrets>) -> Self {
        let identity = identity::announce(secrets.as_ref());
        let machines = match data_dir {
            Ok(folder) => Machines::load(Box::new(JsonFileStore::new(folder.join(MACHINES_FILE))), secrets)
                .map_err(|error| CommandError::machines_not_loaded(&error)),
            Err(reason) => Err(CommandError::no_data_dir(reason)),
        };
        Self {
            machines: SharedMachines(Arc::new(Mutex::new(machines))),
            identity,
            active: Mutex::new(None),
            network: Mutex::new(None),
            boots: Mutex::new(HashMap::new()),
        }
    }

    pub(crate) fn machines<T>(
        &self,
        work: impl FnOnce(&mut Machines) -> Result<T, CommandError>,
    ) -> Result<T, CommandError> {
        self.machines.with(work)
    }

    pub(crate) fn shared_machines(&self) -> SharedMachines {
        self.machines.clone()
    }

    pub(crate) fn identity(&self) -> Result<&Identity, CommandError> {
        self.identity.as_ref().map_err(CommandError::no_identity)
    }

    pub(crate) fn active(&self) -> MutexGuard<'_, Option<Active>> {
        self.active.lock().unwrap_or_else(PoisonError::into_inner)
    }

    pub(crate) fn replace_network(&self, network: Network) -> Option<Network> {
        self.network.lock().unwrap_or_else(PoisonError::into_inner).replace(network)
    }

    pub(crate) fn network_is_down(&self) -> bool {
        self.network.lock().unwrap_or_else(PoisonError::into_inner).is_some_and(Network::is_down)
    }

    pub(crate) fn saw_boot(&self, machine: MachineId, boot: String) {
        self.boots.lock().unwrap_or_else(PoisonError::into_inner).insert(machine, boot);
    }

    pub(crate) fn seen_boots(&self) -> HashMap<MachineId, String> {
        let mut boots = self.boots.lock().unwrap_or_else(PoisonError::into_inner).clone();
        let active = self.active().as_ref().and_then(|active| Some((active.machine(), active.store().boot()?)));
        boots.extend(active);
        boots
    }
}

impl SharedMachines {
    pub(crate) fn with<T>(
        &self,
        work: impl FnOnce(&mut Machines) -> Result<T, CommandError>,
    ) -> Result<T, CommandError> {
        let mut machines = self.0.lock().unwrap_or_else(PoisonError::into_inner);
        match machines.as_mut() {
            Ok(machines) => work(machines),
            Err(error) => Err(error.clone()),
        }
    }
}

impl Active {
    pub(crate) fn new(machine: MachineId, store: Store, driver: JoinHandle<()>, forwarder: JoinHandle<()>) -> Self {
        Self { machine, store, driver, forwarder, terminal: Slot::default(), changes: Slot::default() }
    }

    pub(crate) fn machine(&self) -> MachineId {
        self.machine
    }

    pub(crate) fn store(&self) -> &Store {
        &self.store
    }

    pub(crate) fn show(&mut self, tab: u64, terminal: JoinHandle<()>) {
        self.terminal.fill(tab, terminal);
    }

    pub(crate) fn hide(&mut self, tab: u64) {
        self.terminal.empty(tab);
    }

    pub(crate) fn follow(&mut self, workspace: u64, changes: JoinHandle<()>) {
        self.changes.fill(workspace, changes);
    }

    pub(crate) fn unfollow(&mut self, workspace: u64) {
        self.changes.empty(workspace);
    }
}

impl Drop for Active {
    fn drop(&mut self) {
        self.driver.abort();
        self.forwarder.abort();
    }
}

impl Slot {
    fn fill(&mut self, key: u64, task: JoinHandle<()>) {
        self.stop();
        self.0 = Some((key, task));
    }

    fn empty(&mut self, key: u64) {
        if self.0.as_ref().is_some_and(|(held, _)| *held == key) {
            self.stop();
        }
    }

    fn stop(&mut self) {
        if let Some((_, task)) = self.0.take() {
            task.abort();
        }
    }
}

impl Drop for Slot {
    fn drop(&mut self) {
        self.stop();
    }
}

#[cfg(test)]
mod tests;
