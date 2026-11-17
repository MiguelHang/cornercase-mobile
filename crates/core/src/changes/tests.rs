use serde_json::Value;

use super::*;
use crate::cli::changes::fixtures::SUMMARY;
use crate::follow::fake::{self, BOOT, Host, Said, Watched, connected, dropped, dropped_for_a_while, exits, session};

const FOLLOWED: [&str; 7] = ["changes", "list", "--follow", "--workspace", "2", "--mode", "commits"];

fn listed(state: &str, revision: u64, with_files: bool) -> Said {
    let mut summary: Value = serde_json::from_str(SUMMARY).expect("the summary fixture");
    (summary["state"], summary["revision"]) = (state.into(), revision.into());
    if !with_files {
        summary["workspaces"][0]["files"] = Value::Array(Vec::new());
    }
    Said::Line(summary.to_string())
}

fn ready(revision: u64) -> Said {
    listed("ready", revision, true)
}

fn starting(revision: u64) -> Said {
    listed("loading", revision, false)
}

fn refused() -> Said {
    fake::refused("no workspace 2")
}

struct Followed(Watched<Seen>);

impl Followed {
    async fn after_a_minute(host: Host) -> Self {
        let changes = Changes { workspace: 2, mode: Mode::Commits };
        Self(Watched::after_a_minute(host, move |store, sink| changes.run(store, sink)).await)
    }

    async fn of(streams: Vec<Vec<Said>>) -> Self {
        Self::after_a_minute(Host::of(&FOLLOWED, connected(), streams)).await
    }

    fn seen(&mut self) -> Vec<(u64, String)> {
        self.0.seen(|seen| match seen {
            Seen::Summary(summary) => format!("{:?} {}", summary.state, summary.revision).to_lowercase(),
            Seen::Failed(_) => "warned".to_owned(),
            Seen::Closed => "closed".to_owned(),
        })
    }

    fn follows(&mut self) -> usize {
        self.0.launched(&["changes"]).len()
    }
}

fn at(seen: &[(u64, &str)]) -> Vec<(u64, String)> {
    seen.iter().map(|(at, what)| (*at, (*what).to_owned())).collect()
}

mod summaries {
    use super::*;

    #[tokio::test(start_paused = true)]
    async fn follows_the_list_of_the_workspace_in_its_mode() {
        let mut followed = Followed::of(vec![vec![ready(2)]]).await;
        assert_eq!(followed.0.launched(&["changes"]), [FOLLOWED.join(" ")]);
    }

    #[tokio::test(start_paused = true)]
    async fn hands_each_summary_in_order() {
        let mut followed = Followed::of(vec![vec![ready(2), Said::After(3), ready(3), ready(4)]]).await;
        assert_eq!(followed.seen(), at(&[(0, "ready 2"), (3, "ready 3"), (3, "ready 4")]));
    }

    #[tokio::test(start_paused = true)]
    async fn ignores_a_loading_line_with_no_files_once_a_list_came() {
        let mut followed = Followed::of(vec![vec![starting(1), starting(2), ready(3), starting(4), ready(5)]]).await;
        assert_eq!(followed.seen(), at(&[(0, "loading 1"), (0, "ready 3"), (0, "ready 5")]));
    }

    #[tokio::test(start_paused = true)]
    async fn hands_a_loading_line_that_still_lists_files() {
        let mut followed = Followed::of(vec![vec![ready(2), listed("loading", 3, true)]]).await;
        assert_eq!(followed.seen(), at(&[(0, "ready 2"), (0, "loading 3")]));
    }

    #[tokio::test(start_paused = true)]
    async fn keeps_the_list_through_the_loading_line_of_a_follow_opened_again() {
        let streams = vec![vec![ready(2), Said::After(5), exits()], vec![starting(1), ready(2)]];
        let mut followed = Followed::of(streams).await;
        assert_eq!(followed.seen(), at(&[(0, "ready 2"), (6, "ready 2")]));
    }

    #[tokio::test(start_paused = true)]
    async fn hands_a_failed_listing_as_a_summary_and_keeps_following() {
        let mut followed = Followed::of(vec![vec![listed("failed", 2, false), Said::After(4), ready(3)]]).await;
        assert_eq!(followed.seen(), at(&[(0, "failed 2"), (4, "ready 3")]));
        assert_eq!(followed.follows(), 1);
    }

    #[tokio::test(start_paused = true)]
    async fn warns_once_of_unreadable_lines_and_goes_on() {
        let broken = || Said::Line("{\"boot\":".to_owned());
        let mut followed = Followed::of(vec![vec![broken(), broken(), ready(2), broken()]]).await;
        assert_eq!(followed.seen(), at(&[(0, "warned"), (0, "ready 2"), (0, "warned")]));
        assert_eq!(followed.follows(), 1);
    }
}

mod reopening {
    use super::*;

    #[tokio::test(start_paused = true)]
    async fn follows_again_when_the_stream_ends_with_the_workspace_still_there() {
        let mut followed = Followed::of(vec![vec![ready(2), Said::After(5), exits()], vec![ready(3)]]).await;
        assert_eq!(followed.seen(), at(&[(0, "ready 2"), (6, "ready 3")]));
        assert_eq!(followed.follows(), 2);
    }

    #[tokio::test(start_paused = true)]
    async fn says_the_workspace_closed_and_stops_once_the_session_lacks_it() {
        let sessions = vec![vec![session(BOOT, 7, true), Said::After(5), session(BOOT, 8, false)]];
        let streams = vec![vec![ready(2), Said::After(5), exits()], vec![ready(3)]];
        let mut followed = Followed::after_a_minute(Host::of(&FOLLOWED, sessions, streams)).await;
        assert_eq!(followed.seen(), at(&[(0, "ready 2"), (6, "closed")]));
        assert_eq!((followed.follows(), followed.0.running.is_finished()), (1, true));
    }

    #[tokio::test(start_paused = true)]
    async fn says_the_workspace_closed_without_following_it_when_the_session_lacks_it() {
        let sessions = vec![vec![session(BOOT, 7, false)]];
        let mut followed = Followed::after_a_minute(Host::of(&FOLLOWED, sessions, vec![vec![ready(2)]])).await;
        assert_eq!((followed.seen(), followed.follows()), (at(&[(0, "closed")]), 0));
    }

    #[tokio::test(start_paused = true)]
    async fn follows_again_once_connected_after_the_store_lost_its_transport() {
        let streams = vec![vec![ready(2), Said::After(5), dropped()], vec![ready(3)]];
        let mut followed = Followed::after_a_minute(Host::of(&FOLLOWED, dropped_for_a_while(), streams)).await;
        assert_eq!(followed.seen(), at(&[(0, "ready 2"), (26, "ready 3")]));
    }

    #[tokio::test(start_paused = true)]
    async fn waits_for_a_session_of_the_restarted_server_before_following_again() {
        let streams = vec![vec![ready(2), Said::After(5), exits()], vec![ready(1)]];
        let mut followed = Followed::after_a_minute(Host::restarting(&FOLLOWED, streams)).await;
        assert_eq!(followed.seen(), at(&[(0, "ready 2"), (15, "ready 1")]));
    }
}

mod failures {
    use super::*;

    #[tokio::test(start_paused = true)]
    async fn tells_once_of_a_follow_that_keeps_failing_with_the_workspace_still_there() {
        let mut followed = Followed::of(vec![vec![refused()]]).await;
        assert_eq!(followed.seen(), at(&[(0, "warned")]));
        assert!(followed.follows() > 50);
    }

    #[tokio::test(start_paused = true)]
    async fn tells_again_of_a_failure_after_a_summary_came() {
        let streams = vec![vec![refused()], vec![ready(2), Said::After(5), refused()], vec![ready(3)]];
        let mut followed = Followed::of(streams).await;
        assert_eq!(followed.seen(), at(&[(0, "warned"), (1, "ready 2"), (6, "warned"), (7, "ready 3")]));
    }
}
