use super::route::{Address, Route, saved_address};
use crate::error::Error;
use crate::protocol::{APP_ROUTES, AppRoutes};
use crate::transport::{self, Transport};

pub async fn ask_routes(transport: &dyn Transport, winner: &Route) -> Result<Vec<Route>, Error> {
    let question = APP_ROUTES.iter().map(|&word| word.to_owned()).collect();
    let answer = transport::json::<AppRoutes>(transport, question).await?;
    Ok(learned_routes(&answer, winner))
}

pub fn learned_routes(answer: &AppRoutes, winner: &Route) -> Vec<Route> {
    if !winner.has_token() {
        return Vec::new();
    }
    answer
        .routes
        .iter()
        .filter_map(|route| saved_address(&Address { host: route.host.clone(), port: route.port }.to_string()))
        .map(|address| Route::cli(address, winner.token().cloned()))
        .collect()
}

#[cfg(test)]
mod tests;
