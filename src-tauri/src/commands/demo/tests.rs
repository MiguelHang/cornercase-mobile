use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;

use cornercase_core::error::Error;
use cornercase_core::machines::MachineId;
use cornercase_core::network::{Kind, Network};
use cornercase_core::secrets::Secrets;
use cornercase_core::secrets::file::FileSecrets;
use cornercase_core::test_util::TempDir;

use super::*;
use crate::commands::connection::{current, latest_agents, switch_to};
use crate::commands::error::CommandError;
use crate::commands::machines::{list, remove_or_leave};
use crate::test_util::{BOOT, Recorder, Studio, changed, launcher, route, serving, settle, snapshot, spawned};
use crate::view::MachineView;

const NONE: Network = Network { kind: Kind::None, expensive: false };

fn studio_serving() -> Launcher {
    launcher(serving(&[snapshot(BOOT, 7, "first")])).0
}

fn connected_studio() -> Studio {
    let studio = Studio::new();
    start(&studio.state, Recorder::default(), &studio_serving(), studio.id).expect("the studio connects");
    studio
}

fn active_machine(state: &AppState) -> Option<MachineId> {
    state.active().as_ref().map(Active::machine)
}

fn is_connected(state: &AppState) -> bool {
    matches!(current(state).connection, ConnectionView::Connected { .. })
}

mod entering {
    use super::*;

    #[tokio::test(start_paused = true)]
    async fn connects_to_the_demo_with_the_network_down() {
        let studio = Studio::new();
        studio.state.replace_network(NONE);
        let recorder = Recorder::default();

        enter(&studio.state, recorder.clone(), spawned);
        settle().await;

        assert_eq!(current(&studio.state).machine, Some(DEMO.0));
        assert!(is_connected(&studio.state), "{:?}", current(&studio.state));
        assert_eq!(recorder.connections().first(), Some(&changed(DEMO, ConnectionView::Connecting)));
        assert_ne!(latest_agents(&studio.state).unwrap_or_default(), []);
    }

    #[tokio::test(start_paused = true)]
    async fn neither_remembers_nor_asks_for_the_demo_routes() {
        let studio = Studio::new();
        let recorder = Recorder::default();
        enter(&studio.state, recorder.clone(), spawned);
        settle().await;
        assert!(is_connected(&studio.state), "{:?}", current(&studio.state));
        assert_eq!(recorder.toasts(), []);
    }

    #[tokio::test(start_paused = true)]
    async fn takes_the_place_of_the_active_machine() {
        let studio = connected_studio();
        enter(&studio.state, Recorder::default(), spawned);
        assert_eq!(active_machine(&studio.state), Some(DEMO));
        assert!(studio.state.active().as_ref().is_some_and(Active::is_demo));
    }

    #[test]
    fn uses_the_largest_id_the_app_shows() {
        assert_eq!(DEMO.0, u64::from(u32::MAX));
    }
}

mod leaving {
    use super::*;

    #[tokio::test(start_paused = true)]
    async fn goes_back_to_the_machine_active_before() {
        let studio = connected_studio();
        enter(&studio.state, Recorder::default(), spawned);
        settle().await;

        leave(&studio.state, Recorder::default(), &studio_serving()).expect("the demo is left");
        settle().await;

        assert_eq!(active_machine(&studio.state), Some(studio.id));
        assert!(is_connected(&studio.state), "{:?}", current(&studio.state));
    }

    #[tokio::test(start_paused = true)]
    async fn after_entering_twice_still_goes_back_to_the_machine_before() {
        let studio = connected_studio();
        enter(&studio.state, Recorder::default(), spawned);
        enter(&studio.state, Recorder::default(), spawned);
        leave(&studio.state, Recorder::default(), &studio_serving()).expect("the demo is left");
        assert_eq!(active_machine(&studio.state), Some(studio.id));
    }

    #[tokio::test(start_paused = true)]
    async fn ends_with_no_machine_when_there_was_none() {
        let studio = Studio::new();
        let recorder = Recorder::default();
        enter(&studio.state, Recorder::default(), spawned);

        leave(&studio.state, recorder.clone(), &studio_serving()).expect("the demo is left");

        assert_eq!(active_machine(&studio.state), None);
        assert_eq!(recorder.connections(), [changed(DEMO, ConnectionView::None)]);
    }

    #[tokio::test(start_paused = true)]
    async fn ends_with_no_machine_when_the_one_before_is_gone() {
        let studio = connected_studio();
        enter(&studio.state, Recorder::default(), spawned);
        studio.state.machines(|machines| machines.remove(studio.id).map_err(CommandError::from)).expect("removed");
        leave(&studio.state, Recorder::default(), &studio_serving()).expect("the demo is left");
        assert_eq!(active_machine(&studio.state), None);
    }

    #[tokio::test(start_paused = true)]
    async fn does_nothing_outside_the_demo() {
        let studio = connected_studio();
        let recorder = Recorder::default();
        leave(&studio.state, recorder.clone(), &studio_serving()).expect("nothing to leave");
        assert_eq!(active_machine(&studio.state), Some(studio.id));
        assert_eq!(recorder.all(), []);
    }
}

mod in_the_machines {
    use super::*;

    #[tokio::test(start_paused = true)]
    async fn the_demo_is_listed_last_as_the_active_one_with_no_route() {
        let studio = connected_studio();
        enter(&studio.state, Recorder::default(), spawned);

        let listed = list(&studio.state).expect("the machines load");

        let demo =
            MachineView { id: DEMO.0, name: "Demo Mac".to_owned(), routes: vec![], last_route: None, demo: true };
        assert_eq!(listed.active, Some(DEMO.0));
        assert_eq!(listed.machines.iter().map(|machine| machine.demo).collect::<Vec<_>>(), [false, true]);
        assert_eq!(listed.machines.last(), Some(&demo));
    }

    #[tokio::test(start_paused = true)]
    async fn no_demo_is_listed_outside_it() {
        let studio = connected_studio();
        let listed = list(&studio.state).expect("the machines load");
        assert_eq!(listed.machines.iter().map(|machine| machine.id).collect::<Vec<_>>(), [studio.id.0]);
        assert_ne!(listed.active, Some(DEMO.0));
    }

    #[tokio::test(start_paused = true)]
    async fn removing_the_demo_leaves_it_and_keeps_the_machines_as_they_were() {
        let studio = connected_studio();
        let before = list(&studio.state).expect("the machines load");
        enter(&studio.state, Recorder::default(), spawned);

        remove_or_leave(&studio.state, Recorder::default(), &studio_serving(), DEMO).expect("the demo is left");

        assert_eq!(list(&studio.state).expect("the machines load"), before);
        assert_eq!(active_machine(&studio.state), Some(studio.id));
    }

    #[tokio::test(start_paused = true)]
    async fn removing_another_machine_still_removes_it() {
        let studio = connected_studio();
        remove_or_leave(&studio.state, Recorder::default(), &studio_serving(), studio.id).expect("removed");
        assert_eq!(list(&studio.state).expect("the machines load").machines, []);
        assert_eq!(active_machine(&studio.state), None);
    }
}

mod connecting {
    use super::*;

    #[tokio::test(start_paused = true)]
    async fn to_another_machine_leaves_the_demo() {
        let studio = Studio::new();
        enter(&studio.state, Recorder::default(), spawned);

        switch_to(&studio.state, Recorder::default(), &studio_serving(), studio.id).expect("the studio connects");

        assert_eq!(active_machine(&studio.state), Some(studio.id));
        assert!(!studio.state.in_demo());
        let recorder = Recorder::default();
        leave(&studio.state, recorder.clone(), &studio_serving()).expect("nothing to leave");
        assert_eq!(recorder.all(), []);
        assert_eq!(active_machine(&studio.state), Some(studio.id));
    }

    #[tokio::test(start_paused = true)]
    async fn to_the_demo_while_in_it_stays_there() {
        let studio = connected_studio();
        enter(&studio.state, Recorder::default(), spawned);
        let recorder = Recorder::default();

        switch_to(&studio.state, recorder.clone(), &studio_serving(), DEMO).expect("still in the demo");

        assert_eq!(active_machine(&studio.state), Some(DEMO));
        assert_eq!(recorder.all(), []);
        leave(&studio.state, Recorder::default(), &studio_serving()).expect("the demo is left");
        assert_eq!(active_machine(&studio.state), Some(studio.id));
    }

    #[tokio::test(start_paused = true)]
    async fn to_the_demo_outside_it_finds_no_such_machine() {
        let studio = connected_studio();
        let error = switch_to(&studio.state, Recorder::default(), &studio_serving(), DEMO).expect_err("no demo");
        assert_eq!(error, CommandError::from(Error::UnknownMachine));
        assert_eq!(active_machine(&studio.state), Some(studio.id));
    }

    #[tokio::test(start_paused = true)]
    async fn to_an_unknown_machine_stays_in_the_demo() {
        let studio = Studio::new();
        enter(&studio.state, Recorder::default(), spawned);
        switch_to(&studio.state, Recorder::default(), &studio_serving(), MachineId(99)).expect_err("unknown");
        assert!(studio.state.in_demo());
    }
}

mod the_bar {
    use super::*;

    #[tokio::test(start_paused = true)]
    async fn shows_the_demo_connected_with_no_address() {
        let studio = Studio::new();
        let recorder = Recorder::default();

        enter(&studio.state, recorder.clone(), spawned);
        settle().await;

        let connected = ConnectionView::Connected { route: String::new() };
        assert_eq!(current(&studio.state).connection, connected);
        assert_eq!(recorder.connections().last(), Some(&changed(DEMO, connected)));
    }
}

mod the_traces {
    use super::*;

    fn contents(dir: &TempDir) -> BTreeMap<PathBuf, String> {
        let entries = fs::read_dir(dir.path()).expect("the data folder is readable");
        let paths = entries.map(|entry| entry.expect("a readable entry").path());
        paths.map(|path| (path.clone(), fs::read_to_string(path).expect("a readable file"))).collect()
    }

    #[tokio::test(start_paused = true)]
    async fn leave_the_machines_and_their_secrets_as_they_were() {
        let dir = TempDir::new();
        let secrets: Arc<dyn Secrets> = Arc::new(FileSecrets::new(dir.path().join("secrets.json")));
        let state = AppState::load(Ok::<_, &str>(dir.path().to_path_buf()), secrets.as_ref(), Arc::clone(&secrets));
        let studio = state.machines(|machines| machines.add("studio", &[route()]).map_err(CommandError::from));
        start(&state, Recorder::default(), &studio_serving(), studio.expect("the studio is added")).expect("connected");
        settle().await;
        let before = contents(&dir);

        enter(&state, Recorder::default(), spawned);
        settle().await;
        leave(&state, Recorder::default(), &studio_serving()).expect("the demo is left");
        settle().await;

        assert_eq!(contents(&dir), before);
        assert_eq!(before.len(), 2, "{before:?}");
    }
}
