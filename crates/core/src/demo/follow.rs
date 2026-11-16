use std::collections::HashMap;
use std::pin::pin;
use std::sync::{Mutex, PoisonError};

use futures_util::future;
use tokio::sync::watch;
use tokio::time::{self, Instant};

use super::surface::Surface;
use super::world::World;
use crate::cli::surface::Size;
use crate::error::Error;
use crate::transport::{Exit, Out, OutSink, emit};

const ENDED: i32 = 1;

pub(super) async fn status(world: &Mutex<World>, out: OutSink, app: bool) -> Result<Exit, Error> {
    let mut shown = None;
    followed(world, out, move |world| {
        let revision = Some(world.session.revision);
        if shown == revision {
            return Ok(Vec::new());
        }
        shown = revision;
        Ok(vec![if app { world.app() } else { world.status() }])
    })
    .await
}

pub(super) async fn events(world: &Mutex<World>, out: OutSink) -> Result<Exit, Error> {
    let mut told = HashMap::new();
    followed(world, out, move |world| Ok(world.events(&mut told))).await
}

pub(super) async fn surface(world: &Mutex<World>, out: OutSink, tab: u64, size: Size) -> Result<Exit, Error> {
    let mut surface = Surface::new(tab, size);
    followed(world, out, move |world| surface.fresh(world)).await
}

async fn followed(
    world: &Mutex<World>,
    mut out: OutSink,
    mut fresh: impl FnMut(&World) -> Result<Vec<String>, String> + Send,
) -> Result<Exit, Error> {
    loop {
        let (lines, mut told, due) = {
            let mut world = world.lock().unwrap_or_else(PoisonError::into_inner);
            world.settle();
            (fresh(&world), world.told.subscribe(), world.next_due())
        };
        let lines = match lines {
            Ok(lines) => lines,
            Err(stderr) => return ended(&mut out, stderr).await,
        };
        emit(&mut out, lines.into_iter().map(Out::Stdout).collect()).await?;
        woken(&mut told, due).await;
    }
}

async fn ended(out: &mut OutSink, stderr: String) -> Result<Exit, Error> {
    emit(out, vec![Out::Stderr(stderr.clone())]).await?;
    Ok(Exit { code: ENDED, stderr })
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
