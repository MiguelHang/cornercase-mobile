use std::collections::HashMap;
use std::fs;
use std::future;
use std::sync::atomic::{AtomicUsize, Ordering};

use cornercase_bridge::test_util::TempDir;
use cornercase_core::machines::Machine;
use cornercase_core::machines::pick::PickConfig;
use cornercase_core::machines::route::Route;
use cornercase_core::network::Kind;
use cornercase_core::transport::Transport;
use cornercase_core::transport::fake::FakeTransport;

use super::*;
use crate::commands::error::ErrorCode;
use crate::secrets::{KeychainSecrets, mock_store, on_mock};
use crate::test_util::{code, released, route};

const IDENTITY_KEY: &str = "identity/ed25519";
const NO_FOLDER: Result<PathBuf, &str> = Err("no home folder");

fn keychain() -> Arc<dyn Secrets> {
    Arc::new(on_mock(mock_store()))
}

fn locked_keychain() -> Arc<dyn Secrets> {
    Arc::new(KeychainSecrets::new(Err(Error::Secrets("locked".to_owned())), HashMap::new()))
}

fn loaded(dir: &TempDir, secrets: Arc<dyn Secrets>) -> AppState {
    AppState::load(Ok::<_, &str>(dir.path().to_path_buf()), secrets)
}

fn add_studio(state: &AppState) -> Result<MachineId, CommandError> {
    state.machines(|machines| machines.add("studio", &code()).map_err(CommandError::from))
}

fn saved_file(dir: &TempDir) -> String {
    fs::read_to_string(dir.path().join("machines.json")).expect("read the machines file")
}

mod loading_the_machines {
    use super::*;

    #[test]
    fn finds_none_without_a_file() {
        let dir = TempDir::new();
        let state = loaded(&dir, keychain());
        assert_eq!(state.machines(|machines| Ok(machines.machines().len())), Ok(0));
    }

    #[test]
    fn saves_them_in_the_data_folder() {
        let dir = TempDir::new();
        let state = loaded(&dir, keychain());
        add_studio(&state).expect("the machine is added");
        assert!(saved_file(&dir).contains("\"studio\""), "{}", saved_file(&dir));
    }

    #[test]
    fn keeps_an_unreadable_file_and_says_why() {
        let dir = TempDir::new();
        fs::write(dir.path().join("machines.json"), "{ not json").expect("write the machines file");
        let state = loaded(&dir, keychain());
        let error = add_studio(&state).expect_err("the machines are not loaded");
        assert_eq!(error.code, ErrorCode::MachinesNotLoaded);
        assert!(error.message.contains("is not JSON"), "{}", error.message);
        assert_eq!(saved_file(&dir), "{ not json");
    }

    #[test]
    fn says_why_the_keychain_failed_and_goes_on() {
        let dir = TempDir::new();
        add_studio(&loaded(&dir, keychain())).expect("the machine is added");
        let state = loaded(&dir, locked_keychain());
        assert_eq!(
            state.machines(|machines| Ok(machines.machines().len())),
            Err(CommandError::machines_not_loaded(&Error::Secrets("locked".to_owned())))
        );
        assert!(state.active().is_none());
        assert!(saved_file(&dir).contains("\"studio\""), "{}", saved_file(&dir));
    }

    #[test]
    fn are_the_same_through_a_shared_handle() {
        let dir = TempDir::new();
        let state = loaded(&dir, keychain());
        let shared = state.shared_machines();
        shared.with(|machines| machines.add("studio", &code()).map_err(CommandError::from)).expect("added");
        assert_eq!(state.machines(|machines| Ok(machines.machines().len())), Ok(1));
    }

    #[test]
    fn says_there_is_no_data_folder() {
        let state = AppState::load(NO_FOLDER, keychain());
        let error = add_studio(&state).expect_err("there is no folder for the machines");
        assert_eq!(error, CommandError::no_data_dir("no home folder"));
    }
}

mod the_identity {
    use super::*;

    struct Counted {
        inner: KeychainSecrets,
        identity_reads: AtomicUsize,
    }

    impl Secrets for Counted {
        fn get(&self, key: &str) -> Result<Option<Vec<u8>>, Error> {
            if key == IDENTITY_KEY {
                self.identity_reads.fetch_add(1, Ordering::Relaxed);
            }
            self.inner.get(key)
        }

        fn set(&self, key: &str, value: &[u8]) -> Result<(), Error> {
            self.inner.set(key, value)
        }

        fn delete(&self, key: &str) -> Result<(), Error> {
            self.inner.delete(key)
        }
    }

    #[test]
    fn is_read_from_the_keychain_once() {
        let counted = Arc::new(Counted { inner: on_mock(mock_store()), identity_reads: AtomicUsize::new(0) });
        let state = AppState::load(NO_FOLDER, counted.clone());
        let first = state.identity().map(Identity::fingerprint);
        let second = state.identity().map(Identity::fingerprint);
        assert!(first.is_ok(), "{first:?}");
        assert_eq!(first, second);
        assert_eq!(counted.identity_reads.load(Ordering::Relaxed), 1);
    }

    #[test]
    fn says_why_there_is_none() {
        let state = AppState::load(NO_FOLDER, locked_keychain());
        assert_eq!(
            state.identity().map(Identity::fingerprint),
            Err(CommandError::no_identity(&Error::Secrets("locked".to_owned())))
        );
    }
}

mod the_network {
    use rstest::rstest;

    use super::*;

    const WIFI: Network = Network { kind: Kind::Wifi, expensive: false };
    const NONE: Network = Network { kind: Kind::None, expensive: false };

    #[test]
    fn is_unknown_until_the_first_one_is_applied() {
        let state = AppState::load(NO_FOLDER, keychain());
        assert_eq!(state.replace_network(WIFI), None);
    }

    #[test]
    fn gives_back_the_one_it_replaces() {
        let state = AppState::load(NO_FOLDER, keychain());
        state.replace_network(NONE);
        assert_eq!(state.replace_network(WIFI), Some(NONE));
        assert_eq!(state.replace_network(WIFI), Some(WIFI));
    }

    #[rstest]
    #[case::unknown(None, false)]
    #[case::none(Some(NONE), true)]
    #[case::wifi(Some(WIFI), false)]
    fn is_down_only_once_none_is_applied(#[case] applied: Option<Network>, #[case] down: bool) {
        let state = AppState::load(NO_FOLDER, keychain());
        if let Some(network) = applied {
            state.replace_network(network);
        }
        assert_eq!(state.network_is_down(), down);
    }
}

mod an_active_store {
    use super::*;

    struct Running {
        transport: Arc<FakeTransport>,
        active: Active,
    }

    fn running(id: u64, forwarder: impl FnOnce(Store) -> JoinHandle<()>) -> Running {
        let transport = Arc::new(FakeTransport::default());
        let held = Arc::clone(&transport);
        let machine = Machine { id: MachineId(id), name: "studio".to_owned(), routes: vec![route()], last_route: None };
        let factory = move |_: &Route| Ok(Arc::clone(&held) as Arc<dyn Transport>);
        let (store, driver) = Store::new(machine, factory, PickConfig::default());
        let forwarder = forwarder(store.clone());
        let active = Active::new(MachineId(id), store, JoinHandle::Tokio(tokio::spawn(driver)), forwarder);
        Running { transport, active }
    }

    fn idle(_: Store) -> JoinHandle<()> {
        JoinHandle::Tokio(tokio::spawn(future::pending()))
    }

    fn holding(marker: Arc<()>) -> impl FnOnce(Store) -> JoinHandle<()> {
        move |store| {
            JoinHandle::Tokio(tokio::spawn(async move {
                let _held = (marker, store);
                future::pending::<()>().await;
            }))
        }
    }

    #[tokio::test(start_paused = true)]
    async fn ends_its_driver_when_dropped() {
        let Running { transport, active } = running(1, idle);
        assert!(Arc::strong_count(&transport) > 1);
        drop(active);
        released(&transport).await;
    }

    #[tokio::test(start_paused = true)]
    async fn ends_a_forwarder_that_holds_the_store() {
        let marker = Arc::new(());
        let Running { transport, active } = running(1, holding(Arc::clone(&marker)));
        drop(active);
        released(&marker).await;
        released(&transport).await;
    }

    #[tokio::test(start_paused = true)]
    async fn gives_way_to_the_next_one() {
        let state = AppState::load(NO_FOLDER, keychain());
        let first = running(1, idle);
        *state.active() = Some(first.active);
        let second = running(2, idle);
        *state.active() = Some(second.active);
        released(&first.transport).await;
        let active = state.active();
        let kept = active.as_ref().map(|active| (active.machine(), active.store().session()));
        assert_eq!(kept, Some((MachineId(2), None)));
    }

    fn task(marker: &Arc<()>) -> JoinHandle<()> {
        let marker = Arc::clone(marker);
        JoinHandle::Tokio(tokio::spawn(async move {
            future::pending::<()>().await;
            drop(marker);
        }))
    }

    fn holds(marker: &Arc<()>) -> bool {
        Arc::strong_count(marker) > 1
    }

    struct Both {
        active: Active,
        changes: Arc<()>,
        terminal: Arc<()>,
    }

    fn both() -> Both {
        let Running { mut active, .. } = running(1, idle);
        let (changes, terminal) = (Arc::new(()), Arc::new(()));
        active.follow(2, task(&changes));
        active.show(3, task(&terminal));
        Both { active, changes, terminal }
    }

    #[tokio::test(start_paused = true)]
    async fn keeps_following_changes_while_a_terminal_comes_and_goes() {
        let Both { mut active, changes, terminal } = both();
        active.hide(3);
        released(&terminal).await;
        assert!(holds(&changes));
    }

    #[tokio::test(start_paused = true)]
    async fn keeps_the_terminal_while_changes_are_followed_again_and_dropped() {
        let Both { mut active, changes, terminal } = both();
        let next = Arc::new(());
        active.follow(4, task(&next));
        released(&changes).await;
        active.unfollow(4);
        released(&next).await;
        assert!(holds(&terminal));
    }

    #[tokio::test(start_paused = true)]
    async fn keeps_both_when_another_workspace_is_unfollowed_or_another_tab_hidden() {
        let Both { mut active, changes, terminal } = both();
        active.unfollow(3);
        active.hide(2);
        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
        assert!(holds(&changes) && holds(&terminal));
    }

    #[tokio::test(start_paused = true)]
    async fn stops_the_search_before_a_new_one_and_keeps_both() {
        let Both { mut active, changes, terminal } = both();
        let (before, next) = (Arc::new(()), Arc::new(()));
        active.search(2, task(&before));
        active.search(2, task(&next));
        released(&before).await;
        assert!(holds(&next) && holds(&changes) && holds(&terminal));
    }

    #[tokio::test(start_paused = true)]
    async fn stops_both_when_dropped() {
        let Both { active, changes, terminal } = both();
        drop(active);
        released(&changes).await;
        released(&terminal).await;
    }
}
