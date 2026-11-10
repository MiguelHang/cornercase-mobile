use std::path::Path;
use std::process::Command;

const SCUTIL: &str = "/usr/sbin/scutil";
const LOCAL: &str = ".local";

pub fn read() -> Option<String> {
    machine_name(computer_name(Path::new(SCUTIL)), &rustix::system::uname().nodename().to_string_lossy())
}

fn computer_name(scutil: &Path) -> Option<String> {
    let output = Command::new(scutil).args(["--get", "ComputerName"]).output().ok()?;
    output.status.success().then(|| String::from_utf8(output.stdout).ok()).flatten()
}

pub(crate) fn machine_name(computer_name: Option<String>, hostname: &str) -> Option<String> {
    let computer_name = computer_name.map(|name| name.trim().to_owned()).filter(|name| !name.is_empty());
    computer_name.or_else(|| {
        let hostname = hostname.trim();
        let hostname = hostname.strip_suffix(LOCAL).unwrap_or(hostname);
        Some(hostname.to_owned()).filter(|hostname| !hostname.is_empty())
    })
}

#[cfg(test)]
mod tests;
