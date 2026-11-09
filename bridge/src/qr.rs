use std::fmt;
use std::net::{IpAddr, SocketAddr};

use cornercase_core::machines::code::MachineCode;
use cornercase_core::machines::route::RouteError;
use qrcode::{Color, EcLevel, QrCode};

use crate::network::Target;

const QUIET: usize = 4;
const DARK: u8 = 16;
const LIGHT: u8 = 231;
const HALF: &str = "▀";
const RESET: &str = "\x1b[0m";
const LOOPBACK: &str = "loopback";
const DASH: &str = " — ";
const GAP: &str = "  ";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Place<'a> {
    Network(&'a Target),
    Loopback,
}

impl fmt::Display for Place<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Network(target) => write!(f, "{} ({})", target.kind.label(), target.interface),
            Self::Loopback => write!(f, "{LOOPBACK}"),
        }
    }
}

pub fn block(name: Option<&str>, places: &[(Place<'_>, SocketAddr)], token: &str) -> Result<Vec<String>, RouteError> {
    let Some(code) = machine_code(name, places, token)? else { return Ok(Vec::new()) };
    let text = code.to_string();
    let mut block = vec![heading(code.name(), places), text.clone()];
    block.extend(lines(&text));
    block.push(String::new());
    block.extend(
        places.iter().map(|&(place, address)| format!("{place}{GAP}{}", url(address.ip(), address.port(), token))),
    );
    Ok(block)
}

fn machine_code(
    name: Option<&str>,
    places: &[(Place<'_>, SocketAddr)],
    token: &str,
) -> Result<Option<MachineCode>, RouteError> {
    let Some((&(_, first), others)) = places.split_first() else { return Ok(None) };
    let code = MachineCode::new(first.ip(), first.port(), token)?;
    let code = others.iter().fold(code, |code, &(_, address)| code.with_also(address.ip()));
    Ok(Some(match name {
        Some(name) => code.with_name(name),
        None => code,
    }))
}

fn heading(name: Option<&str>, places: &[(Place<'_>, SocketAddr)]) -> String {
    let mut labels: Vec<String> = Vec::with_capacity(places.len());
    for label in places.iter().map(|(place, _)| place.to_string()) {
        if !labels.contains(&label) {
            labels.push(label);
        }
    }
    let labels = labels.join(", ");
    match name {
        Some(name) => format!("{name}{DASH}{labels}"),
        None => labels,
    }
}

fn url(ip: IpAddr, port: u16, token: &str) -> String {
    format!("ws://{}/?token={token}", SocketAddr::new(ip, port))
}

fn lines(url: &str) -> Vec<String> {
    let Some((width, dark)) = modules(url) else { return Vec::new() };
    let side = width + 2 * QUIET;
    let at = |x: usize, y: usize| {
        let inside = (QUIET..QUIET + width).contains(&x) && (QUIET..QUIET + width).contains(&y);
        inside && dark[(y - QUIET) * width + (x - QUIET)]
    };
    (0..side.div_ceil(2))
        .map(|row| {
            let cells: String = (0..side).map(|x| cell(at(x, row * 2), at(x, row * 2 + 1))).collect();
            format!("{cells}{RESET}")
        })
        .collect()
}

fn modules(url: &str) -> Option<(usize, Vec<bool>)> {
    let code = QrCode::with_error_correction_level(url.as_bytes(), EcLevel::L).ok()?;
    let width = code.width();
    Some((width, code.to_colors().into_iter().map(|module| module == Color::Dark).collect()))
}

fn cell(top: bool, bottom: bool) -> String {
    format!("\x1b[38;5;{}m\x1b[48;5;{}m{HALF}", shade(top), shade(bottom))
}

fn shade(dark: bool) -> u8 {
    if dark { DARK } else { LIGHT }
}

#[cfg(test)]
mod tests;
