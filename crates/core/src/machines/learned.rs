use super::route::{Address, Route, RouteUrl, saved_address};
use crate::protocol::BridgeRoutes;

pub fn learned_routes(answer: &BridgeRoutes, winner: &Route) -> Vec<Route> {
    let Some(token) = winner.token() else {
        return Vec::new();
    };
    answer
        .routes
        .iter()
        .filter_map(|route| saved_address(&Address { host: route.host.clone(), port: route.port }.to_string()))
        .map(|address| Route::Bridge { url: RouteUrl { address, token: Some(token.clone()) } })
        .collect()
}

#[cfg(test)]
mod tests;
