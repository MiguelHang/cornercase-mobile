use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use cornercase_core::error::Error;
use cornercase_core::transport::fake::FakeTransport;
use tokio::task;
use tokio::time::Instant;

use super::*;
use crate::commands::connection::{current, start};
use crate::secrets::{mock_store, on_mock};
use crate::test_util::{CONTRACT, Recorder, Studio, launcher};
use crate::view::{ConnectionState, ConnectionView};

const NO_FOLDER: Result<PathBuf, &str> = Err("no home folder");

fn connecting(recorder: &Recorder) -> usize {
    recorder.connections().iter().filter(|changed| changed.connection == ConnectionView::Connecting).count()
}

fn retrying(recorder: &Recorder) -> usize {
    recorder
        .connections()
        .iter()
        .filter(|changed| matches!(changed.connection, ConnectionView::Retrying { .. }))
        .count()
}

async fn without_waiting(done: impl Fn() -> bool) -> bool {
    for _ in 0..1000 {
        if done() {
            return true;
        }
        task::yield_now().await;
    }
    done()
}

mod starting {
    use super::*;

    #[test]
    fn says_why_the_machines_did_not_load() {
        let state = AppState::load(NO_FOLDER, Arc::new(on_mock(mock_store())));
        let recorder = Recorder::default();
        started(&state, &recorder);
        assert_eq!(recorder.toasts(), [Toast::error("the app has no folder for its data: no home folder")]);
    }

    #[test]
    fn stays_quiet_when_they_load() {
        let studio = Studio::new();
        let recorder = Recorder::default();
        started(&studio.state, &recorder);
        assert_eq!(recorder.all(), []);
    }
}

mod coming_back_to_the_foreground {
    use super::*;

    #[test]
    fn does_nothing_without_a_store() {
        let studio = Studio::new();
        on_foreground(&studio.state);
        assert_eq!(current(&studio.state), ConnectionState::none());
    }

    #[tokio::test(start_paused = true)]
    async fn retries_at_once_instead_of_waiting() {
        let studio = Studio::new();
        let recorder = Recorder::default();
        let (launcher, _) = launcher(FakeTransport::default().fail(CONTRACT, Error::Timeout));
        start(&studio.state, recorder.clone(), &launcher, studio.id).expect("connecting");
        assert!(without_waiting(|| retrying(&recorder) == 1).await, "{:?}", recorder.connections());
        let resumed = Instant::now();
        on_foreground(&studio.state);
        assert!(without_waiting(|| connecting(&recorder) == 2).await, "{:?}", recorder.connections());
        assert_eq!(resumed.elapsed(), Duration::ZERO);
    }
}
