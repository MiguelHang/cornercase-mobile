use std::env;
use std::path::PathBuf;
use std::process::ExitCode;

use clap::Parser;

use cornercase_bridge::binary;

const NOT_FOUND: u8 = 2;

#[derive(Parser)]
#[command(version, about = "Runs cornercase commands for the cornercase mobile app")]
struct Args {
    #[arg(long, value_name = "PATH", help = "The cornercase binary to run, instead of CORNERCASE_BIN or the PATH")]
    cornercase: Option<PathBuf>,
}

fn main() -> ExitCode {
    let args = Args::parse();
    let variable = env::var_os(binary::VARIABLE);
    let path = env::var_os("PATH");
    match binary::find(args.cornercase.as_deref(), variable.as_deref(), path.as_deref(), binary::is_executable) {
        Ok(_) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("cornercase-bridge: {error}");
            ExitCode::from(NOT_FOUND)
        }
    }
}
