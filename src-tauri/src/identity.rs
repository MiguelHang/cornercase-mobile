use cornercase_core::error::Error;
use cornercase_core::identity::Identity;

use crate::secrets;

pub(crate) fn announce() {
    eprintln!("{}", line(Identity::load_or_create(&secrets::store())));
}

fn line(identity: Result<Identity, Error>) -> String {
    match identity {
        Ok(identity) => format!("cornercase-mobile: identity {}", identity.fingerprint()),
        Err(error) => format!("cornercase-mobile: no identity: {error}"),
    }
}

#[cfg(test)]
mod tests;
