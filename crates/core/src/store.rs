use std::future::Future;
use std::mem;
use std::pin::pin;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::time::Duration;

use futures_util::FutureExt;
use futures_util::future::{self, BoxFuture};
use serde_json::Value;
use tokio::sync::{mpsc, oneshot};
use tokio::time;

use self::backoff::Backoff;
use self::connect::Failure;
use self::resume::{Resume, Resumes, Woken};
use self::streams::Ended;
use crate::board::{self, AgentRow, Since};
use crate::cli::contract::Missing;
use crate::cli::contract::boot::{Boot, BootWatch, Observed};
use crate::cli::events::Event;
use crate::cli::status::Snapshot;
use crate::error::Error;
use crate::machines::Machine;
use crate::machines::pick::{PickConfig, Picked};
use crate::machines::route::{Address, Route};
use crate::queue::Queue;
use crate::transport::Transport;

mod backoff;
mod connect;
mod resume;
mod streams;

const DISAGREE: &str = "the server's contract and its snapshots name different boots";

type Worker = BoxFuture<'static, ()>;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub enum Connection {
    #[default]
    Connecting,
    Connected {
        route: Address,
    },
    Retrying {
        r#in: Duration,
        error: Error,
    },
    Missing(Vec<Missing>),
    Unauthorized,
    Incompatible(Error),
    Offline,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Change {
    Connection(Connection),
    Session,
    App,
    Event(Box<Event>),
    Warning(Error),
}

#[derive(Clone)]
pub struct Store {
    shared: Arc<Shared>,
    resumes: mpsc::UnboundedSender<Resume>,
}

impl Store {
    pub fn new<F>(machine: Machine, factory: F, config: PickConfig) -> (Self, impl Future<Output = ()> + Send + 'static)
    where
        F: Fn(&Route) -> Result<Arc<dyn Transport>, Error> + Send + Sync + 'static,
    {
        let shared = Arc::new(Shared::default());
        let (resumes, asked) = mpsc::unbounded_channel();
        let (driver, worker) = Driver::new(machine, factory, config, shared.clone());
        (Self { shared, resumes }, driver.drive(asked, worker))
    }

    pub fn connection(&self) -> Connection {
        self.shared.state().connection.clone()
    }

    pub fn session(&self) -> Option<Snapshot> {
        self.shared.state().session.clone()
    }

    pub fn agents(&self) -> Option<Vec<AgentRow>> {
        let state = self.shared.state();
        state.session.as_ref().map(|session| board::agents(session, &state.since))
    }

    pub fn app(&self) -> Option<Value> {
        self.shared.state().app.clone()
    }

    pub fn queue(&self) -> Option<Queue> {
        self.shared.queue()
    }

    pub fn changes(&self) -> Changes {
        self.shared.subscribe()
    }

    pub fn resume_now(&self) {
        let _ = self.resumes.send(Resume::Foreground);
    }

    pub fn network_changed(&self) {
        let _ = self.resumes.send(Resume::Network);
    }

    pub fn offline(&self) {
        let _ = self.resumes.send(Resume::Offline);
    }

    pub fn online(&self) {
        let _ = self.resumes.send(Resume::Online);
    }
}

pub struct Changes {
    receiver: mpsc::UnboundedReceiver<Change>,
    unread: Arc<Unread>,
}

impl Changes {
    pub async fn next(&mut self) -> Option<Change> {
        let change = self.receiver.recv().await?;
        self.unread.read(&change);
        Some(change)
    }
}

#[derive(Default)]
struct Unread {
    session: AtomicBool,
    app: AtomicBool,
}

impl Unread {
    fn flag(&self, change: &Change) -> Option<&AtomicBool> {
        match change {
            Change::Session => Some(&self.session),
            Change::App => Some(&self.app),
            Change::Connection(_) | Change::Event(_) | Change::Warning(_) => None,
        }
    }

    fn read(&self, change: &Change) {
        if let Some(flag) = self.flag(change) {
            flag.store(false, Ordering::SeqCst);
        }
    }

    fn already(&self, change: &Change) -> bool {
        self.flag(change).is_some_and(|flag| flag.swap(true, Ordering::SeqCst))
    }
}

struct Subscriber {
    sender: mpsc::UnboundedSender<Change>,
    unread: Arc<Unread>,
}

impl Subscriber {
    fn send(&self, change: &Change) -> bool {
        if self.unread.already(change) {
            return !self.sender.is_closed();
        }
        self.sender.send(change.clone()).is_ok()
    }
}

#[derive(Default)]
struct Shared {
    state: Mutex<State>,
}

#[derive(Default)]
struct State {
    connection: Connection,
    session: Option<Snapshot>,
    since: Since,
    app: Option<Value>,
    queue: Option<Queue>,
    subscribers: Vec<Subscriber>,
    boots: BootWatch,
}

impl Shared {
    fn state(&self) -> MutexGuard<'_, State> {
        self.state.lock().unwrap_or_else(PoisonError::into_inner)
    }

    fn subscribe(&self) -> Changes {
        let (sender, receiver) = mpsc::unbounded_channel();
        let unread = Arc::new(Unread::default());
        self.state().subscribers.push(Subscriber { sender, unread: unread.clone() });
        Changes { receiver, unread }
    }

    fn publish(&self, change: &Change) {
        self.state().subscribers.retain(|subscriber| subscriber.send(change));
    }

    fn queue(&self) -> Option<Queue> {
        self.state().queue.clone()
    }

    fn connected(&self) -> bool {
        matches!(self.state().connection, Connection::Connected { .. })
    }

    fn set_connection(&self, connection: Connection) {
        self.state().connection = connection.clone();
        self.publish(&Change::Connection(connection));
    }

    fn forget(&self) {
        let mut state = self.state();
        state.session = None;
        state.since.clear();
        state.app = None;
        drop(state);
        self.publish(&Change::Session);
        self.publish(&Change::App);
    }

    fn restarted(&self, boot: Boot) -> bool {
        let restarted = self.state().boots.observe(boot) == Observed::Restarted;
        if restarted {
            self.forget();
        }
        restarted
    }
}

struct Driver<F> {
    machine: Machine,
    factory: F,
    config: PickConfig,
    shared: Arc<Shared>,
    backoff: Backoff,
    restarted: bool,
    worker: Option<oneshot::Sender<Worker>>,
}

impl<F> Driver<F>
where
    F: Fn(&Route) -> Result<Arc<dyn Transport>, Error> + Send + Sync,
{
    fn new(machine: Machine, factory: F, config: PickConfig, shared: Arc<Shared>) -> (Self, oneshot::Receiver<Worker>) {
        let (worker, started) = oneshot::channel();
        let backoff = Backoff::default();
        (Self { machine, factory, config, shared, backoff, restarted: false, worker: Some(worker) }, started)
    }

    async fn drive(mut self, resumes: mpsc::UnboundedReceiver<Resume>, worker: oneshot::Receiver<Worker>) {
        let mut resumes = Resumes::new(resumes);
        let attempts = async { while self.attempt(&mut resumes).await {} };
        future::select(pin!(attempts), pin!(worked(worker))).await;
    }

    fn join(&mut self, transport: &Arc<dyn Transport>) {
        if let Some(queue) = self.shared.queue() {
            queue.reconnect(transport.clone());
            return;
        }
        let shared = self.shared.clone();
        let (queue, worker) = Queue::new(transport.clone(), move |error| shared.publish(&Change::Warning(error)));
        self.shared.state().queue = Some(queue);
        if let Some(started) = self.worker.take() {
            let _ = started.send(worker.boxed());
        }
    }

    async fn attempt(&mut self, resumes: &mut Resumes) -> bool {
        if resumes.forget() {
            return self.offline(resumes).await;
        }
        self.shared.set_connection(Connection::Connecting);
        let picked = match resumes.race(self.connect(), Resume::interrupts).await {
            Woken::Done(Ok(picked)) => picked,
            Woken::Done(Err(Failure::Retry(error))) => return self.retry(error, resumes).await,
            Woken::Done(Err(Failure::Stop(connection))) => return self.stop(connection, resumes).await,
            woken => return self.woke(woken, resumes).await,
        };
        match self.follow(&picked, resumes).await {
            Woken::Done(error) => self.retry(error, resumes).await,
            woken => self.woke(woken, resumes).await,
        }
    }

    async fn follow(&mut self, picked: &Picked, resumes: &mut Resumes) -> Woken<Error> {
        let shared = &self.shared;
        let woken =
            resumes.race(streams::follow(shared, picked), |resume| resume.interrupts() || shared.connected()).await;
        let reached = self.shared.connected();
        if reached {
            self.backoff.reset();
            self.restarted = false;
        }
        if reached && !matches!(woken, Woken::Offline) {
            resumes.up();
        }
        match woken {
            Woken::Done(Ended::Failed(error)) => Woken::Done(error),
            Woken::Done(Ended::Restarted) => self.restart(&picked.transport),
            Woken::Resumed => Woken::Resumed,
            Woken::Offline => Woken::Offline,
            Woken::Dropped => Woken::Dropped,
        }
    }

    fn restart(&mut self, transport: &Arc<dyn Transport>) -> Woken<Error> {
        self.join(transport);
        if mem::replace(&mut self.restarted, true) {
            return Woken::Done(Error::StaleBoot(DISAGREE.to_owned()));
        }
        Woken::Resumed
    }

    async fn stop(&mut self, connection: Connection, resumes: &mut Resumes) -> bool {
        self.shared.set_connection(connection);
        let woken = resumes.asked::<()>(|_| true).await;
        self.woke(woken, resumes).await
    }

    async fn retry(&mut self, error: Error, resumes: &mut Resumes) -> bool {
        if resumes.is_down() {
            return self.offline(resumes).await;
        }
        let wait = self.backoff.failed();
        self.shared.set_connection(Connection::Retrying { r#in: wait, error });
        let woken = resumes.race(time::sleep(wait), |_| true).await;
        self.woke(woken, resumes).await
    }

    async fn woke<T>(&mut self, woken: Woken<T>, resumes: &mut Resumes) -> bool {
        match woken {
            Woken::Done(_) | Woken::Resumed => true,
            Woken::Offline => self.offline(resumes).await,
            Woken::Dropped => false,
        }
    }

    async fn offline(&mut self, resumes: &mut Resumes) -> bool {
        self.shared.set_connection(Connection::Offline);
        let woken = resumes.asked::<()>(|resume| matches!(resume, Resume::Online | Resume::Foreground)).await;
        self.backoff.reset();
        !matches!(woken, Woken::Dropped)
    }
}

async fn worked(worker: oneshot::Receiver<Worker>) {
    if let Ok(worker) = worker.await {
        worker.await;
    }
}

#[cfg(test)]
mod tests;
