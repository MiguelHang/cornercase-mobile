use std::fmt;
use std::sync::Arc;
use std::time::Duration;

use futures_util::StreamExt;
use futures_util::future::BoxFuture;
use futures_util::stream::FuturesUnordered;
use serde_json::Value;
use tokio::time::{self, Instant};

use super::Machine;
use super::route::{Address, Route};
use crate::error::Error;
use crate::transport::bridge::{BridgeConfig, BridgeTransport};
use crate::transport::{self, Transport};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PickConfig {
    pub deadline: Duration,
    pub head_start: Duration,
}

impl Default for PickConfig {
    fn default() -> Self {
        Self { deadline: Duration::from_secs(5), head_start: Duration::from_millis(300) }
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
    let started = Instant::now();
    let deadline = started + config.deadline;
    let favourite = favourite(machine);
    let mut failures = vec![None; machine.routes.len()];
    let mut racing = FuturesUnordered::new();
    let mut others = Vec::new();
    for (index, route) in machine.routes.iter().enumerate() {
        match connect(route, &factory) {
            Ok(transport) if favourite == Some(index) => racing.push(attempt(index, route, transport)),
            Ok(transport) => others.push(attempt(index, route, transport)),
            Err(error) => failures[index] = Some(error),
        }
    }
    if let Some(picked) = race(&mut racing, deadline.min(started + config.head_start), &mut failures).await {
        return Ok(picked);
    }
    racing.extend(others);
    match race(&mut racing, deadline, &mut failures).await {
        Some(picked) => Ok(picked),
        None => Err(PickError::AllFailed(timed_out(machine, failures))),
    }
}

pub fn bridge_transports(config: BridgeConfig) -> impl Fn(&Route) -> Result<Arc<dyn Transport>, Error> {
    move |route| {
        let transport: Arc<dyn Transport> = Arc::new(BridgeTransport::new(route.connection_url()?, config));
        Ok(transport)
    }
}

fn favourite(machine: &Machine) -> Option<usize> {
    let saved = machine.last_route.as_ref()?;
    machine.routes.iter().position(|route| route.address() == saved)
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

async fn race(
    racing: &mut FuturesUnordered<Attempt<'_>>,
    deadline: Instant,
    failures: &mut [Option<Error>],
) -> Option<Picked> {
    while let Ok(Some((index, result))) = time::timeout_at(deadline, racing.next()).await {
        match result {
            Ok(picked) => return Some(picked),
            Err(error) => failures[index] = Some(error),
        }
    }
    None
}

fn timed_out(machine: &Machine, failures: Vec<Option<Error>>) -> Vec<(Address, Error)> {
    let addresses = machine.routes.iter().map(|route| route.address().clone());
    addresses.zip(failures).map(|(address, failure)| (address, failure.unwrap_or(Error::Timeout))).collect()
}

#[cfg(test)]
mod tests;
