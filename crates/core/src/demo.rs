use std::sync::{Arc, Mutex};

use futures_util::FutureExt;
use futures_util::future::BoxFuture;
use futures_util::stream::BoxStream;

use self::world::World;
use crate::error::Error;
use crate::transport::{Exit, OutSink, Stdin, Transport};

mod chat;
mod clock;
mod data;
mod dispatch;
mod follow;
mod question;
mod queue;
mod status;
mod surface;
mod world;

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
