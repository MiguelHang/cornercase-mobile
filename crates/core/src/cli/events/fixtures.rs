macro_rules! event {
    ($name:literal) => {
        include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/control/events/v1/", $name, ".json"))
    };
}

pub(crate) const BELL: &str = event!("bell");
pub(crate) const CLIPBOARD: &str = event!("clipboard");
pub(crate) const CLOSED: &str = event!("closed");
pub(crate) const DROPPED: &str = event!("dropped");
pub(crate) const EXITED: &str = event!("exited");
pub(crate) const NOTICE: &str = event!("notice");
pub(crate) const NOTICE_MACHINE: &str = event!("notice-machine");
pub(crate) const NOTICE_SOUND: &str = event!("notice-sound");
pub(crate) const OPENED: &str = event!("opened");
pub(crate) const OPENED_ACCOUNTS: &str = event!("opened-accounts");
pub(crate) const OPENED_PARENT: &str = event!("opened-parent");
pub(crate) const RECAP_FAILED: &str = event!("recap-failed");
pub(crate) const RECAP_FINISHED: &str = event!("recap-finished");
pub(crate) const RECAP_STARTED: &str = event!("recap-started");
pub(crate) const STATUS: &str = event!("status");
