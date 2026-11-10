use std::fmt;
use std::sync::Arc;
use std::time::Duration;

use futures_util::StreamExt;
use futures_util::future::BoxFuture;
use futures_util::stream::FuturesUnordered;
use serde_json::Value;
use tokio::time::{self, Instant};

use super::route::{Address, Route};
use super::{Machine, Reach, reach};
use crate::error::Error;
use crate::transport::bridge::{BridgeConfig, BridgeTransport};
use crate::transport::{self, Transport};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PickConfig {
    pub deadline: Duration,
    pub window: Duration,
}

impl Default for PickConfig {
    fn default() -> Self {
        Self { deadline: Duration::from_secs(5), window: Duration::from_millis(150) }
    }
}

pub struct Picked {
    pub route: Route,
    pub transport: Arc<dyn Transport>,
    pub contract: Value,
}

impl fmt::Debug for Picked {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("Picked")
            .field("route", &self.route)
            .field("contract", &self.contract)
            .finish_non_exhaustive()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum PickError {
    #[error("the machine has no routes")]
    NoRoutes,
    #[error("no route to the machine answered")]
    AllFailed(Vec<(Address, Error)>),
}

type Attempt<'a> = BoxFuture<'a, (usize, Result<Picked, Error>)>;

pub async fn pick_route<F>(machine: &Machine, factory: F, config: PickConfig) -> Result<Picked, PickError>
where
    F: Fn(&Route) -> Result<Arc<dyn Transport>, Error>,
{
    if machine.routes.is_empty() {
        return Err(PickError::NoRoutes);
    }
    let deadline = Instant::now() + config.deadline;
    let mut race = Race::start(machine, &factory);
    let Some(first) = race.first(deadline).await else {
        return Err(PickError::AllFailed(timed_out(machine, race.failures)));
    };
    let until = deadline.min(Instant::now() + config.window);
    Ok(race.prefer(first, until).await)
}

pub fn bridge_transports(config: BridgeConfig) -> impl Fn(&Route) -> Result<Arc<dyn Transport>, Error> {
    move |route| {
        let transport: Arc<dyn Transport> = Arc::new(BridgeTransport::new(route.connection_url()?, config));
        Ok(transport)
    }
}

fn connect<F>(route: &Route, factory: &F) -> Result<Arc<dyn Transport>, Error>
where
    F: Fn(&Route) -> Result<Arc<dyn Transport>, Error>,
{
    if route.token().is_none() {
        return Err(Error::NoToken);
    }
    factory(route)
}

fn attempt(index: usize, route: &Route, transport: Arc<dyn Transport>) -> Attempt<'_> {
    Box::pin(async move {
        let argv = vec!["contract".to_owned(), "--json".to_owned()];
        let result = transport::json::<Value>(transport.as_ref(), argv).await;
        (index, result.map(|contract| Picked { route: route.clone(), transport, contract }))
    })
}

struct Race<'a> {
    racing: FuturesUnordered<Attempt<'a>>,
    pending: Vec<Option<Reach>>,
    failures: Vec<Option<Error>>,
}

impl<'a> Race<'a> {
    fn start<F>(machine: &'a Machine, factory: &F) -> Self
    where
        F: Fn(&Route) -> Result<Arc<dyn Transport>, Error>,
    {
        let count = machine.routes.len();
        let mut race =
            Self { racing: FuturesUnordered::new(), pending: vec![None; count], failures: vec![None; count] };
        for (index, route) in machine.routes.iter().enumerate() {
            match connect(route, factory) {
                Ok(transport) => {
                    race.pending[index] = Some(reach(route.address()));
                    race.racing.push(attempt(index, route, transport));
                }
                Err(error) => race.failures[index] = Some(error),
            }
        }
        race
    }

    async fn next(&mut self, until: Instant) -> Option<(usize, Result<Picked, Error>)> {
        let (index, result) = time::timeout_at(until, self.racing.next()).await.ok()??;
        self.pending[index] = None;
        Some((index, result))
    }

    async fn first(&mut self, deadline: Instant) -> Option<Picked> {
        while let Some((index, result)) = self.next(deadline).await {
            match result {
                Ok(picked) => return Some(picked),
                Err(error) => self.failures[index] = Some(error),
            }
        }
        None
    }

    async fn prefer(&mut self, mut best: Picked, until: Instant) -> Picked {
        let mut best_reach = reach(best.route.address());
        while self.better_pending(best_reach) {
            match self.next(until).await {
                Some((_, Ok(picked))) if reach(picked.route.address()) < best_reach => {
                    best_reach = reach(picked.route.address());
                    best = picked;
                }
                Some(_) => {}
                None => break,
            }
        }
        best
    }

    fn better_pending(&self, than: Reach) -> bool {
        self.pending.iter().flatten().any(|pending| *pending < than)
    }
}

fn timed_out(machine: &Machine, failures: Vec<Option<Error>>) -> Vec<(Address, Error)> {
    let addresses = machine.routes.iter().map(|route| route.address().clone());
    addresses.zip(failures).map(|(address, failure)| (address, failure.unwrap_or(Error::Timeout))).collect()
}

#[cfg(test)]
mod tests;
