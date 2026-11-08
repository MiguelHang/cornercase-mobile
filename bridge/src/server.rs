use std::fmt;
use std::io;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::pin::pin;
use std::sync::Arc;
use std::time::Duration;

use futures_util::stream::{self, Stream, StreamExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{OwnedSemaphorePermit, Semaphore, watch};
use tokio::task::JoinSet;
use tokio::time::{sleep, timeout};
use tokio_tungstenite::tungstenite::Error;
use tokio_tungstenite::tungstenite::protocol::WebSocketConfig;
use tokio_tungstenite::{WebSocketStream, accept_hdr_async_with_config};

use cornercase_core::protocol::{LARGEST_MESSAGE, LONGEST_LINE};

mod auth;
mod connection;

use auth::Gate;

const PING: Duration = Duration::from_secs(15);
const SILENCE: Duration = Duration::from_secs(45);
const HELLO: Duration = Duration::from_secs(10);
const HANDSHAKE: Duration = Duration::from_secs(10);
const CONNECTIONS: usize = 64;
const HANDSHAKES: usize = 64;
const WRITE_BUFFER_LIMIT: usize = 7 * LONGEST_LINE;
const ACCEPT_PAUSE: Duration = Duration::from_millis(100);

type Stop = watch::Receiver<bool>;

pub struct Config {
    pub token: String,
    pub binary: PathBuf,
    pub ping: Duration,
    pub silence: Duration,
    pub hello: Duration,
    pub handshake: Duration,
    pub connections: usize,
    pub handshakes: usize,
}

impl Config {
    pub fn new(token: String, binary: PathBuf) -> Self {
        Self {
            token,
            binary,
            ping: PING,
            silence: SILENCE,
            hello: HELLO,
            handshake: HANDSHAKE,
            connections: CONNECTIONS,
            handshakes: HANDSHAKES,
        }
    }
}

pub struct Server {
    listeners: Vec<TcpListener>,
    config: Arc<Config>,
}

pub async fn bind(address: SocketAddr, config: Config) -> io::Result<Server> {
    Ok(Server::from_listener(TcpListener::bind(address).await?, config))
}

impl Server {
    pub fn from_listener(listener: TcpListener, config: Config) -> Self {
        Self::from_listeners(vec![listener], config)
    }

    pub fn from_listeners(listeners: Vec<TcpListener>, config: Config) -> Self {
        Self { listeners, config: Arc::new(config) }
    }

    pub fn local_addr(&self) -> io::Result<SocketAddr> {
        let first = self.listeners.first().ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "no listener"))?;
        first.local_addr()
    }

    pub fn local_addrs(&self) -> io::Result<Vec<SocketAddr>> {
        self.listeners.iter().map(TcpListener::local_addr).collect()
    }

    pub async fn run(self, shutdown: impl Future<Output = ()>) {
        let Self { listeners, config } = self;
        for address in listeners.iter().filter_map(|listener| listener.local_addr().ok()) {
            log(format_args!("listening on {address}"));
        }
        let mut accepting = accepting(listeners);
        let (stop, stopping) = watch::channel(false);
        let shared = Shared::new(config, stopping);
        let mut crowding = Crowding::default();
        let mut connections = JoinSet::new();
        let mut shutdown = pin!(shutdown);
        loop {
            tokio::select! {
                biased;
                () = &mut shutdown => break,
                Some(_) = connections.join_next(), if !connections.is_empty() => {}
                Some((stream, peer)) = accepting.next() => {
                    admit(stream, peer, &shared, &mut crowding, &mut connections);
                }
            }
        }
        wind_down(accepting, crowding, &stop, connections).await;
    }
}

fn accepting(listeners: Vec<TcpListener>) -> impl Stream<Item = (TcpStream, SocketAddr)> + Unpin {
    stream::select_all(listeners.into_iter().map(|listener| {
        Box::pin(stream::unfold(listener, async |listener| Some((accepted(&listener).await, listener))))
    }))
}

async fn accepted(listener: &TcpListener) -> (TcpStream, SocketAddr) {
    loop {
        match listener.accept().await {
            Ok(accepted) => return accepted,
            Err(error) => {
                log(format_args!("could not accept a connection: {error}"));
                sleep(ACCEPT_PAUSE).await;
            }
        }
    }
}

async fn wind_down(
    accepting: impl Stream,
    mut crowding: Crowding,
    stop: &watch::Sender<bool>,
    mut connections: JoinSet<()>,
) {
    log_eased(&mut crowding);
    log(format_args!("stopping"));
    drop(accepting);
    stop.send_replace(true);
    while connections.join_next().await.is_some() {}
}

#[derive(Clone)]
struct Shared {
    config: Arc<Config>,
    connections: Arc<Semaphore>,
    handshakes: Arc<Semaphore>,
    stop: Stop,
}

impl Shared {
    fn new(config: Arc<Config>, stop: Stop) -> Self {
        let connections = Arc::new(Semaphore::new(config.connections));
        let handshakes = Arc::new(Semaphore::new(config.handshakes));
        Self { config, connections, handshakes, stop }
    }
}

#[derive(Default)]
struct Crowding {
    refused: usize,
}

impl Crowding {
    fn refuse(&mut self) -> bool {
        self.refused += 1;
        self.refused == 1
    }

    fn ease(&mut self) -> Option<usize> {
        Some(std::mem::take(&mut self.refused).saturating_sub(1)).filter(|&unlogged| unlogged > 0)
    }
}

fn admit(stream: TcpStream, peer: SocketAddr, shared: &Shared, crowding: &mut Crowding, connections: &mut JoinSet<()>) {
    let Ok(handshaking) = Arc::clone(&shared.handshakes).try_acquire_owned() else {
        if crowding.refuse() {
            log(format_args!("refused {peer}: too many handshakes; the next ones are counted until a place frees up"));
        }
        return;
    };
    log_eased(crowding);
    connections.spawn(serve(stream, peer, shared.clone(), handshaking));
}

fn log_eased(crowding: &mut Crowding) {
    if let Some(unlogged) = crowding.ease() {
        let plural = if unlogged == 1 { "" } else { "s" };
        log(format_args!("refused {unlogged} more connection{plural}: too many handshakes"));
    }
}

async fn serve(stream: TcpStream, peer: SocketAddr, shared: Shared, handshaking: OwnedSemaphorePermit) {
    let Shared { config, connections, mut stop, .. } = shared;
    let mut permit = None;
    let gate = Gate { token: &config.token, peer, permits: &connections, permit: &mut permit };
    let shaken = tokio::select! {
        socket = handshake(stream, peer, gate, config.handshake) => socket,
        () = stopped(&mut stop) => None,
    };
    drop(handshaking);
    let (Some(socket), Some(_permit)) = (shaken, permit) else {
        return;
    };
    log(format_args!("accepted {peer}"));
    connection::serve(socket, peer, &config, stop).await;
}

async fn handshake(
    stream: TcpStream,
    peer: SocketAddr,
    gate: Gate<'_>,
    within: Duration,
) -> Option<WebSocketStream<TcpStream>> {
    match timeout(within, accept_hdr_async_with_config(stream, gate, Some(websocket()))).await {
        Ok(Ok(socket)) => Some(socket),
        Ok(Err(Error::Http(_))) => None,
        Ok(Err(error)) => {
            log(format_args!("refused {peer}: the handshake failed: {error}"));
            None
        }
        Err(_) => {
            log(format_args!("refused {peer}: no handshake within {within:?}"));
            None
        }
    }
}

fn websocket() -> WebSocketConfig {
    WebSocketConfig::default()
        .max_message_size(Some(LARGEST_MESSAGE))
        .max_frame_size(Some(LARGEST_MESSAGE))
        .max_write_buffer_size(WRITE_BUFFER_LIMIT)
}

async fn stopped(stop: &mut Stop) {
    let _ = stop.wait_for(|&stopped| stopped).await;
}

fn log(message: fmt::Arguments<'_>) {
    eprintln!("cornercase-bridge: {message}");
}

#[cfg(test)]
mod tests;
