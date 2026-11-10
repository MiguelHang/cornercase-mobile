use std::fs;
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Mutex, PoisonError};
use std::thread;

use cornercase_bridge::test_util::TempDir;
use cornercase_core::error::Error;
use cornercase_core::machines::route::Route;
use cornercase_core::secrets::Secrets;
use rstest::rstest;
use tauri::async_runtime::{self, JoinHandle};

use super::*;
use crate::commands::connection::{Launcher, Task, start};
use crate::commands::error::ErrorCode;
use crate::secrets::{mock_store, on_mock};
use crate::state::Active;
use crate::test_util::{
    ADDRESS, BOOT, BRIDGE, Ipc, Recorder, Studio, TOKEN, changed, heard, launcher, released, serving, settle, snapshot,
};
use crate::view::{MachineView, RouteView};

const LAPTOP: &str = "ws://laptop.lan:9000/?token=bGFwdG9wLXNlY3JldA";
const LAPTOP_CODE: &str = "ws://laptop.lan:9000/?token=bGFwdG9wLXNlY3JldA&also=100.101.102.103&name=Laptop";
const LAPTOP_TOKEN: &str = "bGFwdG9wLXNlY3JldA";

static GATE: Mutex<Option<(Sender<()>, Receiver<()>)>> = Mutex::new(None);

fn gated(task: Task) -> JoinHandle<()> {
    drop(task);
    let gate = GATE.lock().unwrap_or_else(PoisonError::into_inner).take();
    if let Some((entered, go)) = gate {
        entered.send(()).expect("the test waits for the connect");
        go.recv().expect("the test lets the connect go on");
    }
    async_runtime::spawn(async {})
}

fn names(state: &AppState) -> Vec<String> {
    list(state).expect("the machines load").machines.into_iter().map(|machine| machine.name).collect()
}

mod adding_a_machine {
    use super::*;

    #[rstest]
    #[case::not_a_url("studio.lan c3R1ZGlvLXNlY3JldA", ErrorCode::RouteNotAUrl)]
    #[case::plain_http("http://studio.lan:9000/?token=c3R1ZGlvLXNlY3JldA", ErrorCode::RouteScheme)]
    #[case::no_token("ws://studio.lan:9000/", ErrorCode::RouteNoToken)]
    fn says_what_is_wrong_with_the_url_without_repeating_it(#[case] url: &str, #[case] code: ErrorCode) {
        let studio = Studio::new();
        let error = add(&studio.state, url, "laptop").expect_err("the url is refused");
        assert_eq!(error.code, code);
        assert!(!error.message.contains(TOKEN) && !error.message.contains("studio.lan"), "{}", error.message);
        assert_eq!(names(&studio.state), ["studio"]);
    }

    #[test]
    fn says_the_machines_are_not_loaded() {
        let dir = TempDir::new();
        fs::write(dir.path().join("machines.json"), "{ not json").expect("write the machines file");
        let secrets: Arc<dyn Secrets> = Arc::new(on_mock(mock_store()));
        let state = AppState::load(Ok::<_, &str>(dir.path().to_path_buf()), secrets.as_ref(), Arc::clone(&secrets));
        let error = add(&state, BRIDGE, "studio").expect_err("the machines are not loaded");
        assert_eq!(error.code, ErrorCode::MachinesNotLoaded);
    }

    #[test]
    fn saves_every_route_of_a_code_under_one_machine_with_its_token() {
        let studio = Studio::new();
        let laptop = add(&studio.state, LAPTOP_CODE, "laptop").expect("the laptop is added");
        let machines = list(&studio.reloaded()).expect("the machines load").machines;
        let routes = ["ws://laptop.lan:9000/", "ws://100.101.102.103:9000/"];
        let views = routes.map(|url| RouteView::Bridge { url: url.to_owned(), has_token: true });
        assert_eq!(names(&studio.state), ["studio", "laptop"]);
        assert_eq!(machines[1].routes, views);
        for url in routes {
            let saved = studio.secrets.get(&format!("route/{}/{url}", laptop.0)).expect("the keychain answers");
            assert_eq!(saved.as_deref(), Some(LAPTOP_TOKEN.as_bytes()));
        }
    }

    #[test]
    fn lists_it_without_its_token() {
        let studio = Studio::new();
        let machines = list(&studio.state).expect("the machines load").machines;
        let expected = MachineView {
            id: studio.id.0,
            name: "studio".to_owned(),
            routes: vec![RouteView::Bridge { url: ADDRESS.to_owned(), has_token: true }],
            last_route: None,
        };
        assert_eq!(machines, [expected]);
        let json = serde_json::to_string(&machines).expect("the machines serialize");
        assert!(!json.contains(TOKEN), "{json}");
    }
}

mod listing_the_machines {
    use super::*;

    fn active(state: &AppState) -> Option<u64> {
        list(state).expect("the machines load").active
    }

    #[test]
    fn names_the_saved_active_machine() {
        let studio = Studio::new();
        let laptop = add(&studio.state, LAPTOP, "laptop").expect("the laptop is added");
        studio.state.machines(|machines| machines.activate(laptop).map_err(CommandError::from)).expect("activated");
        assert_eq!(active(&studio.reloaded()), Some(laptop.0));
    }

    #[test]
    fn names_the_first_machine_while_none_was_activated() {
        let studio = Studio::new();
        add(&studio.state, LAPTOP, "laptop").expect("the laptop is added");
        assert_eq!(active(&studio.state), Some(studio.id.0));
    }

    #[test]
    fn names_none_without_machines() {
        let studio = Studio::new();
        remove(&studio.state, &Recorder::default(), studio.id).expect("removed");
        assert_eq!(list(&studio.state), Ok(MachineList { machines: vec![], active: None }));
    }
}

mod renaming_a_machine {
    use super::*;

    #[test]
    fn saves_the_trimmed_name() {
        let studio = Studio::new();
        rename(&studio.state, studio.id, "  Studio Mac ").expect("renamed");
        assert_eq!(names(&studio.reloaded()), ["Studio Mac"]);
    }

    #[test]
    fn keeps_its_own_name_in_other_case() {
        let studio = Studio::new();
        rename(&studio.state, studio.id, "STUDIO").expect("renamed");
        assert_eq!(names(&studio.reloaded()), ["STUDIO"]);
    }

    #[rstest]
    #[case::empty("   ", 0, ErrorCode::NameEmpty)]
    #[case::too_long(&"m".repeat(65), 0, ErrorCode::NameTooLong)]
    #[case::taken(" LAPTOP ", 0, ErrorCode::NameTaken)]
    #[case::unknown("mini", 99, ErrorCode::UnknownMachine)]
    fn says_why_it_did_not(#[case] name: &str, #[case] id: u64, #[case] code: ErrorCode) {
        let studio = Studio::new();
        add(&studio.state, LAPTOP, "laptop").expect("the laptop is added");
        let id = if id == 0 { studio.id } else { MachineId(id) };
        let error = rename(&studio.state, id, name).expect_err("refused");
        assert_eq!(error.code, code);
        assert_eq!(names(&studio.reloaded()), ["studio", "laptop"]);
    }
}

mod removing_a_machine {
    use super::*;

    #[tokio::test(start_paused = true)]
    async fn ends_its_store_when_it_is_the_active_one() {
        let studio = Studio::new();
        let recorder = Recorder::default();
        let (launcher, transport) = launcher(serving(&[snapshot(BOOT, 7, "first")]));
        start(&studio.state, recorder.clone(), &launcher, studio.id).expect("connected");
        drop(launcher);
        settle().await;
        remove(&studio.state, &recorder, studio.id).expect("removed");
        released(&transport).await;
        assert!(studio.state.active().is_none());
        assert_eq!(recorder.connections().last(), Some(&changed(studio.id, ConnectionView::None)));
        assert_eq!(names(&studio.state), Vec::<String>::new());
    }

    #[tokio::test(start_paused = true)]
    async fn keeps_the_store_of_another_machine() {
        let studio = Studio::new();
        let laptop = add(&studio.state, LAPTOP, "laptop").expect("the laptop is added");
        let recorder = Recorder::default();
        let (launcher, _) = launcher(serving(&[snapshot(BOOT, 7, "first")]));
        start(&studio.state, recorder.clone(), &launcher, studio.id).expect("connected");
        remove(&studio.state, &recorder, laptop).expect("removed");
        assert_eq!(studio.state.active().as_ref().map(Active::machine), Some(studio.id));
        assert!(!recorder.connections().contains(&changed(laptop, ConnectionView::None)));
        assert_eq!(names(&studio.state), ["studio"]);
    }

    #[test]
    fn while_it_connects_ends_the_new_store() {
        let studio = Studio::new();
        let recorder = Recorder::default();
        let (entered, inside) = mpsc::channel();
        let (go, waiting) = mpsc::channel();
        *GATE.lock().unwrap_or_else(PoisonError::into_inner) = Some((entered, waiting));
        let launcher = Launcher::new(|_: &Route| Err(Error::UnknownMachine), gated);
        thread::scope(|scope| {
            scope.spawn(|| start(&studio.state, Recorder::default(), &launcher, studio.id).expect("connecting"));
            inside.recv().expect("the connect has read the machine");
            scope.spawn(|| remove(&studio.state, &recorder, studio.id).expect("removed"));
            while !names(&studio.state).is_empty() {
                thread::yield_now();
            }
            go.send(()).expect("the connect waits");
        });
        assert!(studio.state.active().is_none());
        assert_eq!(recorder.connections(), [changed(studio.id, ConnectionView::None)]);
    }

    #[test]
    fn is_fine_with_an_unknown_id() {
        let studio = Studio::new();
        let recorder = Recorder::default();
        remove(&studio.state, &recorder, MachineId(99)).expect("nothing to remove");
        assert_eq!(recorder.all(), []);
        assert_eq!(names(&studio.state), ["studio"]);
    }
}

mod over_ipc {
    use serde_json::{Value, json};

    use super::*;

    #[test]
    fn lists_the_machines() {
        let studio = Studio::new();
        let ipc = Ipc::new(studio.reloaded());
        let listed = ipc.invoke("machines_list", json!({})).expect("the machines load");
        assert_eq!(listed["machines"][0]["name"], "studio");
        assert_eq!(listed["machines"].as_array().map(Vec::len), Some(1));
        assert_eq!(listed["active"], studio.id.0);
    }

    #[test]
    fn adds_a_machine_and_answers_its_id() {
        let studio = Studio::new();
        let ipc = Ipc::new(studio.reloaded());
        let id = ipc.invoke("machine_add", json!({ "code": LAPTOP, "name": "laptop" })).expect("the machine is added");
        assert_eq!(names(&ipc.state()), ["studio", "laptop"]);
        assert_eq!(list(&ipc.state()).expect("the machines load").machines[1].id, id.as_u64().expect("an id"));
    }

    #[test]
    fn renames_a_machine() {
        let studio = Studio::new();
        let ipc = Ipc::new(studio.reloaded());
        assert_eq!(ipc.invoke("machine_rename", json!({ "id": studio.id.0, "name": "mini" })), Ok(Value::Null));
        assert_eq!(names(&ipc.state()), ["mini"]);
        let refused = ipc.invoke("machine_rename", json!({ "id": studio.id.0, "name": "" }));
        assert_eq!(refused.expect_err("the name is refused")["code"], "name_empty");
    }

    #[test]
    fn says_why_it_did_not_add_a_machine() {
        let studio = Studio::new();
        let ipc = Ipc::new(studio.reloaded());
        let refused = ipc.invoke("machine_add", json!({ "code": "ws://laptop.lan:9000/", "name": "laptop" }));
        assert_eq!(refused.expect_err("the url is refused")["code"], "route_no_token");
    }

    #[tokio::test(start_paused = true)]
    async fn removes_the_active_machine_and_says_it_is_no_longer_connected() {
        let studio = Studio::new();
        let state = studio.reloaded();
        let (launcher, _) = launcher(serving(&[snapshot(BOOT, 7, "first")]));
        start(&state, Recorder::default(), &launcher, studio.id).expect("connecting");
        let ipc = Ipc::new(state);
        let connections = heard(&ipc.app, "connection-changed");
        assert_eq!(ipc.invoke("machine_remove", json!({ "id": studio.id.0 })), Ok(Value::Null));
        assert_eq!(names(&ipc.state()), Vec::<String>::new());
        let expected = json!({ "machine": studio.id.0, "connection": { "state": "none" } });
        assert_eq!(connections.try_recv(), Ok(expected));
    }
}
