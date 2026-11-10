use cornercase_core::error::Error;
use rstest::rstest;
use tauri::Manager;
use tauri::test::{mock_builder, mock_context, noop_assets};

use super::*;

mod off_android {
    use super::*;

    fn refusal(operation: fn(&dyn Secrets) -> Result<(), Error>) -> Result<(), Error> {
        let app = mock_builder().plugin(init()).build(mock_context(noop_assets())).expect("a mock app");
        operation(app.state::<SecretStore>().secrets().as_ref())
    }

    #[rstest]
    #[case::reading(|store: &dyn Secrets| store.get("identity/ed25519").map(drop))]
    #[case::writing(|store: &dyn Secrets| store.set("identity/ed25519", b"seed"))]
    #[case::deleting(|store: &dyn Secrets| store.delete("identity/ed25519"))]
    fn says_there_is_no_secret_store(#[case] operation: fn(&dyn Secrets) -> Result<(), Error>) {
        assert_eq!(refusal(operation), Err(Error::Secrets("no secret store on this platform yet".to_owned())));
    }
}
