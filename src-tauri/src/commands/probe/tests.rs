use std::collections::HashMap;
use std::sync::atomic::Ordering;
use std::sync::{Arc, Mutex, PoisonError};

use cornercase_core::machines::pairing::PairError;
use cornercase_core::machines::route::Token;
use cornercase_core::transport::Transport;
use cornercase_core::transport::fake::FakeTransport;
use rstest::rstest;
use serde_json::json;
use tauri::async_runtime::JoinHandle;

use super::*;
use crate::commands::connection::start;
use crate::commands::error::ErrorCode;
use crate::commands::machines::list;
use crate::commands::routes::merge;
use crate::test_util::{
    ADDRESS, BOOT, BRIDGE, CONTRACT, Ipc, PAIRING, PAIRING_CODE, Recorder, SECRET, Studio, contracted, launcher,
    paired_routes, pairing, route, serving, settle, snapshot,
};

const LAPTOP: &str = "ws://laptop.lan:9000/?token=bGFwdG9wLXNlY3JldA";
const LAPTOP_ADDRESS: &str = "ws://laptop.lan:9000/";
const PROBED: &str = "6262-1770000999";
const TAILSCALE: &str = "ws://100.101.102.103:9000/?token=c3R1ZGlvLXNlY3JldA";

fn answering(studio: FakeTransport, laptop: Result<FakeTransport, Error>) -> Launcher {
    let studio: Arc<dyn Transport> = Arc::new(studio);
    let laptop = laptop.map(|fake| Arc::new(fake) as Arc<dyn Transport>);
    Launcher::new(
        move |route: &Route| {
            if route.connection_url()? == LAPTOP { laptop.clone() } else { Ok(Arc::clone(&studio)) }
        },
        |task| JoinHandle::Tokio(tokio::spawn(task)),
    )
}

fn refusing(error: Error) -> FakeTransport {
    FakeTransport::default().fail(CONTRACT, error)
}

fn asleep() -> Error {
    Error::Connect("the laptop is asleep".to_owned())
}

async fn with_the_laptop() -> Studio {
    let studio = Studio::new();
    merge(&studio.state, Recorder::default(), &answering(contracted(BOOT), Err(asleep())), studio.id, LAPTOP)
        .await
        .expect("the laptop route is added");
    studio
}

fn tokenless(studio: &Studio, address: &str) {
    studio.secrets.delete(&format!("route/{}/{address}", studio.id.0)).expect("the token is forgotten");
}

fn failure(route: &str, error: &Error) -> RouteFailure {
    RouteFailure { route: route.to_owned(), error: error.into() }
}

mod probing_a_machine {
    use super::*;

    #[tokio::test(start_paused = true)]
    async fn says_which_route_answers_and_keeps_its_boot() {
        let studio = Studio::new();
        let launcher = answering(contracted(PROBED), Err(asleep()));
        let view = probe(&studio.state, &launcher, studio.id).await.expect("probed");
        assert_eq!(view, ProbeView::Answers { route: ADDRESS.to_owned() });
        assert_eq!(studio.state.seen_boots(), HashMap::from([(studio.id, PROBED.to_owned())]));
    }

    #[tokio::test(start_paused = true)]
    async fn answers_through_another_route_when_the_first_fails() {
        let studio = with_the_laptop().await;
        let launcher = answering(refusing(asleep()), Ok(contracted(PROBED)));
        let view = probe(&studio.state, &launcher, studio.id).await.expect("probed");
        assert_eq!(view, ProbeView::Answers { route: LAPTOP_ADDRESS.to_owned() });
    }

    #[tokio::test(start_paused = true)]
    async fn keeps_no_boot_when_the_contract_is_unreadable() {
        let studio = Studio::new();
        let garbled = FakeTransport::default().answer(CONTRACT, &[r#"{"boot":"x"}"#], &[], 0);
        let view = probe(&studio.state, &answering(garbled, Err(asleep())), studio.id).await.expect("probed");
        assert_eq!(view, ProbeView::Answers { route: ADDRESS.to_owned() });
        assert_eq!(studio.state.seen_boots(), HashMap::new());
    }

    #[tokio::test(start_paused = true)]
    async fn lists_the_error_of_every_route_when_none_answers() {
        let studio = with_the_laptop().await;
        let launcher = answering(refusing(Error::Timeout), Err(asleep()));
        let view = probe(&studio.state, &launcher, studio.id).await.expect("probed");
        let failures = vec![failure(ADDRESS, &Error::Timeout), failure(LAPTOP_ADDRESS, &asleep())];
        assert_eq!(view, ProbeView::NoAnswer { failures });
        assert_eq!(studio.state.seen_boots(), HashMap::new());
    }

    #[tokio::test(start_paused = true)]
    async fn says_the_token_was_rejected_when_any_route_refuses_it() {
        let studio = with_the_laptop().await;
        let launcher = answering(refusing(Error::Unauthorized), Err(asleep()));
        let view = probe(&studio.state, &launcher, studio.id).await.expect("probed");
        assert_eq!(view, ProbeView::Unauthorized);
    }

    #[tokio::test(start_paused = true)]
    async fn says_it_has_no_token_when_no_route_has_one() {
        let studio = Studio::new();
        tokenless(&studio, ADDRESS);
        let reloaded = studio.reloaded();
        let view = probe(&reloaded, &answering(contracted(PROBED), Err(asleep())), studio.id).await.expect("probed");
        assert_eq!(view, ProbeView::NoToken);
    }

    #[tokio::test(start_paused = true)]
    async fn tells_a_missing_token_apart_when_another_route_fails() {
        let studio = with_the_laptop().await;
        tokenless(&studio, ADDRESS);
        let reloaded = studio.reloaded();
        let view = probe(&reloaded, &answering(contracted(PROBED), Err(asleep())), studio.id).await.expect("probed");
        let failures = vec![failure(ADDRESS, &Error::NoToken), failure(LAPTOP_ADDRESS, &asleep())];
        assert_eq!(view, ProbeView::NoAnswer { failures });
    }

    #[tokio::test(start_paused = true)]
    async fn says_the_machine_is_unknown() {
        let studio = Studio::new();
        let result = probe(&studio.state, &answering(contracted(PROBED), Err(asleep())), MachineId(99)).await;
        assert_eq!(result.map_err(|error| error.code), Err(ErrorCode::UnknownMachine));
    }

    #[rstest]
    #[case::no_routes(PickError::NoRoutes, ProbeView::NoAnswer { failures: Vec::new() })]
    #[case::only_tokenless(
        PickError::AllFailed(vec![(route().address().clone(), Error::NoToken)]),
        ProbeView::NoToken
    )]
    fn reads_a_failed_pick(#[case] error: PickError, #[case] expected: ProbeView) {
        assert_eq!(failed(error), expected);
    }
}

fn same(studio: &Studio, why: SameWhy) -> MatchFound {
    MatchFound::Same { machine: studio.id.0, name: "studio".to_owned(), why }
}

async fn matched(studio: &Studio, launcher: &Launcher, url: &str, name: &str) -> MatchView {
    matching(&studio.state, launcher, url, name).await.expect("matched")
}

mod matching_a_url {
    use super::*;

    #[rstest]
    #[case::by_address("ws://studio.lan:9000/?token=bmV3LXNlY3JldA", "desk", SameWhy::Address)]
    #[case::by_one_lan_address_of_a_code(
        "ws://studio.lan:9000/?token=bmV3LXNlY3JldA&also=desk.lan",
        "desk",
        SameWhy::Address
    )]
    #[case::by_name(LAPTOP, " Studio ", SameWhy::Name)]
    #[tokio::test(start_paused = true)]
    async fn finds_the_same_machine(#[case] url: &str, #[case] name: &str, #[case] why: SameWhy) {
        let studio = Studio::new();
        let view = matched(&studio, &answering(contracted(BOOT), Ok(contracted(PROBED))), url, name).await;
        assert_eq!(view, MatchView { found: same(&studio, why), checked: true });
    }

    #[tokio::test(start_paused = true)]
    async fn finds_it_by_the_boot_of_a_machine_probed_before() {
        let studio = Studio::new();
        let launcher = answering(contracted(PROBED), Ok(contracted(PROBED)));
        probe(&studio.state, &launcher, studio.id).await.expect("probed");
        let view = matched(&studio, &launcher, LAPTOP, "desk").await;
        assert_eq!(view, MatchView { found: same(&studio, SameWhy::Boot), checked: true });
    }

    #[tokio::test(start_paused = true)]
    async fn finds_it_by_the_boot_of_the_active_machine() {
        let studio = Studio::new();
        let (active, _transport) = launcher(serving(&[snapshot(BOOT, 7, "first")]));
        start(&studio.state, Recorder::default(), &active, studio.id).expect("connecting");
        settle().await;
        let view = matched(&studio, &answering(contracted(PROBED), Ok(contracted(BOOT))), LAPTOP, "desk").await;
        assert_eq!(view, MatchView { found: same(&studio, SameWhy::Boot), checked: true });
    }

    #[tokio::test(start_paused = true)]
    async fn leaves_the_boot_out_when_the_url_does_not_answer() {
        let studio = Studio::new();
        let launcher = answering(contracted(PROBED), Err(asleep()));
        probe(&studio.state, &launcher, studio.id).await.expect("probed");
        let view = matched(&studio, &launcher, LAPTOP, "desk").await;
        assert_eq!(view, MatchView { found: MatchFound::None, checked: false });
    }

    #[tokio::test(start_paused = true)]
    async fn finds_none_when_nothing_matches() {
        let studio = Studio::new();
        let view = matched(&studio, &answering(contracted(BOOT), Ok(contracted(PROBED))), LAPTOP, "desk").await;
        assert_eq!(view, MatchView { found: MatchFound::None, checked: true });
    }

    #[tokio::test(start_paused = true)]
    async fn answers_an_address_without_asking_the_url() {
        let studio = Studio::new();
        let url = "ws://studio.lan:9000/?token=bmV3LXNlY3JldA";
        let view = matched(&studio, &answering(refusing(asleep()), Err(asleep())), url, "desk").await;
        assert_eq!(view, MatchView { found: same(&studio, SameWhy::Address), checked: true });
    }

    #[rstest]
    #[case::already_saved(BRIDGE)]
    #[case::by_token("ws://studio.tail:9000/?token=c3R1ZGlvLXNlY3JldA")]
    #[case::by_a_tailscale_address("ws://100.101.102.103:9000/?token=bmV3LXNlY3JldA")]
    #[case::by_every_address_of_the_code("ws://studio.lan:9000/?token=bmV3LXNlY3JldA&also=laptop.lan")]
    #[tokio::test(start_paused = true)]
    async fn merges_into_the_machine_it_surely_is_without_asking_the_url(#[case] code: &str) {
        let studio = with_the_laptop().await;
        let launcher = answering(contracted(BOOT), Err(asleep()));
        merge(&studio.state, Recorder::default(), &launcher, studio.id, TAILSCALE)
            .await
            .expect("the tailscale route is added");
        let view = matched(&studio, &answering(refusing(asleep()), Err(asleep())), code, "desk").await;
        let found = MatchFound::Merge { machine: studio.id.0, name: "studio".to_owned() };
        assert_eq!(view, MatchView { found, checked: true });
    }

    #[tokio::test(start_paused = true)]
    async fn still_asks_the_url_after_a_name_match() {
        let studio = Studio::new();
        let view = matched(&studio, &answering(contracted(BOOT), Err(asleep())), LAPTOP, "Studio").await;
        assert_eq!(view, MatchView { found: same(&studio, SameWhy::Name), checked: false });
    }

    #[tokio::test(start_paused = true)]
    async fn saves_nothing() {
        let studio = Studio::new();
        let before = list(&studio.state).expect("listed");
        matched(&studio, &answering(contracted(BOOT), Ok(contracted(PROBED))), LAPTOP, "desk").await;
        assert_eq!(list(&studio.reloaded()).expect("listed"), before);
        assert_eq!(studio.state.seen_boots(), HashMap::new());
    }

    #[tokio::test(start_paused = true)]
    async fn refuses_a_url_that_is_not_a_route() {
        let studio = Studio::new();
        let launcher = answering(contracted(BOOT), Err(asleep()));
        let result = matching(&studio.state, &launcher, "http://laptop.lan:9000/?token=eA", "desk").await;
        assert_eq!(result.map_err(|error| error.code), Err(ErrorCode::RouteScheme));
    }
}

mod matching_a_pairing_code {
    use super::*;

    const DESK: &str = "ws://desk.lan:47682/?code=0f1e2d3c4b5a69788796a5b4c3d2e1f0";
    const BY_TAILSCALE: &str = "ws://studio.lan:47682/?code=0f1e2d3c4b5a69788796a5b4c3d2e1f0&also=100.101.102.103";

    type Asked = Arc<Mutex<Vec<Route>>>;

    fn asking(fake: FakeTransport) -> (Launcher, Asked) {
        let transport: Arc<dyn Transport> = Arc::new(fake);
        let asked = Asked::default();
        let seen = Arc::clone(&asked);
        let launcher = Launcher::new(
            move |route: &Route| {
                seen.lock().unwrap_or_else(PoisonError::into_inner).push(route.clone());
                Ok(Arc::clone(&transport))
            },
            |task| JoinHandle::Tokio(tokio::spawn(task)),
        );
        (launcher, asked)
    }

    #[tokio::test(start_paused = true)]
    async fn pairs_first_and_asks_the_mac_with_the_secret_it_got() {
        let studio = Studio::new();
        let (launcher, asked) = asking(contracted(PROBED));
        let (launcher, pairings) = pairing(launcher, Ok(SECRET));
        let view = matched(&studio, &launcher, DESK, "desk").await;
        assert_eq!(view, MatchView { found: MatchFound::None, checked: true });
        assert_eq!(pairings.load(Ordering::SeqCst), 1);
        assert_eq!(*asked.lock().unwrap_or_else(PoisonError::into_inner), paired_routes(DESK));
    }

    #[tokio::test(start_paused = true)]
    async fn finds_the_mac_saved_with_routes_of_the_bridge_by_its_host() {
        let studio = Studio::new();
        let (launcher, _) = pairing(answering(refusing(asleep()), Err(asleep())), Ok(SECRET));
        let view = matched(&studio, &launcher, PAIRING, "desk").await;
        assert_eq!(view, MatchView { found: same(&studio, SameWhy::Address), checked: true });
    }

    #[tokio::test(start_paused = true)]
    async fn merges_into_the_mac_saved_with_routes_of_the_bridge() {
        let studio = Studio::new();
        merge(&studio.state, Recorder::default(), &answering(contracted(BOOT), Err(asleep())), studio.id, TAILSCALE)
            .await
            .expect("the tailscale route is added");
        let (launcher, _) = pairing(answering(refusing(asleep()), Err(asleep())), Ok(SECRET));
        let view = matched(&studio, &launcher, BY_TAILSCALE, "desk").await;
        let found = MatchFound::Merge { machine: studio.id.0, name: "studio".to_owned() };
        assert_eq!(view, MatchView { found, checked: true });
    }

    #[tokio::test(start_paused = true)]
    async fn pairs_once_for_a_code_it_already_redeemed() {
        let studio = Studio::new();
        let (launcher, pairings) = pairing(answering(contracted(PROBED), Err(asleep())), Ok(SECRET));
        matched(&studio, &launcher, DESK, "desk").await;
        matched(&studio, &launcher, &format!(" {DESK} "), "desk").await;
        assert_eq!(pairings.load(Ordering::SeqCst), 1);
    }

    #[tokio::test(start_paused = true)]
    async fn keeps_the_secret_in_memory_and_saves_nothing() {
        let studio = Studio::new();
        let before = list(&studio.state).expect("listed");
        let (launcher, _) = pairing(answering(contracted(PROBED), Err(asleep())), Ok(SECRET));
        matched(&studio, &launcher, DESK, "desk").await;
        assert_eq!(list(&studio.reloaded()).expect("listed"), before);
        assert_eq!(studio.state.secret_of(DESK), Some(Token::fake(SECRET)));
    }

    #[tokio::test(start_paused = true)]
    async fn says_why_the_mac_did_not_pair_and_keeps_no_secret() {
        let studio = Studio::new();
        let refused = Err(PairError::Answered(Error::PairingRefused));
        let (launcher, _) = pairing(answering(contracted(PROBED), Err(asleep())), refused);
        let error = matching(&studio.state, &launcher, DESK, "desk").await.expect_err("the code is refused");
        assert_eq!(error.code, ErrorCode::PairingRefused);
        assert!(!error.message.contains(PAIRING_CODE), "{}", error.message);
        assert_eq!(studio.state.secret_of(DESK), None);
    }

    #[tokio::test(start_paused = true)]
    async fn never_pairs_a_code_of_the_bridge() {
        let studio = Studio::new();
        let (launcher, pairings) = pairing(answering(contracted(BOOT), Err(asleep())), Ok(SECRET));
        matched(&studio, &launcher, LAPTOP, "desk").await;
        assert_eq!(pairings.load(Ordering::SeqCst), 0);
        assert_eq!(studio.state.secret_of(LAPTOP), None);
    }
}

mod the_boots_seen {
    use super::*;

    #[tokio::test(start_paused = true)]
    async fn include_the_one_the_active_machine_reports() {
        let studio = Studio::new();
        let (launcher, _transport) = launcher(serving(&[snapshot(BOOT, 7, "first")]));
        start(&studio.state, Recorder::default(), &launcher, studio.id).expect("connecting");
        settle().await;
        assert_eq!(studio.state.seen_boots(), HashMap::from([(studio.id, BOOT.to_owned())]));
    }

    #[tokio::test(start_paused = true)]
    async fn prefer_the_active_machine_to_an_older_probe() {
        let studio = Studio::new();
        probe(&studio.state, &answering(contracted(PROBED), Err(asleep())), studio.id).await.expect("probed");
        let (launcher, _transport) = launcher(serving(&[snapshot(BOOT, 7, "first")]));
        start(&studio.state, Recorder::default(), &launcher, studio.id).expect("connecting");
        settle().await;
        assert_eq!(studio.state.seen_boots(), HashMap::from([(studio.id, BOOT.to_owned())]));
    }

    #[test]
    fn are_none_at_first() {
        assert_eq!(Studio::new().state.seen_boots(), HashMap::new());
    }
}

mod over_ipc {
    use super::*;

    #[test]
    fn says_a_machine_without_a_token_has_none() {
        let studio = Studio::new();
        tokenless(&studio, ADDRESS);
        let ipc = Ipc::new(studio.reloaded());
        let answer = ipc.invoke("machine_probe", json!({ "id": studio.id.0 }));
        assert_eq!(answer, Ok(json!({ "state": "no_token" })));
    }

    #[test]
    fn refuses_to_match_a_url_without_a_token() {
        let ipc = Ipc::new(Studio::new().state);
        let answer = ipc.invoke("machine_match", json!({ "code": ADDRESS, "name": "desk" })).expect_err("no token");
        assert_eq!(answer["code"], "route_no_token");
    }

    #[test]
    fn refuses_an_unknown_machine() {
        let ipc = Ipc::new(Studio::new().state);
        let answer = ipc.invoke("machine_probe", json!({ "id": 99 })).expect_err("unknown");
        assert_eq!(answer["code"], "unknown_machine");
    }
}
