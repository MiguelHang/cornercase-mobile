use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;

use cornercase_core::machines::MachineId;
use cornercase_core::network::{Kind, Network};
use cornercase_core::secrets::Secrets;
use cornercase_core::secrets::file::FileSecrets;
use cornercase_core::test_util::TempDir;

use super::*;
use crate::commands::connection::{current, latest_agents};
use crate::commands::error::CommandError;
use crate::test_util::{BOOT, Recorder, Studio, changed, launcher, route, serving, settle, snapshot, spawned};

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
