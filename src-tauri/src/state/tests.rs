use std::collections::HashMap;
use std::fs;
use std::future;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

use cornercase_bridge::test_util::TempDir;
use cornercase_core::machines::Machine;
use cornercase_core::machines::pick::PickConfig;
use cornercase_core::machines::route::{Route, Token};
use cornercase_core::network::Kind;
use cornercase_core::secrets::file::FileSecrets;
use cornercase_core::transport::Transport;
use cornercase_core::transport::fake::FakeTransport;

use super::*;
use crate::commands::error::ErrorCode;
use crate::secrets::{KeychainSecrets, mock_store, on_mock};
use crate::test_util::{PAIRING, SECRET, TOKEN, code, released, route};

const IDENTITY_KEY: &str = "identity/ed25519";
const NO_FOLDER: Result<PathBuf, &str> = Err("no home folder");

fn keychain() -> Arc<dyn Secrets> {
    Arc::new(on_mock(mock_store()))
}

fn locked_keychain() -> Arc<dyn Secrets> {
    Arc::new(KeychainSecrets::new(Err(Error::Secrets("locked".to_owned())), HashMap::new()))
}

fn loaded(dir: &TempDir, routes: Arc<dyn Secrets>) -> AppState {
    loaded_with(dir, keychain().as_ref(), routes)
}

fn loaded_with(dir: &TempDir, identity: &dyn Secrets, routes: Arc<dyn Secrets>) -> AppState {
    AppState::load(Ok::<_, &str>(dir.path().to_path_buf()), identity, routes)
}

fn folderless() -> AppState {
    AppState::load(NO_FOLDER, keychain().as_ref(), keychain())
}

fn add_studio(state: &AppState) -> Result<MachineId, CommandError> {
    state.machines(|machines| machines.add("studio", &code().routes()).map_err(CommandError::from))
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
        shared.with(|machines| machines.add("studio", &code().routes()).map_err(CommandError::from)).expect("added");
        assert_eq!(state.machines(|machines| Ok(machines.machines().len())), Ok(1));
    }

    #[test]
    fn says_there_is_no_data_folder() {
        let error = add_studio(&folderless()).expect_err("there is no folder for the machines");
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
        let counted = Counted { inner: on_mock(mock_store()), identity_reads: AtomicUsize::new(0) };
        let state = AppState::load(NO_FOLDER, &counted, keychain());
        let first = state.identity().map(Identity::fingerprint);
        let second = state.identity().map(Identity::fingerprint);
        assert!(first.is_ok(), "{first:?}");
        assert_eq!(first, second);
        assert_eq!(counted.identity_reads.load(Ordering::Relaxed), 1);
    }

    #[test]
    fn says_why_there_is_none() {
        let state = AppState::load(NO_FOLDER, locked_keychain().as_ref(), keychain());
        assert_eq!(
            state.identity().map(Identity::fingerprint),
            Err(CommandError::no_identity(&Error::Secrets("locked".to_owned())))
        );
    }
}

mod separate_stores {
    use super::*;

    fn tokens(state: &AppState) -> Result<Vec<bool>, CommandError> {
        state.machines(|machines| {
            Ok(machines.machines().iter().flat_map(|machine| machine.routes.iter().map(Route::has_token)).collect())
        })
    }

    #[test]
    fn add_and_reload_machines_while_the_identity_store_is_broken() {
        let dir = TempDir::new();
        let routes = keychain();
        let state = loaded_with(&dir, locked_keychain().as_ref(), Arc::clone(&routes));
        assert!(state.identity().is_err());
        add_studio(&state).expect("the machine is added");
        assert_eq!(tokens(&loaded_with(&dir, locked_keychain().as_ref(), routes)), Ok(vec![true]));
    }

    #[test]
    fn keep_the_identity_while_the_routes_store_is_broken() {
        let dir = TempDir::new();
        add_studio(&loaded(&dir, keychain())).expect("the machine is added");
        let state = loaded_with(&dir, keychain().as_ref(), locked_keychain());
        assert!(state.identity().is_ok());
        assert!(tokens(&state).is_err());
    }

    #[test]
    fn bring_a_machine_back_with_its_token_from_a_secrets_file() {
        let dir = TempDir::new();
        let file = || -> Arc<dyn Secrets> { Arc::new(FileSecrets::new(dir.path().join("route-secrets.json"))) };
        add_studio(&loaded(&dir, file())).expect("the machine is added");
        assert_eq!(tokens(&loaded(&dir, file())), Ok(vec![true]));
        assert!(!saved_file(&dir).contains(TOKEN), "{}", saved_file(&dir));
    }
}

mod the_attached_images {
    use super::*;

    #[test]
    fn are_kept_by_the_app_state() {
        let state = folderless();
        let id = state.attached().store(7, "photo.jpg".to_owned(), vec![1]).expect("the image is stored");
        assert_eq!(state.attached().get(7, id).map(|image| image.bytes), Some(vec![1]));
    }
}

mod the_redeemed_secret {
    use tokio::time;

    use super::*;

    const DESK: &str = "ws://desk.lan:47682/?code=0f1e2d3c4b5a69788796a5b4c3d2e1f0";

    fn redeemed() -> AppState {
        let state = folderless();
        state.redeemed(&format!(" {PAIRING}\n"), Token::fake(SECRET));
        state
    }

    #[test]
    fn is_kept_for_its_code_only() {
        let state = redeemed();
        assert_eq!(state.secret_of(PAIRING), Some(Token::fake(SECRET)));
        assert_eq!(state.secret_of(&format!("  {PAIRING}")), Some(Token::fake(SECRET)));
        assert_eq!(state.secret_of(DESK), None);
    }

    #[test]
    fn gives_way_to_the_next_one() {
        let state = redeemed();
        state.redeemed(DESK, Token::fake("aa.bb"));
        assert_eq!(state.secret_of(PAIRING), None);
        assert_eq!(state.secret_of(DESK), Some(Token::fake("aa.bb")));
    }

    #[tokio::test(start_paused = true)]
    async fn lasts_ten_minutes() {
        let state = redeemed();
        time::advance(Duration::from_mins(10) - Duration::from_millis(1)).await;
        assert_eq!(state.secret_of(PAIRING), Some(Token::fake(SECRET)));
        time::advance(Duration::from_millis(1)).await;
        assert_eq!(state.secret_of(PAIRING), None);
    }

    #[tokio::test(start_paused = true)]
    async fn is_gone_once_expired_even_for_another_code() {
        let state = redeemed();
        time::advance(Duration::from_mins(10)).await;
        state.secret_of(DESK);
        assert!(state.redeemed.lock().unwrap_or_else(PoisonError::into_inner).is_none());
    }

    #[test]
    fn is_forgotten_once_its_code_is_used() {
        let state = redeemed();
        state.used(DESK);
        assert_eq!(state.secret_of(PAIRING), Some(Token::fake(SECRET)));
        state.used(&format!("{PAIRING} "));
        assert_eq!(state.secret_of(PAIRING), None);
    }

    #[test]
    fn is_forgotten_as_the_form_closes() {
        let state = redeemed();
        state.forget_redeemed();
        assert_eq!(state.secret_of(PAIRING), None);
    }
}

mod the_network {
    use rstest::rstest;

    use super::*;

    const WIFI: Network = Network { kind: Kind::Wifi, expensive: false };
    const NONE: Network = Network { kind: Kind::None, expensive: false };

    #[test]
    fn is_unknown_until_the_first_one_is_applied() {
        let state = folderless();
        assert_eq!(state.replace_network(WIFI), None);
    }

    #[test]
    fn gives_back_the_one_it_replaces() {
        let state = folderless();
        state.replace_network(NONE);
        assert_eq!(state.replace_network(WIFI), Some(NONE));
        assert_eq!(state.replace_network(WIFI), Some(WIFI));
    }

    #[rstest]
    #[case::unknown(None, false)]
    #[case::none(Some(NONE), true)]
    #[case::wifi(Some(WIFI), false)]
    fn is_down_only_once_none_is_applied(#[case] applied: Option<Network>, #[case] down: bool) {
        let state = folderless();
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
        let state = folderless();
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

    struct Held {
        active: Active,
        changes: Arc<()>,
        terminal: Arc<()>,
        question: Arc<()>,
    }

    fn held() -> Held {
        let Running { mut active, .. } = running(1, idle);
        let (changes, terminal, question) = (Arc::new(()), Arc::new(()), Arc::new(()));
        active.follow(2, task(&changes));
        active.show(3, task(&terminal));
        active.ask(3, 5, task(&question));
        Held { active, changes, terminal, question }
    }

    #[tokio::test(start_paused = true)]
    async fn keeps_following_changes_while_a_terminal_comes_and_goes() {
        let Held { mut active, changes, terminal, .. } = held();
        active.hide(3);
        released(&terminal).await;
        assert!(holds(&changes));
    }

    #[tokio::test(start_paused = true)]
    async fn keeps_the_terminal_while_changes_are_followed_again_and_dropped() {
        let Held { mut active, changes, terminal, .. } = held();
        let next = Arc::new(());
        active.follow(4, task(&next));
        released(&changes).await;
        active.unfollow(4);
        released(&next).await;
        assert!(holds(&terminal));
    }

    #[tokio::test(start_paused = true)]
    async fn keeps_the_question_of_a_tab_while_its_terminal_is_hidden() {
        let Held { mut active, terminal, question, .. } = held();
        active.hide(3);
        released(&terminal).await;
        assert!(holds(&question));
    }

    #[tokio::test(start_paused = true)]
    async fn keeps_the_terminal_of_a_tab_while_its_question_is_dropped() {
        let Held { mut active, terminal, question, .. } = held();
        active.unask(3, 5);
        released(&question).await;
        assert!(holds(&terminal));
    }

    #[tokio::test(start_paused = true)]
    async fn keeps_the_question_of_a_pane_when_another_pane_of_its_tab_is_unasked() {
        let Held { mut active, question, .. } = held();
        active.unask(3, 6);
        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
        assert!(holds(&question));
    }

    #[tokio::test(start_paused = true)]
    async fn ends_the_previous_question_when_another_is_asked() {
        let Held { mut active, terminal, question, .. } = held();
        let next = Arc::new(());
        active.ask(4, 5, task(&next));
        released(&question).await;
        assert!(holds(&next) && holds(&terminal));
    }

    #[tokio::test(start_paused = true)]
    async fn keeps_all_three_when_another_workspace_is_unfollowed_or_another_tab_hidden_or_unasked() {
        let Held { mut active, changes, terminal, question } = held();
        active.unfollow(3);
        active.hide(2);
        active.unask(2, 5);
        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
        assert!(holds(&changes) && holds(&terminal) && holds(&question));
    }

    #[tokio::test(start_paused = true)]
    async fn stops_the_search_before_a_new_one_and_keeps_the_others() {
        let Held { mut active, changes, terminal, question } = held();
        let (before, next) = (Arc::new(()), Arc::new(()));
        active.search(2, task(&before));
        active.search(2, task(&next));
        released(&before).await;
        assert!(holds(&next) && holds(&changes) && holds(&terminal) && holds(&question));
    }

    #[tokio::test(start_paused = true)]
    async fn stops_the_search_of_its_workspace_only_and_keeps_the_others() {
        let Held { mut active, changes, terminal, question } = held();
        let search = Arc::new(());
        active.search(2, task(&search));
        active.unsearch(3);
        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
        assert!(holds(&search));
        active.unsearch(2);
        released(&search).await;
        assert!(holds(&changes) && holds(&terminal) && holds(&question));
    }

    #[tokio::test(start_paused = true)]
    async fn stops_all_three_when_dropped() {
        let Held { active, changes, terminal, question } = held();
        drop(active);
        released(&changes).await;
        released(&terminal).await;
        released(&question).await;
    }
}
