use std::fmt;
use std::fmt::Write as _;

use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use ed25519_dalek::SigningKey;

use crate::error::Error;

const FINGERPRINT_BYTES: usize = 5;

pub struct Identity {
    signing: SigningKey,
}

impl Identity {
    #[cfg_attr(not(test), expect(dead_code, reason = "only the tests read a seed until load_or_create comes"))]
    pub(crate) fn from_seed(seed: &[u8]) -> Result<Self, Error> {
        let signing = SigningKey::try_from(seed).map_err(|_| Error::Identity)?;
        Ok(Self { signing })
    }

    pub fn public_key(&self) -> String {
        URL_SAFE_NO_PAD.encode(self.signing.verifying_key().as_bytes())
    }

    pub fn fingerprint(&self) -> String {
        let key = self.signing.verifying_key();
        key.as_bytes()[..FINGERPRINT_BYTES].iter().fold(String::new(), |mut text, byte| {
            let _ = write!(text, "{byte:02x}");
            text
        })
    }
}

impl fmt::Debug for Identity {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.debug_struct("Identity").finish_non_exhaustive()
    }
}

#[cfg(test)]
mod tests;
