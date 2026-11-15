use std::collections::HashMap;
use std::fmt::Display;
use std::path::PathBuf;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::time::Duration;

use cornercase_core::error::Error;
use cornercase_core::history::store::JsonFolderStore;
use cornercase_core::identity::Identity;
use cornercase_core::machines::MachineId;
use cornercase_core::machines::registry::Machines;
use cornercase_core::machines::route::Token;
use cornercase_core::machines::store::JsonFileStore;
use cornercase_core::network::Network;
use cornercase_core::secrets::Secrets;
use cornercase_core::store::Store;
use tauri::async_runtime::JoinHandle;
use tokio::time::Instant;

use crate::commands::error::CommandError;
use crate::identity;
use crate::state::attached::AttachedImages;

pub(crate) mod attached;

const MACHINES_FILE: &str = "machines.json";
const HISTORY_FOLDER: &str = "history";
const REDEEMED_FOR: Duration = Duration::from_mins(10);

pub(crate) struct AppState {
    machines: SharedMachines,
    identity: Result<Identity, Error>,
    active: Mutex<Option<Active>>,
    network: Mutex<Option<Network>>,
    boots: Mutex<HashMap<MachineId, String>>,
    attached: AttachedImages,
    history: Mutex<Option<JsonFolderStore>>,
    redeemed: Mutex<Option<Redeemed>>,
}

struct Redeemed {
    code: String,
    secret: Token,
    until: Instant,
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
    search: Slot,
    question: Slot<(u64, u64)>,
}

#[derive(Default)]
struct Slot<K = u64>(Option<(K, JoinHandle<()>)>);

impl AppState {
    pub(crate) fn load(
        data_dir: Result<PathBuf, impl Display>,
        identity: &dyn Secrets,
        routes: Arc<dyn Secrets>,
    ) -> Self {
        let identity = identity::announce(identity);
        let machines = match data_dir {
            Ok(folder) => Machines::load(Box::new(JsonFileStore::new(folder.join(MACHINES_FILE))), routes)
                .map_err(|error| CommandError::machines_not_loaded(&error)),
            Err(reason) => Err(CommandError::no_data_dir(reason)),
        };
        Self {
            machines: SharedMachines(Arc::new(Mutex::new(machines))),
            identity,
            active: Mutex::new(None),
            network: Mutex::new(None),
            boots: Mutex::new(HashMap::new()),
            attached: AttachedImages::default(),
            history: Mutex::new(None),
            redeemed: Mutex::new(None),
        }
    }

    pub(crate) fn keeping_history(self, cache_dir: Option<PathBuf>) -> Self {
        Self { history: Mutex::new(cache_dir.map(|folder| JsonFolderStore::new(folder.join(HISTORY_FOLDER)))), ..self }
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

    pub(crate) fn attached(&self) -> &AttachedImages {
        &self.attached
    }

    pub(crate) fn history(&self) -> MutexGuard<'_, Option<JsonFolderStore>> {
        self.history.lock().unwrap_or_else(PoisonError::into_inner)
    }

    pub(crate) fn redeemed(&self, code: &str, secret: Token) {
        let code = code.trim().to_owned();
        *self.redeemed_slot() = Some(Redeemed { code, secret, until: Instant::now() + REDEEMED_FOR });
    }

    pub(crate) fn secret_of(&self, code: &str) -> Option<Token> {
        let mut slot = self.redeemed_slot();
        if slot.as_ref().is_some_and(|redeemed| redeemed.until <= Instant::now()) {
            *slot = None;
        }
        slot.as_ref().filter(|redeemed| redeemed.code == code.trim()).map(|redeemed| redeemed.secret.clone())
    }

    pub(crate) fn used(&self, code: &str) {
        let mut slot = self.redeemed_slot();
        if slot.as_ref().is_some_and(|redeemed| redeemed.code == code.trim()) {
            *slot = None;
        }
    }

    pub(crate) fn forget_redeemed(&self) {
        *self.redeemed_slot() = None;
    }

    fn redeemed_slot(&self) -> MutexGuard<'_, Option<Redeemed>> {
        self.redeemed.lock().unwrap_or_else(PoisonError::into_inner)
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
        Self {
            machine,
            store,
            driver,
            forwarder,
            terminal: Slot::default(),
            changes: Slot::default(),
            search: Slot::default(),
            question: Slot::default(),
        }
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
        self.terminal.empty(&tab);
    }

    pub(crate) fn follow(&mut self, workspace: u64, changes: JoinHandle<()>) {
        self.changes.fill(workspace, changes);
    }

    pub(crate) fn unfollow(&mut self, workspace: u64) {
        self.changes.empty(&workspace);
    }

    pub(crate) fn ask(&mut self, tab: u64, pane: u64, question: JoinHandle<()>) {
        self.question.fill((tab, pane), question);
    }

    pub(crate) fn unask(&mut self, tab: u64, pane: u64) {
        self.question.empty(&(tab, pane));
    }

    pub(crate) fn search(&mut self, workspace: u64, search: JoinHandle<()>) {
        self.search.fill(workspace, search);
    }

    pub(crate) fn unsearch(&mut self, workspace: u64) {
        self.search.empty(&workspace);
    }
}

impl Drop for Active {
    fn drop(&mut self) {
        self.driver.abort();
        self.forwarder.abort();
    }
}

impl<K: PartialEq> Slot<K> {
    fn fill(&mut self, key: K, task: JoinHandle<()>) {
        self.stop();
        self.0 = Some((key, task));
    }

    fn empty(&mut self, key: &K) {
        if self.0.as_ref().is_some_and(|(held, _)| held == key) {
            self.stop();
        }
    }
}

impl<K> Slot<K> {
    fn stop(&mut self) {
        if let Some((_, task)) = self.0.take() {
            task.abort();
        }
    }
}

impl<K> Drop for Slot<K> {
    fn drop(&mut self) {
        self.stop();
    }
}

#[cfg(test)]
mod tests;
