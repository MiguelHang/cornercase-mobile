use std::fs;
use std::sync::Arc;

use cornercase_bridge::test_util::TempDir;
use cornercase_core::secrets::Secrets;
use rstest::rstest;

use super::*;
use crate::commands::connection::start;
use crate::commands::error::ErrorCode;
use crate::secrets::{mock_store, on_mock};
use crate::state::Active;
use crate::test_util::{
    ADDRESS, BOOT, BRIDGE, Ipc, Recorder, Studio, TOKEN, changed, heard, launcher, released, serving, settle, snapshot,
};
use crate::view::RouteView;

const LAPTOP: &str = "ws://laptop.lan:9000/?token=bGFwdG9wLXNlY3JldA";

fn names(state: &AppState) -> Vec<String> {
    list(state).expect("the machines load").into_iter().map(|machine| machine.name).collect()
}

mod adding_a_machine {
    use super::*;

    #[rstest]
    #[case::not_a_url("studio.lan c3R1ZGlvLXNlY3JldA", ErrorCode::RouteNotAUrl)]
    #[case::plain_http("http://studio.lan:9000/?token=c3R1ZGlvLXNlY3JldA", ErrorCode::RouteScheme)]
    #[case::no_token("ws://studio.lan:9000/", ErrorCode::RouteNoToken)]
    fn says_what_is_wrong_with_the_url_without_repeating_it(#[case] url: &str, #[case] code: ErrorCode) {
        let studio = Studio::new();
        let error = add(&studio.state, url, "laptop".to_owned()).expect_err("the url is refused");
        assert_eq!(error.code, code);
        assert!(!error.message.contains(TOKEN) && !error.message.contains("studio.lan"), "{}", error.message);
        assert_eq!(names(&studio.state), ["studio"]);
    }

    #[test]
    fn says_the_machines_are_not_loaded() {
        let dir = TempDir::new();
        fs::write(dir.path().join("machines.json"), "{ not json").expect("write the machines file");
        let secrets: Arc<dyn Secrets> = Arc::new(on_mock(mock_store()));
        let state = AppState::load(Ok::<_, &str>(dir.path().to_path_buf()), secrets);
        let error = add(&state, BRIDGE, "studio".to_owned()).expect_err("the machines are not loaded");
        assert_eq!(error.code, ErrorCode::MachinesNotLoaded);
    }

    #[test]
    fn lists_it_without_its_token() {
        let studio = Studio::new();
        let machines = list(&studio.state).expect("the machines load");
        let expected = MachineView {
            id: studio.id.0,
            name: "studio".to_owned(),
            routes: vec![RouteView::Bridge { url: ADDRESS.to_owned() }],
            last_route: None,
        };
        assert_eq!(machines, [expected]);
        let json = serde_json::to_string(&machines).expect("the machines serialize");
        assert!(!json.contains(TOKEN), "{json}");
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
        let laptop = add(&studio.state, LAPTOP, "laptop".to_owned()).expect("the laptop is added");
        let recorder = Recorder::default();
        let (launcher, _) = launcher(serving(&[snapshot(BOOT, 7, "first")]));
        start(&studio.state, recorder.clone(), &launcher, studio.id).expect("connected");
        remove(&studio.state, &recorder, laptop).expect("removed");
        assert_eq!(studio.state.active().as_ref().map(Active::machine), Some(studio.id));
        assert!(!recorder.connections().contains(&changed(laptop, ConnectionView::None)));
        assert_eq!(names(&studio.state), ["studio"]);
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
        let machines = ipc.invoke("machines_list", json!({})).expect("the machines load");
        assert_eq!(machines[0]["name"], "studio");
        assert_eq!(machines.as_array().map(Vec::len), Some(1));
    }

    #[test]
    fn adds_a_machine_and_answers_its_id() {
        let studio = Studio::new();
        let ipc = Ipc::new(studio.reloaded());
        let id = ipc.invoke("machine_add", json!({ "url": LAPTOP, "name": "laptop" })).expect("the machine is added");
        assert_eq!(names(&ipc.state()), ["studio", "laptop"]);
        assert_eq!(list(&ipc.state()).expect("the machines load")[1].id, id.as_u64().expect("an id"));
    }

    #[test]
    fn says_why_it_did_not_add_a_machine() {
        let studio = Studio::new();
        let ipc = Ipc::new(studio.reloaded());
        let refused = ipc.invoke("machine_add", json!({ "url": "ws://laptop.lan:9000/", "name": "laptop" }));
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
