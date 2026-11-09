use cornercase_core::error::Error;
use cornercase_core::identity::Identity;
use cornercase_core::secrets::Secrets;

pub(crate) fn announce(secrets: &dyn Secrets) -> Result<Identity, Error> {
    let identity = Identity::load_or_create(secrets);
    eprintln!("{}", line(&identity));
    identity
}

fn line(identity: &Result<Identity, Error>) -> String {
    match identity {
        Ok(identity) => format!("cornercase-mobile: identity {}", identity.fingerprint()),
        Err(error) => format!("cornercase-mobile: no identity: {error}"),
    }
}

#[cfg(test)]
mod tests;
