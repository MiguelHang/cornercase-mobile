use std::sync::{Arc, Mutex, PoisonError};

use cornercase_core::network::{Kind, Network};
use cornercase_core::transport::Transport;
use rstest::rstest;
use serde_json::{Value, json};
use tauri::async_runtime::JoinHandle;

use super::*;
use crate::commands::error::ErrorCode;
use crate::commands::machines::{add, list};
use crate::test_util::{ADDRESS, BOOT, BRIDGE, Ipc, Recorder, Studio, changed, connected, serving, settle, snapshot};
use crate::view::{ConnectionView, RouteView};

const LAPTOP: &str = "ws://laptop.lan:9000/?token=bGFwdG9wLXNlY3JldA";
const LAPTOP_ADDRESS: &str = "ws://laptop.lan:9000/";
const NEW_TOKEN: &str = "ws://studio.lan:9000/?token=bmV3LXNlY3JldA";

type Asked = Arc<Mutex<Vec<String>>>;

fn recording() -> (Launcher, Asked) {
    let transport: Arc<dyn Transport> = Arc::new(serving(&[snapshot(BOOT, 7, "first")]));
    let asked = Arc::new(Mutex::new(Vec::new()));
    let seen = Arc::clone(&asked);
    let launcher = Launcher::new(
        move |route: &Route| {
            let url = route.connection_url()?;
            seen.lock().unwrap_or_else(PoisonError::into_inner).push(url.clone());
            if url == LAPTOP {
                Err(Error::Connect("the laptop is asleep".to_owned()))
            } else {
                Ok(Arc::clone(&transport))
            }
        },
        |task| JoinHandle::Tokio(tokio::spawn(task)),
    );
    (launcher, asked)
}

fn urls_asked(asked: &Asked) -> Vec<String> {
    asked.lock().unwrap_or_else(PoisonError::into_inner).clone()
}

fn urls(state: &AppState, id: MachineId) -> Vec<String> {
    let machines = list(state).expect("the machines load").machines;
    let machine = machines.into_iter().find(|machine| machine.id == id.0).expect("the machine is listed");
    machine.routes.into_iter().map(|RouteView::Bridge { url, .. }| url).collect()
}

fn twice_connected(studio: &Studio) -> Vec<crate::events::ConnectionChanged> {
    let first = [changed(studio.id, ConnectionView::Connecting), changed(studio.id, connected())];
    first.iter().chain(&first).cloned().collect()
}

fn once_connected(studio: &Studio) -> Vec<crate::events::ConnectionChanged> {
    vec![changed(studio.id, ConnectionView::Connecting), changed(studio.id, connected())]
}

async fn connected_with_the_laptop(studio: &Studio, recorder: &Recorder) -> Asked {
    add_route(&studio.state, recorder.clone(), &recording().0, studio.id, LAPTOP).expect("the laptop route is added");
    let (launcher, asked) = recording();
    start(&studio.state, recorder.clone(), &launcher, studio.id).expect("connecting");
    settle().await;
    asked
}

mod adding_a_route {
    use super::*;

    #[tokio::test(start_paused = true)]
    async fn to_a_machine_that_is_not_connected_saves_it_without_connecting() {
        let studio = Studio::new();
        let (launcher, asked) = recording();
        add_route(&studio.state, Recorder::default(), &launcher, studio.id, LAPTOP).expect("added");
        assert_eq!(urls(&studio.reloaded(), studio.id), [ADDRESS, LAPTOP_ADDRESS]);
        assert!(studio.state.active().is_none());
        assert_eq!(urls_asked(&asked), Vec::<String>::new());
    }

    #[tokio::test(start_paused = true)]
    async fn to_another_machine_keeps_the_connection() {
        let studio = Studio::new();
        let laptop = add(&studio.state, LAPTOP, "laptop").expect("the laptop is added");
        let recorder = Recorder::default();
        let (launcher, asked) = recording();
        start(&studio.state, recorder.clone(), &launcher, studio.id).expect("connecting");
        settle().await;
        add_route(&studio.state, recorder.clone(), &launcher, laptop, NEW_TOKEN).expect("added");
        settle().await;
        assert_eq!(urls_asked(&asked), [BRIDGE]);
        assert_eq!(recorder.connections(), once_connected(&studio));
    }

    #[tokio::test(start_paused = true)]
    async fn to_the_connected_machine_connects_again_with_both_routes() {
        let studio = Studio::new();
        let recorder = Recorder::default();
        let (launcher, asked) = recording();
        start(&studio.state, recorder.clone(), &launcher, studio.id).expect("connecting");
        settle().await;
        add_route(&studio.state, recorder.clone(), &launcher, studio.id, LAPTOP).expect("added");
        settle().await;
        assert_eq!(urls_asked(&asked), [BRIDGE, BRIDGE, LAPTOP]);
        assert_eq!(recorder.connections(), twice_connected(&studio));
    }

    #[tokio::test(start_paused = true)]
    async fn with_a_new_token_for_the_connected_machine_connects_again_with_it() {
        let studio = Studio::new();
        let (launcher, asked) = recording();
        start(&studio.state, Recorder::default(), &launcher, studio.id).expect("connecting");
        settle().await;
        add_route(&studio.state, Recorder::default(), &launcher, studio.id, NEW_TOKEN).expect("replaced");
        settle().await;
        assert_eq!(urls_asked(&asked), [BRIDGE, NEW_TOKEN]);
        assert_eq!(urls(&studio.reloaded(), studio.id), [ADDRESS]);
    }

    #[rstest]
    #[case::without_a_token("ws://laptop.lan:9000/", 0, ErrorCode::RouteNoToken)]
    #[case::to_an_unknown_machine(LAPTOP, 99, ErrorCode::UnknownMachine)]
    #[tokio::test(start_paused = true)]
    async fn says_why_it_did_not(#[case] url: &str, #[case] id: u64, #[case] code: ErrorCode) {
        let studio = Studio::new();
        let id = if id == 0 { studio.id } else { MachineId(id) };
        let (launcher, asked) = recording();
        let error = add_route(&studio.state, Recorder::default(), &launcher, id, url).expect_err("refused");
        assert_eq!(error.code, code);
        assert_eq!(urls(&studio.state, studio.id), [ADDRESS]);
        assert_eq!(urls_asked(&asked), Vec::<String>::new());
    }
}

mod removing_a_route {
    use super::*;

    #[tokio::test(start_paused = true)]
    async fn the_connection_does_not_use_keeps_it() {
        let studio = Studio::new();
        let recorder = Recorder::default();
        let asked = connected_with_the_laptop(&studio, &recorder).await;
        remove_route(&studio.state, recorder.clone(), &recording().0, studio.id, LAPTOP_ADDRESS).expect("removed");
        settle().await;
        assert_eq!(urls(&studio.reloaded(), studio.id), [ADDRESS]);
        assert_eq!(urls_asked(&asked), [BRIDGE, LAPTOP]);
        assert_eq!(recorder.connections(), once_connected(&studio));
    }

    #[tokio::test(start_paused = true)]
    async fn the_connection_uses_connects_again_through_the_others() {
        let studio = Studio::new();
        let recorder = Recorder::default();
        connected_with_the_laptop(&studio, &recorder).await;
        let (launcher, asked) = recording();
        remove_route(&studio.state, recorder.clone(), &launcher, studio.id, ADDRESS).expect("removed");
        settle().await;
        assert_eq!(urls(&studio.reloaded(), studio.id), [LAPTOP_ADDRESS]);
        assert_eq!(urls_asked(&asked).first().map(String::as_str), Some(LAPTOP));
        assert_eq!(recorder.connections()[2], changed(studio.id, ConnectionView::Connecting));
    }

    #[tokio::test(start_paused = true)]
    async fn while_it_is_not_connected_connects_again() {
        let studio = Studio::new();
        studio.state.replace_network(Network { kind: Kind::None, expensive: false });
        let recorder = Recorder::default();
        connected_with_the_laptop(&studio, &recorder).await;
        remove_route(&studio.state, recorder.clone(), &recording().0, studio.id, LAPTOP_ADDRESS).expect("removed");
        settle().await;
        let offline = changed(studio.id, ConnectionView::Offline);
        assert_eq!(recorder.connections(), [offline.clone(), offline]);
    }

    #[tokio::test(start_paused = true)]
    async fn of_another_machine_keeps_the_connection() {
        let studio = Studio::new();
        let laptop = add(&studio.state, LAPTOP, "laptop").expect("the laptop is added");
        add_route(&studio.state, Recorder::default(), &recording().0, laptop, NEW_TOKEN).expect("added");
        let recorder = Recorder::default();
        start(&studio.state, recorder.clone(), &recording().0, studio.id).expect("connecting");
        settle().await;
        remove_route(&studio.state, recorder.clone(), &recording().0, laptop, ADDRESS).expect("removed");
        settle().await;
        assert_eq!(urls(&studio.state, laptop), [LAPTOP_ADDRESS]);
        assert_eq!(recorder.connections(), once_connected(&studio));
    }

    #[rstest]
    #[case::the_last_one(ADDRESS, 0, ErrorCode::LastRoute)]
    #[case::unknown(LAPTOP_ADDRESS, 0, ErrorCode::UnknownRoute)]
    #[case::with_its_token(BRIDGE, 0, ErrorCode::UnknownRoute)]
    #[case::of_an_unknown_machine(ADDRESS, 99, ErrorCode::UnknownMachine)]
    #[tokio::test(start_paused = true)]
    async fn says_why_it_did_not(#[case] url: &str, #[case] id: u64, #[case] code: ErrorCode) {
        let studio = Studio::new();
        let id = if id == 0 { studio.id } else { MachineId(id) };
        let error = remove_route(&studio.state, Recorder::default(), &recording().0, id, url).expect_err("refused");
        assert_eq!(error.code, code);
        assert_eq!(urls(&studio.state, studio.id), [ADDRESS]);
    }
}

mod moving_a_route {
    use super::*;

    #[rstest]
    #[case::up(LAPTOP_ADDRESS, Direction::Up, [LAPTOP_ADDRESS, ADDRESS])]
    #[case::down(ADDRESS, Direction::Down, [LAPTOP_ADDRESS, ADDRESS])]
    #[case::up_from_the_top(ADDRESS, Direction::Up, [ADDRESS, LAPTOP_ADDRESS])]
    #[tokio::test(start_paused = true)]
    async fn saves_the_order_and_keeps_the_connection(
        #[case] url: &str,
        #[case] direction: Direction,
        #[case] expected: [&str; 2],
    ) {
        let studio = Studio::new();
        let recorder = Recorder::default();
        connected_with_the_laptop(&studio, &recorder).await;
        move_route(&studio.state, studio.id, url, direction).expect("moved");
        settle().await;
        assert_eq!(urls(&studio.reloaded(), studio.id), expected);
        assert_eq!(recorder.connections(), once_connected(&studio));
    }

    #[rstest]
    #[case::unknown(LAPTOP_ADDRESS, 0, ErrorCode::UnknownRoute)]
    #[case::of_an_unknown_machine(ADDRESS, 99, ErrorCode::UnknownMachine)]
    fn says_why_it_did_not(#[case] url: &str, #[case] id: u64, #[case] code: ErrorCode) {
        let studio = Studio::new();
        let id = if id == 0 { studio.id } else { MachineId(id) };
        let error = move_route(&studio.state, id, url, Direction::Down).expect_err("refused");
        assert_eq!(error.code, code);
    }
}

mod over_ipc {
    use super::*;

    #[test]
    fn adds_moves_and_removes_a_route_of_a_machine_that_is_not_connected() {
        let studio = Studio::new();
        let ipc = Ipc::new(studio.reloaded());
        let id = studio.id.0;
        assert_eq!(ipc.invoke("machine_route_add", json!({ "id": id, "url": LAPTOP })), Ok(Value::Null));
        let moved = ipc.invoke("machine_route_move", json!({ "id": id, "url": LAPTOP_ADDRESS, "direction": "up" }));
        assert_eq!(moved, Ok(Value::Null));
        assert_eq!(urls(&ipc.state(), studio.id), [LAPTOP_ADDRESS, ADDRESS]);
        assert_eq!(ipc.invoke("machine_route_remove", json!({ "id": id, "url": ADDRESS })), Ok(Value::Null));
        assert_eq!(urls(&ipc.state(), studio.id), [LAPTOP_ADDRESS]);
    }

    #[test]
    fn says_why_it_did_not_remove_a_route() {
        let studio = Studio::new();
        let ipc = Ipc::new(studio.reloaded());
        let refused = ipc.invoke("machine_route_remove", json!({ "id": studio.id.0, "url": ADDRESS }));
        assert_eq!(refused.expect_err("the last route stays")["code"], "last_route");
    }
}
