use std::collections::HashMap;
use std::pin::pin;
use std::sync::{Mutex, PoisonError};

use futures_util::future;
use tokio::sync::watch;
use tokio::time::{self, Instant};

use super::world::World;
use crate::error::Error;
use crate::transport::{Exit, Out, OutSink, emit};

pub(super) async fn status(world: &Mutex<World>, out: OutSink, app: bool) -> Result<Exit, Error> {
    let mut shown = None;
    followed(world, out, move |world| {
        let revision = Some(world.session.revision);
        if shown == revision {
            return Vec::new();
        }
        shown = revision;
        vec![if app { world.app() } else { world.status() }]
    })
    .await
}

pub(super) async fn events(world: &Mutex<World>, out: OutSink) -> Result<Exit, Error> {
    let mut told = HashMap::new();
    followed(world, out, move |world| world.events(&mut told)).await
}

async fn followed(
    world: &Mutex<World>,
    mut out: OutSink,
    mut fresh: impl FnMut(&World) -> Vec<String> + Send,
) -> Result<Exit, Error> {
    loop {
        let (lines, mut told, due) = {
            let mut world = world.lock().unwrap_or_else(PoisonError::into_inner);
            world.settle();
            (fresh(&world), world.told.subscribe(), world.next_due())
        };
        emit(&mut out, lines.into_iter().map(Out::Stdout).collect()).await?;
        woken(&mut told, due).await;
    }
}

async fn woken(told: &mut watch::Receiver<u64>, due: Option<Instant>) {
    let due = async move {
        match due {
            Some(at) => time::sleep_until(at).await,
            None => future::pending().await,
        }
    };
    future::select(pin!(told.changed()), pin!(due)).await;
}
