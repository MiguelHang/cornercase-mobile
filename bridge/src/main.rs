use std::env;
use std::io;
use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use clap::Parser;
use clap::builder::NonEmptyStringValueParser;
use tokio::signal::unix::{SignalKind, signal};

use cornercase_bridge::binary;
use cornercase_bridge::server::{self, Config};

const NOT_FOUND: u8 = 2;
const FAILED: u8 = 1;

#[derive(Parser)]
#[command(version, about = "Runs cornercase commands for the cornercase mobile app")]
struct Args {
    #[arg(
        long,
        value_name = "ADDRESS",
        value_parser = loopback,
        help = "The loopback address and port to listen on, such as 127.0.0.1:0 for any free port"
    )]
    listen: SocketAddr,
    #[arg(
        long,
        value_name = "TOKEN",
        value_parser = NonEmptyStringValueParser::new(),
        help = "The token every client must send"
    )]
    token: String,
    #[arg(long, value_name = "PATH", help = "The cornercase binary to run, instead of CORNERCASE_BIN or the PATH")]
    cornercase: Option<PathBuf>,
}

fn loopback(text: &str) -> Result<SocketAddr, String> {
    let address: SocketAddr = text.parse().map_err(|error| format!("{error}"))?;
    if address.ip().is_loopback() {
        Ok(address)
    } else {
        Err(format!(
            "{} is not a loopback address (127.0.0.0/8 or ::1); listening beyond this machine comes with task 006",
            address.ip()
        ))
    }
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> ExitCode {
    let args = Args::parse();
    let binary = match find(args.cornercase.as_deref()) {
        Ok(binary) => binary,
        Err(error) => return fail(&error, NOT_FOUND),
    };
    match serve(args.listen, Config::new(args.token, binary)).await {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => fail(&error, FAILED),
    }
}

fn find(option: Option<&Path>) -> Result<PathBuf, binary::NotFound> {
    let variable = env::var_os(binary::VARIABLE);
    let path = env::var_os("PATH");
    binary::find(option, variable.as_deref(), path.as_deref(), binary::is_executable)
}

async fn serve(address: SocketAddr, config: Config) -> io::Result<()> {
    let shutdown = shutdown()?;
    let server = server::bind(address, config)
        .await
        .map_err(|error| io::Error::new(error.kind(), format!("could not listen on {address}: {error}")))?;
    server.run(shutdown).await;
    Ok(())
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

fn fail(error: &impl std::fmt::Display, code: u8) -> ExitCode {
    eprintln!("cornercase-bridge: {error}");
    ExitCode::from(code)
}
