use std::future::Future;
use std::pin::pin;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::time::Duration;

use futures_util::future;
use serde_json::Value;
use tokio::sync::mpsc;
use tokio::time;

use self::backoff::Backoff;
use self::connect::Failure;
use crate::cli::contract::Missing;
use crate::cli::events::Event;
use crate::cli::status::Snapshot;
use crate::error::Error;
use crate::machines::Machine;
use crate::machines::pick::PickConfig;
use crate::machines::route::{Address, Route};
use crate::transport::Transport;

mod backoff;
mod connect;
mod streams;

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
    resumes: mpsc::UnboundedSender<()>,
}

impl Store {
    pub fn new<F>(machine: Machine, factory: F, config: PickConfig) -> (Self, impl Future<Output = ()> + Send + 'static)
    where
        F: Fn(&Route) -> Result<Arc<dyn Transport>, Error> + Send + Sync + 'static,
    {
        let shared = Arc::new(Shared::default());
        let (resumes, asked) = mpsc::unbounded_channel();
        let driver = Driver { machine, factory, config, shared: shared.clone(), backoff: Backoff::default() };
        (Self { shared, resumes }, driver.drive(asked))
    }

    pub fn connection(&self) -> Connection {
        self.shared.state().connection.clone()
    }

    pub fn session(&self) -> Option<Snapshot> {
        self.shared.state().session.clone()
    }

    pub fn app(&self) -> Option<Value> {
        self.shared.state().app.clone()
    }

    pub fn changes(&self) -> Changes {
        self.shared.subscribe()
    }

    pub fn resume_now(&self) {
        let _ = self.resumes.send(());
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
    app: Option<Value>,
    subscribers: Vec<Subscriber>,
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

    fn set_connection(&self, connection: Connection) {
        self.state().connection = connection.clone();
        self.publish(&Change::Connection(connection));
    }
}

struct Driver<F> {
    machine: Machine,
    factory: F,
    config: PickConfig,
    shared: Arc<Shared>,
    backoff: Backoff,
}

impl<F> Driver<F>
where
    F: Fn(&Route) -> Result<Arc<dyn Transport>, Error> + Send + Sync,
{
    async fn drive(mut self, resumes: mpsc::UnboundedReceiver<()>) {
        future::select(pin!(self.cycle()), pin!(dropped(resumes))).await;
    }

    async fn cycle(&mut self) {
        loop {
            self.shared.set_connection(Connection::Connecting);
            match self.connect().await {
                Ok(picked) => {
                    let error = streams::follow(&self.shared, &picked).await;
                    self.retry(error).await;
                }
                Err(Failure::Retry(error)) => self.retry(error).await,
                Err(Failure::Stop(connection)) => self.stop(connection).await,
            }
        }
    }

    async fn stop(&self, connection: Connection) {
        self.shared.set_connection(connection);
        future::pending::<()>().await;
    }

    async fn retry(&mut self, error: Error) {
        let wait = self.backoff.failed();
        self.shared.set_connection(Connection::Retrying { r#in: wait, error });
        time::sleep(wait).await;
    }
}

async fn dropped(mut resumes: mpsc::UnboundedReceiver<()>) {
    while resumes.recv().await.is_some() {}
}

#[cfg(test)]
mod tests;
