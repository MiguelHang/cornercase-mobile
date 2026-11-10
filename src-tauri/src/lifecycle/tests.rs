use std::time::Duration;

use cornercase_core::error::Error;
use cornercase_core::transport::fake::FakeTransport;
use tokio::task;
use tokio::time::Instant;

use super::*;
use crate::commands::connection::{current, start};
use crate::test_util::{CONTRACT, Ipc, Recorder, Studio, launcher};
use crate::view::{ConnectionState, ConnectionView};

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

async fn retrying_in_an_app(studio: &Studio, recorder: &Recorder) -> Ipc {
    let state = studio.reloaded();
    let (launcher, _) = launcher(FakeTransport::default().fail(CONTRACT, Error::Timeout));
    start(&state, recorder.clone(), &launcher, studio.id).expect("connecting");
    assert!(without_waiting(|| retrying(recorder) == 1).await, "{:?}", recorder.connections());
    Ipc::new(state)
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

mod an_app_event {
    use super::*;

    #[tokio::test(start_paused = true)]
    async fn resumes_the_store_of_the_app() {
        let studio = Studio::new();
        let recorder = Recorder::default();
        let ipc = retrying_in_an_app(&studio, &recorder).await;
        let resumed = Instant::now();
        resume(ipc.app.handle());
        assert!(without_waiting(|| connecting(&recorder) == 2).await, "{:?}", recorder.connections());
        assert_eq!(resumed.elapsed(), Duration::ZERO);
    }

    #[test]
    fn resumes_nothing_before_the_app_has_its_state() {
        let app = tauri::test::mock_app();
        resume(app.handle());
        assert!(app.try_state::<AppState>().is_none());
    }

    #[tokio::test(start_paused = true)]
    async fn that_is_not_a_return_to_the_foreground_leaves_the_store_waiting() {
        let studio = Studio::new();
        let recorder = Recorder::default();
        let ipc = retrying_in_an_app(&studio, &recorder).await;
        on_event(ipc.app.handle(), &RunEvent::Ready);
        assert!(!without_waiting(|| connecting(&recorder) == 2).await, "{:?}", recorder.connections());
    }
}

mod a_page_loading {
    use tauri::webview::PageLoadEvent;

    use super::*;

    fn holding_an_image() -> (Ipc, u64) {
        let ipc = Ipc::new(Studio::new().reloaded());
        let id = ipc.app.state::<AppState>().attached().store(3, "IMG_0042.jpg".into(), vec![1, 2, 3]).expect("kept");
        (ipc, id)
    }

    #[test]
    fn that_starts_forgets_the_images_kept_for_a_page_gone() {
        let (ipc, id) = holding_an_image();
        page_loading(ipc.app.handle(), PageLoadEvent::Started);
        assert_eq!(ipc.app.state::<AppState>().attached().get(3, id), None);
    }

    #[test]
    fn that_finishes_keeps_them() {
        let (ipc, id) = holding_an_image();
        page_loading(ipc.app.handle(), PageLoadEvent::Finished);
        assert!(ipc.app.state::<AppState>().attached().get(3, id).is_some());
    }

    #[test]
    fn before_the_app_has_its_state_does_nothing() {
        let app = tauri::test::mock_app();
        page_loading(app.handle(), PageLoadEvent::Started);
        assert!(app.try_state::<AppState>().is_none());
    }
}
