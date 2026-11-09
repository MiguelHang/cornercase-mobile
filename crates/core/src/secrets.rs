use crate::error::Error;

#[cfg(test)]
pub(crate) mod memory;

pub trait Secrets: Send + Sync {
    fn get(&self, key: &str) -> Result<Option<Vec<u8>>, Error>;
    fn set(&self, key: &str, value: &[u8]) -> Result<(), Error>;
    fn delete(&self, key: &str) -> Result<(), Error>;
}
