use std::fmt::Write as _;

use crate::error::Error;

pub fn encoded(value: &[u8]) -> String {
    value.iter().fold(String::with_capacity(value.len() * 2), |mut text, byte| {
        let _ = write!(text, "{byte:02x}");
        text
    })
}

pub fn decoded(hex: &str) -> Result<Vec<u8>, Error> {
    let (pairs, rest) = hex.as_bytes().as_chunks::<2>();
    let bytes: Option<Vec<u8>> = pairs.iter().copied().map(byte).collect();
    bytes.filter(|_| rest.is_empty()).ok_or_else(|| Error::Secrets("a secret is not hex".to_owned()))
}

fn byte([high, low]: [u8; 2]) -> Option<u8> {
    Some((nibble(high)? << 4) | nibble(low)?)
}

fn nibble(digit: u8) -> Option<u8> {
    char::from(digit).to_digit(16).and_then(|value| u8::try_from(value).ok())
}

#[cfg(test)]
mod tests;
