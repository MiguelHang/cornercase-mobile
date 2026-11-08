use std::env;
use std::fmt;
use std::io::{self, Write};
use std::net::{IpAddr, Ipv4Addr};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use clap::Parser;
use tokio::net::TcpListener;
use tokio::signal::unix::{SignalKind, signal};

use cornercase_bridge::network::{self, Target};
use cornercase_bridge::qr::{self, Place};
use cornercase_bridge::server::{Config, Server};
use cornercase_bridge::{binary, token};

const NOT_FOUND: u8 = 2;
const FAILED: u8 = 1;
const PORT: u16 = 8765;
const LOOPBACK: IpAddr = IpAddr::V4(Ipv4Addr::LOCALHOST);

#[derive(Parser)]
#[command(version, about = "Runs cornercase commands for the cornercase mobile app")]
struct Args {
    #[arg(long, value_name = "PORT", default_value_t = PORT, help = "The port to listen on, 0 for any free one")]
    port: u16,
    #[arg(long, help = "Make a new token, so the URLs and QR codes printed before stop working")]
    new_token: bool,
    #[arg(long, help = "Listen only on 127.0.0.1, instead of the LAN and Tailscale addresses")]
    loopback: bool,
    #[arg(long, value_name = "PATH", help = "The cornercase binary to run, instead of CORNERCASE_BIN or the PATH")]
    cornercase: Option<PathBuf>,
}

struct Failure {
    message: String,
    code: u8,
}

fn exit<E: fmt::Display>(code: u8) -> impl FnOnce(E) -> Failure {
    move |error| Failure { message: error.to_string(), code }
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> ExitCode {
    match run(Args::parse()).await {
        Ok(()) => ExitCode::SUCCESS,
        Err(failure) => {
            eprintln!("cornercase-bridge: {}", failure.message);
            ExitCode::from(failure.code)
        }
    }
}

async fn run(args: Args) -> Result<(), Failure> {
    let binary = find(args.cornercase.as_deref()).map_err(exit(NOT_FOUND))?;
    let token = load_token(args.new_token)?;
    let targets = if args.loopback { None } else { Some(network_targets()?) };
    let places = places(targets.as_deref());
    let shutdown = shutdown().map_err(exit(FAILED))?;
    let ips: Vec<IpAddr> = places.iter().map(|&(_, ip)| ip).collect();
    let listeners = network::listen(&ips, args.port).await.map_err(exit(FAILED))?;
    let count = listeners.len();
    eprintln!("cornercase-bridge: listening on {count} address{}", if count == 1 { "" } else { "es" });
    print(&places, &listeners, &token).map_err(exit(FAILED))?;
    Server::from_listeners(listeners, Config::new(token, binary)).run(shutdown).await;
    Ok(())
}

fn find(option: Option<&Path>) -> Result<PathBuf, binary::NotFound> {
    let variable = env::var_os(binary::VARIABLE);
    let path = env::var_os("PATH");
    binary::find(option, variable.as_deref(), path.as_deref(), binary::is_executable)
}

fn load_token(new: bool) -> Result<String, Failure> {
    let config_home = env::var_os(token::CONFIG_HOME);
    let home = env::var_os(token::HOME);
    let path = token::path(config_home.as_deref(), home.as_deref()).map_err(exit(NOT_FOUND))?;
    let loaded = if new { token::rotate(&path) } else { token::load_or_create(&path) };
    loaded.map_err(exit(FAILED))
}

fn network_targets() -> Result<Vec<Target>, Failure> {
    let interfaces = network::interfaces()
        .map_err(|error| exit(FAILED)(format!("could not list the network interfaces: {error}")))?;
    network::targets(&interfaces).map_err(exit(FAILED))
}

fn places(targets: Option<&[Target]>) -> Vec<(Place<'_>, IpAddr)> {
    match targets {
        None => vec![(Place::Loopback, LOOPBACK)],
        Some(targets) => targets.iter().map(|target| (Place::Network(target), target.ip)).collect(),
    }
}

fn print(places: &[(Place<'_>, IpAddr)], listeners: &[TcpListener], token: &str) -> io::Result<()> {
    let mut blocks = Vec::with_capacity(places.len());
    for (&(place, _), listener) in places.iter().zip(listeners) {
        let address = listener.local_addr()?;
        blocks.push(qr::block(place, &qr::url(address.ip(), address.port(), token)).join("\n"));
    }
    writeln!(io::stdout().lock(), "{}", blocks.join("\n\n"))
}

fn shutdown() -> io::Result<impl Future<Output = ()>> {
    let mut interrupt = signal(SignalKind::interrupt())?;
    let mut terminate = signal(SignalKind::terminate())?;
    Ok(async move {
        tokio::select! {
            _ = interrupt.recv() => {}
            _ = terminate.recv() => {}
        }
    })
}
