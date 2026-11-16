use crate::cli::contract::served;

const BOOT: &str = "demo";

pub(crate) struct World {
    boot: String,
}

impl Default for World {
    fn default() -> Self {
        Self { boot: BOOT.to_owned() }
    }
}

impl World {
    pub(super) fn contract(&self) -> String {
        served::descriptor(&self.boot).to_string()
    }
}
