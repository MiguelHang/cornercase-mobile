use super::route::{Address, BridgeUrl, Route, saved_address};
use crate::protocol::BridgeRoutes;

pub fn learned_routes(answer: &BridgeRoutes, winner: &Route) -> Vec<Route> {
    let Some(token) = winner.token() else {
        return Vec::new();
    };
    answer
        .routes
        .iter()
        .filter_map(|route| saved_address(&Address { host: route.host.clone(), port: route.port }.to_string()))
        .map(|address| Route::Bridge { url: BridgeUrl { address, token: Some(token.clone()) } })
        .collect()
}

#[cfg(test)]
mod tests;
