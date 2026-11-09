use std::time::Duration;

use rstest::rstest;
use tokio::time;

use cornercase_core::transport::fake::FakeTransport;

use super::*;
use crate::commands::connection::start;
use crate::events::ConnectionChanged;
use crate::test_util::{
    APP, BOOT, EVENTS, Ipc, Recorder, STATUS, Studio, changed, connected, contracted, launcher, serving, settle,
    snapshot,
};
use crate::view::ConnectionView;

const WIFI: Network = Network { kind: Kind::Wifi, expensive: false };
const CELLULAR: Network = Network { kind: Kind::Cellular, expensive: true };

struct Following {
    hear: Box<dyn Fn(Network) + Send + Sync>,
    recorder: Recorder,
    studio: Studio,
    _ipc: Ipc,
}

impl Following {
    fn new() -> Self {
        Self::on(serving(&[snapshot(BOOT, 1, "first")]))
    }

    fn on(fake: FakeTransport) -> Self {
        let studio = Studio::new();
        let state = studio.reloaded();
        let recorder = Recorder::default();
        let (launcher, _) = launcher(fake);
        start(&state, recorder.clone(), &launcher, studio.id).expect("connecting");
        let ipc = Ipc::new(state);
        let (hear, paths) = listen();
        tokio::spawn(follow(ipc.app.handle().clone(), paths));
        Self { hear: Box::new(hear), recorder, studio, _ipc: ipc }
    }

    async fn heard(&self, networks: &[Network]) {
        for network in networks {
            (self.hear)(*network);
            time::sleep(Duration::from_millis(300)).await;
        }
    }

    fn connections(&self) -> Vec<ConnectionChanged> {
        self.recorder.connections()
    }

    fn changed(&self, connections: &[ConnectionView]) -> Vec<ConnectionChanged> {
        connections.iter().map(|connection| changed(self.studio.id, connection.clone())).collect()
    }
}

fn connecting_count(connections: &[ConnectionChanged]) -> usize {
    connections.iter().filter(|changed| changed.connection == ConnectionView::Connecting).count()
}

mod the_baseline {
    use super::*;

    #[tokio::test(start_paused = true)]
    async fn on_a_network_leaves_the_store_connected() {
        let following = Following::new();
        following.heard(&[WIFI]).await;
        settle().await;
        assert_eq!(following.connections(), following.changed(&[ConnectionView::Connecting, connected()]));
    }

    #[tokio::test(start_paused = true)]
    async fn without_network_takes_the_store_offline() {
        let following = Following::new();
        following.heard(&[DOWN]).await;
        settle().await;
        assert_eq!(following.connections().last().map(|it| &it.connection), Some(&ConnectionView::Offline));
    }
}

mod a_change {
    use super::*;

    #[tokio::test(start_paused = true)]
    async fn to_no_network_takes_the_store_offline() {
        let following = Following::new();
        following.heard(&[WIFI]).await;
        settle().await;
        following.heard(&[DOWN]).await;
        settle().await;
        assert_eq!(
            following.connections(),
            following.changed(&[ConnectionView::Connecting, connected(), ConnectionView::Offline])
        );
    }

    #[tokio::test(start_paused = true)]
    async fn back_from_no_network_connects_again() {
        let following = Following::new();
        following.heard(&[DOWN]).await;
        settle().await;
        following.heard(&[CELLULAR]).await;
        settle().await;
        let connections = following.connections();
        assert_eq!(connections.last().map(|it| &it.connection), Some(&connected()), "{connections:?}");
        assert_eq!(connecting_count(&connections), 2, "{connections:?}");
    }

    #[tokio::test(start_paused = true)]
    async fn while_connecting_abandons_the_attempt_for_another() {
        let hanging = contracted(BOOT).open(EVENTS, &[]).open(STATUS, &[]).open(APP, &[]);
        let following = Following::on(hanging);
        following.heard(&[WIFI]).await;
        settle().await;
        assert_eq!(connecting_count(&following.connections()), 1);
        following.heard(&[CELLULAR]).await;
        settle().await;
        assert_eq!(connecting_count(&following.connections()), 2, "{:?}", following.connections());
    }

    #[tokio::test(start_paused = true)]
    async fn in_a_burst_reconnects_once() {
        let following = Following::new();
        following.heard(&[WIFI]).await;
        settle().await;
        following.heard(&[CELLULAR, DOWN, Network { kind: Kind::Other, expensive: false }]).await;
        settle().await;
        let connections = following.connections();
        assert_eq!(connecting_count(&connections), 2, "{connections:?}");
        assert!(!connections.iter().any(|it| it.connection == ConnectionView::Offline), "{connections:?}");
        assert_eq!(connections.last().map(|it| &it.connection), Some(&connected()), "{connections:?}");
    }
}

mod an_app_without_its_state {
    use super::*;

    #[tokio::test(start_paused = true)]
    async fn ignores_what_it_hears() {
        let app = tauri::test::mock_app();
        let (hear, paths) = listen();
        let following = tokio::spawn(follow(app.handle().clone(), paths));
        hear(DOWN);
        settle().await;
        drop(hear);
        following.await.expect("the follower ends once the plugin is gone");
        assert!(app.try_state::<AppState>().is_none());
    }
}

mod the_log_line {
    use super::*;

    #[rstest]
    #[case::wifi(WIFI, "network wifi")]
    #[case::expensive_cellular(CELLULAR, "network cellular expensive")]
    #[case::other(Network { kind: Kind::Other, expensive: false }, "network other")]
    #[case::none(DOWN, "network none")]
    fn names_the_kind_and_whether_it_costs(#[case] network: Network, #[case] expected: &str) {
        assert_eq!(line(network), expected);
    }
}
