use super::route::{Address, Route, saved_address};
use crate::error::Error;
use crate::protocol::{APP_ROUTES, BRIDGE_ROUTES, BridgeRoutes};
use crate::transport::{self, Transport};

pub async fn ask_routes(transport: &dyn Transport, winner: &Route) -> Result<Vec<Route>, Error> {
    let answer = transport::json::<BridgeRoutes>(transport, question(winner)).await?;
    Ok(learned_routes(&answer, winner))
}

pub fn learned_routes(answer: &BridgeRoutes, winner: &Route) -> Vec<Route> {
    if !winner.has_token() {
        return Vec::new();
    }
    answer
        .routes
        .iter()
        .filter_map(|route| saved_address(&Address { host: route.host.clone(), port: route.port }.to_string()))
        .map(|address| winner.at(address))
        .collect()
}

fn question(winner: &Route) -> Vec<String> {
    match winner {
        Route::Bridge { .. } => vec![BRIDGE_ROUTES.to_owned()],
        Route::Cli { .. } => APP_ROUTES.iter().map(|&word| word.to_owned()).collect(),
    }
}

#[cfg(test)]
mod tests;
