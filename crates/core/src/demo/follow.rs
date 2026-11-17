use std::collections::HashMap;
use std::pin::pin;
use std::sync::{Mutex, PoisonError};

use futures_util::future;
use tokio::sync::watch;
use tokio::time::{self, Instant};

use super::changes::Listed;
use super::surface::Surface;
use super::world::World;
use crate::cli::changes::Mode;
use crate::cli::surface::Size;
use crate::error::Error;
use crate::transport::{Exit, Out, OutSink, emit};

const ENDED: i32 = 1;

pub(super) enum Followed {
    Status { app: bool },
    Events,
    View { tab: u64, size: Size },
    Surface { tab: u64, size: Size },
    Viewed { view: u64 },
    Changes { workspace: u64, mode: Mode },
}

pub(super) async fn run(world: &Mutex<World>, followed: Followed, out: OutSink) -> Result<Exit, Error> {
    match followed {
        Followed::Status { app } => status(world, out, app).await,
        Followed::Events => events(world, out).await,
        Followed::View { tab, size } => view(world, out, tab, size).await,
        Followed::Surface { tab, size } => surface(world, out, tab, size).await,
        Followed::Viewed { view } => viewed(world, out, view).await,
        Followed::Changes { workspace, mode } => changes(world, out, Listed::new(workspace, mode)).await,
    }
}

async fn status(world: &Mutex<World>, out: OutSink, app: bool) -> Result<Exit, Error> {
    revised(world, out, move |world| if app { world.app() } else { world.status() }).await
}

async fn view(world: &Mutex<World>, out: OutSink, tab: u64, size: Size) -> Result<Exit, Error> {
    world.lock().unwrap_or_else(PoisonError::into_inner).views.insert(tab, size);
    revised(world, out, move |world| world.view(tab, size)).await
}

async fn events(world: &Mutex<World>, out: OutSink) -> Result<Exit, Error> {
    let mut told = HashMap::new();
    followed(world, out, move |world| Ok(world.events(&mut told))).await
}

async fn surface(world: &Mutex<World>, out: OutSink, tab: u64, size: Size) -> Result<Exit, Error> {
    drawn(world, out, Surface::new(tab, None, size)).await
}

async fn viewed(world: &Mutex<World>, mut out: OutSink, view: u64) -> Result<Exit, Error> {
    let size = world.lock().unwrap_or_else(PoisonError::into_inner).views.get(&view).copied();
    match size {
        Some(size) => drawn(world, out, Surface::new(view, Some(view), size)).await,
        None => ended(&mut out, format!("error: no view {view}")).await,
    }
}

async fn changes(world: &Mutex<World>, out: OutSink, mut listed: Listed) -> Result<Exit, Error> {
    followed(world, out, move |world| Ok(listed.fresh(world))).await
}

async fn drawn(world: &Mutex<World>, out: OutSink, mut surface: Surface) -> Result<Exit, Error> {
    followed(world, out, move |world| surface.fresh(world)).await
}

async fn revised(world: &Mutex<World>, out: OutSink, line: impl Fn(&World) -> String + Send) -> Result<Exit, Error> {
    let mut shown = None;
    followed(world, out, move |world| {
        let revision = Some(world.session.revision);
        if shown == revision {
            return Ok(Vec::new());
        }
        shown = revision;
        Ok(vec![line(world)])
    })
    .await
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
