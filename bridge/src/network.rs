use std::fmt;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

const TAILSCALE_V4_FIRST: u8 = 100;
const TAILSCALE_V4_MASK: u8 = 0b1100_0000;
const TAILSCALE_V4_SECOND: u8 = 64;
const TAILSCALE_V6_PREFIX: [u16; 3] = [0xfd7a, 0x115c, 0xa1e0];

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Kind {
    Lan,
    Tailscale,
}

impl Kind {
    pub fn label(self) -> &'static str {
        match self {
            Self::Lan => "LAN",
            Self::Tailscale => "Tailscale",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Target {
    pub kind: Kind,
    pub interface: String,
    pub ip: IpAddr,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NoAddress;

impl fmt::Display for NoAddress {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "no LAN or Tailscale address to listen on")
    }
}

impl std::error::Error for NoAddress {}

pub fn eligible(ip: IpAddr) -> Option<Kind> {
    match ip {
        IpAddr::V4(ip) => eligible_v4(ip),
        IpAddr::V6(ip) => eligible_v6(ip),
    }
}

fn eligible_v4(ip: Ipv4Addr) -> Option<Kind> {
    let [first, second, ..] = ip.octets();
    if ip.is_private() {
        Some(Kind::Lan)
    } else if first == TAILSCALE_V4_FIRST && second & TAILSCALE_V4_MASK == TAILSCALE_V4_SECOND {
        Some(Kind::Tailscale)
    } else {
        None
    }
}

fn eligible_v6(ip: Ipv6Addr) -> Option<Kind> {
    (ip.segments()[..TAILSCALE_V6_PREFIX.len()] == TAILSCALE_V6_PREFIX).then_some(Kind::Tailscale)
}

pub fn targets(addresses: &[(String, IpAddr)]) -> Result<Vec<Target>, NoAddress> {
    let mut targets: Vec<Target> = addresses
        .iter()
        .filter_map(|(interface, ip)| eligible(*ip).map(|kind| Target { kind, interface: interface.clone(), ip: *ip }))
        .collect();
    targets.sort_by_key(|target| (target.kind, target.ip));
    targets.dedup_by_key(|target| target.ip);
    if targets.is_empty() { Err(NoAddress) } else { Ok(targets) }
}

#[cfg(test)]
mod tests;
