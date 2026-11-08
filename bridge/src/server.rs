use std::fmt;
use std::io;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::pin::pin;
use std::sync::Arc;
use std::time::Duration;

use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{OwnedSemaphorePermit, Semaphore, watch};
use tokio::task::JoinSet;
use tokio::time::{sleep, timeout};
use tokio_tungstenite::tungstenite::Error;
use tokio_tungstenite::tungstenite::protocol::WebSocketConfig;
use tokio_tungstenite::{WebSocketStream, accept_hdr_async_with_config};

use crate::process::LONGEST_LINE;

mod auth;
mod connection;

use auth::Gate;

const PING: Duration = Duration::from_secs(15);
const SILENCE: Duration = Duration::from_secs(45);
const HELLO: Duration = Duration::from_secs(10);
const HANDSHAKE: Duration = Duration::from_secs(10);
const CONNECTIONS: usize = 64;
const HANDSHAKES: usize = 64;
const LARGEST_MESSAGE: usize = 1 << 20;
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
    listener: TcpListener,
    config: Arc<Config>,
}

pub async fn bind(address: SocketAddr, config: Config) -> io::Result<Server> {
    Ok(Server::from_listener(TcpListener::bind(address).await?, config))
}

impl Server {
    pub fn from_listener(listener: TcpListener, config: Config) -> Self {
        Self { listener, config: Arc::new(config) }
    }

    pub fn local_addr(&self) -> io::Result<SocketAddr> {
        self.listener.local_addr()
    }

    pub async fn run(self, shutdown: impl Future<Output = ()>) {
        let Self { listener, config } = self;
        if let Ok(address) = listener.local_addr() {
            log(format_args!("listening on {address}"));
        }
        let (stop, stopping) = watch::channel(false);
        let shared = Shared::new(config, stopping);
        let mut connections = JoinSet::new();
        let mut shutdown = pin!(shutdown);
        loop {
            tokio::select! {
                biased;
                () = &mut shutdown => break,
                Some(_) = connections.join_next(), if !connections.is_empty() => {}
                accepted = listener.accept() => match accepted {
                    Ok((stream, peer)) => admit(stream, peer, &shared, &mut connections),
                    Err(error) => {
                        log(format_args!("could not accept a connection: {error}"));
                        sleep(ACCEPT_PAUSE).await;
                    }
                },
            }
        }
        wind_down(listener, &stop, connections).await;
    }
}

async fn wind_down(listener: TcpListener, stop: &watch::Sender<bool>, mut connections: JoinSet<()>) {
    log(format_args!("stopping"));
    drop(listener);
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

fn admit(stream: TcpStream, peer: SocketAddr, shared: &Shared, connections: &mut JoinSet<()>) {
    let Ok(handshaking) = Arc::clone(&shared.handshakes).try_acquire_owned() else {
        log(format_args!("refused {peer}: too many handshakes"));
        return;
    };
    connections.spawn(serve(stream, peer, shared.clone(), handshaking));
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
