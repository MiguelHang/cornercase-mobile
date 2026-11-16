use std::sync::{Arc, Mutex};

use futures_util::FutureExt;
use futures_util::future::BoxFuture;
use futures_util::stream::BoxStream;

use self::world::World;
use crate::error::Error;
use crate::machines::route::{Address, Route, Token};
use crate::machines::{Machine, MachineId};
use crate::transport::{Exit, OutSink, Stdin, Transport};

mod changes;
mod chat;
mod clock;
mod close;
mod data;
mod dispatch;
mod empty;
mod files;
mod follow;
mod question;
mod queue;
mod start;
mod status;
mod surface;
mod terminal;
mod world;

pub const NAME: &str = "Demo Mac";
const HOST: &str = "demo";

pub fn machine(id: MachineId) -> Machine {
    let route = Route::cli(Address { host: HOST.to_owned(), port: 0 }, Some(Token::new(HOST.to_owned())));
    Machine { id, name: NAME.to_owned(), routes: vec![route], last_route: None }
}

#[derive(Clone, Default)]
#[must_use]
pub struct DemoTransport {
    world: Arc<Mutex<World>>,
}

impl DemoTransport {
    pub fn new() -> Self {
        Self::default()
    }
}

impl Transport for DemoTransport {
    fn run(
        &self,
        argv: Vec<String>,
        stdin: BoxStream<'static, Stdin>,
        out: OutSink,
    ) -> BoxFuture<'_, Result<Exit, Error>> {
        dispatch::run(&self.world, argv, stdin, out).boxed()
    }
}

#[cfg(test)]
mod tests;
