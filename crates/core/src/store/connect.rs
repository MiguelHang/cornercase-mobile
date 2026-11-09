use std::sync::Arc;

use super::{Connection, Driver};
use crate::cli::contract::{self, Descriptor};
use crate::error::Error;
use crate::machines::pick::{PickError, Picked, pick_route};
use crate::machines::route::Route;
use crate::transport::Transport;

#[derive(Debug, PartialEq, Eq)]
pub(super) enum Failure {
    Stop(Connection),
    Retry(Error),
}

impl<F> Driver<F>
where
    F: Fn(&Route) -> Result<Arc<dyn Transport>, Error> + Send + Sync,
{
    pub(super) async fn connect(&mut self) -> Result<Picked, Failure> {
        let picked = pick_route(&self.machine, &self.factory, self.config).await.map_err(classify)?;
        self.machine.last_route = Some(picked.route.address().clone());
        let descriptor = Descriptor::from_value(picked.contract.clone())
            .map_err(|error| Failure::Stop(Connection::Incompatible(error)))?;
        self.shared.restarted(descriptor.boot());
        self.join(&picked.transport);
        let missing = contract::missing(&descriptor);
        if missing.is_empty() { Ok(picked) } else { Err(Failure::Stop(Connection::Missing(missing))) }
    }
}

fn classify(error: PickError) -> Failure {
    let PickError::AllFailed(failures) = error else {
        return Failure::Stop(Connection::Unauthorized);
    };
    let errors: Vec<Error> = failures.into_iter().map(|(_, error)| contract::outdated(error)).collect();
    if errors.contains(&Error::Outdated) {
        return Failure::Stop(Connection::Incompatible(Error::Outdated));
    }
    match errors.into_iter().find(|error| !refused(error)) {
        Some(error) => Failure::Retry(error),
        None => Failure::Stop(Connection::Unauthorized),
    }
}

fn refused(error: &Error) -> bool {
    matches!(error, Error::Unauthorized | Error::NoToken)
}
