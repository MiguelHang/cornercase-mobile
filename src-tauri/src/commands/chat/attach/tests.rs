use std::fs;
use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver, Sender};

use cornercase_bridge::test_util::TempDir;
use cornercase_core::cli::paste_file::{Kind, MAX_IMAGE_BYTES, Pasted, Picked};
use cornercase_core::error::Error;
use cornercase_core::transport::fake::FakeTransport;
use cornercase_core::transport::{Closed, Exit, Out, OutSink, Stdin};
use futures_util::stream::BoxStream;
use futures_util::{SinkExt, StreamExt};
use rstest::rstest;
use serde_json::{Value, json};

use super::{attach_image, paste_image};
use crate::commands::chat::press_key;
use crate::commands::chat::tests::json_of;
use crate::commands::error::CommandError;
use crate::state::AppState;
use crate::test_util::{Ipc, Studio, connected_with, requests, written};

const PASTE: &[&str] = &["paste-file", "--pane", "543", "--name=IMG_0042.jpg", "--json", "-"];
const PASTED: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../crates/core/tests/fixtures/control/paste-file/0.13.19/pasted.json"
));
const PHOTO: &[u8] = &[0xff, 0xd8, 0xff, 0xe0, 0x00, 0x10];
const OLD_CLI: &str = "error: unrecognized subcommand 'paste-file'";
const NO_PANE: &str = "there is no pane 543; `cornercase status` lists them";
const BUSY: &str = "a message sent with `cornercase send` is between its paste and its Enter in pane 543, and what \
                    you type would join it; nothing was sent: send again in a moment";
const MALFORMED: &str = "`paste-file` needs the file: `data` holds its bytes in base64";

type Fed = Sender<Vec<Stdin>>;

struct Attached {
    answer: Result<Option<Pasted>, Value>,
    fed: Vec<Vec<Stdin>>,
    left: bool,
}

fn photo(dir: &TempDir, bytes: &[u8]) -> Picked {
    let path = dir.path().join("photo.jpg");
    fs::write(&path, bytes).expect("the photo is written");
    Picked {
        path: path.display().to_string(),
        name: Some("IMG_0042.HEIC".to_owned()),
        kind: Kind::Jpeg,
        thumbnail_path: None,
    }
}

fn answering(printed: Out, code: i32) -> impl FnOnce(FakeTransport, &Fed) -> FakeTransport {
    move |fake, fed| {
        let fed = fed.clone();
        fake.script(PASTE, move |stdin, out| answer(fed.clone(), printed.clone(), code, stdin, out))
    }
}

async fn answer(
    fed: Fed,
    printed: Out,
    code: i32,
    stdin: BoxStream<'static, Stdin>,
    mut out: OutSink,
) -> Result<Exit, Error> {
    fed.send(stdin.collect().await).expect("the test reads what was fed");
    let stderr = match &printed {
        Out::Stderr(text) => text.clone(),
        Out::Stdout(_) => String::new(),
    };
    out.send(printed).await.map_err(|Closed| Error::OutputClosed)?;
    Ok(Exit { code, stderr })
}

async fn attaching(bytes: &[u8], scripted: impl FnOnce(FakeTransport, &Fed) -> FakeTransport) -> Attached {
    let dir = TempDir::new();
    let picked = photo(&dir, bytes);
    let path = picked.path.clone();
    let (fed, heard) = mpsc::channel();
    let ipc = connected_with(|fake| scripted(fake, &fed)).await;
    let answer = attach_image(&ipc.state(), 543, move || Ok(Some(picked))).await.map_err(|error| json_of(&error));
    Attached { answer, fed: heard.try_iter().collect(), left: Path::new(&path).exists() }
}

async fn picking(pick: Result<Option<Picked>, Error>) -> (Result<Option<Pasted>, Value>, Vec<Vec<Stdin>>) {
    let (fed, heard) = mpsc::channel();
    let ipc = connected_with(|fake| answering(Out::Stdout(PASTED.to_owned()), 0)(fake, &fed)).await;
    let answer = attach_image(&ipc.state(), 543, move || pick).await.map_err(|error| json_of(&error));
    (answer, heard.try_iter().collect())
}

struct Pasting {
    answer: Result<Pasted, Value>,
    fed: Vec<Vec<Stdin>>,
    kept: bool,
}

fn stored(state: &AppState) -> u64 {
    state.attached().store(543, "IMG_0042.jpg".to_owned(), PHOTO.to_vec()).expect("the photo is stored")
}

async fn pasting(scripted: impl FnOnce(FakeTransport, &Fed) -> FakeTransport) -> Pasting {
    let (fed, heard) = mpsc::channel();
    let ipc = connected_with(|fake| scripted(fake, &fed)).await;
    let state = ipc.state();
    let id = stored(&state);
    let answer = paste_image(&state, 543, id).await.map_err(|error| json_of(&error));
    Pasting { answer, fed: heard.try_iter().collect(), kept: state.attached().get(543, id).is_some() }
}

async fn in_line() -> (Ipc, Receiver<String>) {
    let (lines, heard) = mpsc::channel();
    let ipc = connected_with(|fake| {
        let pasted = lines.clone();
        requests(fake, Some(json!({ "status": 0, "text": "" })), &lines)
            .script(PASTE, move |stdin, out| noted(pasted.clone(), stdin, out))
    })
    .await;
    (ipc, heard)
}

async fn noted(lines: Sender<String>, stdin: BoxStream<'static, Stdin>, mut out: OutSink) -> Result<Exit, Error> {
    let fed: Vec<Stdin> = stdin.collect().await;
    let line = json!({ "args": PASTE, "stdin": fed == [Stdin::Bytes(PHOTO.to_vec())] });
    lines.send(line.to_string()).expect("the test reads the lines");
    out.send(Out::Stdout(PASTED.to_owned())).await.map_err(|Closed| Error::OutputClosed)?;
    Ok(Exit { code: 0, stderr: String::new() })
}

mod attaching_an_image {
    use super::*;

    #[tokio::test(start_paused = true)]
    async fn feeds_its_bytes_to_paste_file_and_answers_where_they_went() {
        let attached = attaching(PHOTO, answering(Out::Stdout(PASTED.to_owned()), 0)).await;
        let pasted: Pasted = serde_json::from_str(PASTED).expect("the fixture parses");
        assert_eq!(attached.answer, Ok(Some(pasted)));
        assert_eq!(attached.fed, [vec![Stdin::Bytes(PHOTO.to_vec())]]);
        assert!(!attached.left);
    }

    #[rstest]
    #[case::busy(&format!("Error: {BUSY}"), 1, BUSY)]
    #[case::not_found(&format!("Error: {NO_PANE}\n"), 1, NO_PANE)]
    #[case::malformed(&format!("Error: {MALFORMED}"), 1, MALFORMED)]
    #[case::an_older_cornercase(OLD_CLI, 2, OLD_CLI)]
    #[tokio::test(start_paused = true)]
    async fn fails_with_the_reason_cornercase_gave(#[case] stderr: &str, #[case] code: i32, #[case] reason: &str) {
        let attached = attaching(PHOTO, answering(Out::Stderr(stderr.to_owned()), code)).await;
        assert_eq!(attached.answer, Err(json!({ "code": "failed", "message": reason })));
        assert!(!attached.left);
    }

    #[tokio::test(start_paused = true)]
    async fn may_have_sent_it_when_the_answer_is_lost() {
        let attached = attaching(PHOTO, |fake, _| fake.fail(PASTE, Error::Unanswered)).await;
        assert_eq!(attached.answer, Err(json_of(&CommandError::maybe_sent())));
        assert!(!attached.left);
    }

    #[tokio::test(start_paused = true)]
    async fn says_when_cornercase_printed_something_else() {
        let attached = attaching(PHOTO, answering(Out::Stdout("pasted".to_owned()), 0)).await;
        assert_eq!(attached.answer.expect_err("not json")["code"], "json");
        assert!(!attached.left);
    }

    #[rstest]
    #[case::empty(Vec::new(), Error::ImageEmpty)]
    #[case::over_the_most_cornercase_pastes(vec![0; MAX_IMAGE_BYTES + 1], Error::ImageTooLarge)]
    #[tokio::test(start_paused = true)]
    async fn refuses_what_cornercase_would_and_queues_nothing(#[case] bytes: Vec<u8>, #[case] refusal: Error) {
        let attached = attaching(&bytes, answering(Out::Stdout(PASTED.to_owned()), 0)).await;
        assert_eq!(attached.answer, Err(json_of(&CommandError::from(refusal))));
        assert_eq!(attached.fed, Vec::<Vec<Stdin>>::new());
        assert!(!attached.left);
    }
}

mod pasting_an_attached_image {
    use super::*;

    #[tokio::test(start_paused = true)]
    async fn feeds_the_stored_bytes_to_paste_file_and_forgets_them() {
        let pasting = pasting(answering(Out::Stdout(PASTED.to_owned()), 0)).await;
        let pasted: Pasted = serde_json::from_str(PASTED).expect("the fixture parses");
        assert_eq!(pasting.answer, Ok(pasted));
        assert_eq!(pasting.fed, [vec![Stdin::Bytes(PHOTO.to_vec())]]);
        assert!(!pasting.kept);
    }

    #[tokio::test(start_paused = true)]
    async fn goes_through_the_queue_behind_a_key_pressed_before() {
        let (ipc, lines) = in_line().await;
        let state = ipc.state();
        let id = stored(&state);
        let (pressed, pasted) = tokio::join!(press_key(&state, 543, "enter"), paste_image(&state, 543, id));
        assert_eq!(pressed, Ok(()));
        assert!(pasted.is_ok(), "{pasted:?}");
        let keys = json!({ "args": ["keys", "--pane", "543", "enter"] });
        assert_eq!(written(&lines), [keys, json!({ "args": PASTE, "stdin": true })]);
    }

    #[rstest]
    #[case::busy(answering(Out::Stderr(format!("Error: {BUSY}")), 1), json!({ "code": "failed", "message": BUSY }))]
    #[case::lost(|fake: FakeTransport, _: &Fed| fake.fail(PASTE, Error::Unanswered), json_of(&CommandError::maybe_sent()))]
    #[tokio::test(start_paused = true)]
    async fn keeps_the_image_when_the_paste_fails(
        #[case] scripted: impl FnOnce(FakeTransport, &Fed) -> FakeTransport,
        #[case] reason: Value,
    ) {
        let pasting = pasting(scripted).await;
        assert_eq!(pasting.answer, Err(reason));
        assert!(pasting.kept);
    }

    #[tokio::test(start_paused = true)]
    async fn forgets_an_image_cornercase_pasted_even_when_its_answer_is_not_json() {
        let pasting = pasting(answering(Out::Stdout("pasted".to_owned()), 0)).await;
        assert_eq!(pasting.answer.expect_err("not json")["code"], "json");
        assert!(!pasting.kept);
    }

    #[rstest]
    #[case::unknown(|_: &AppState| 99, 543)]
    #[case::of_another_pane(stored, 544)]
    #[case::discarded(|state: &AppState| { let id = stored(state); state.attached().discard(&[id]); id }, 543)]
    #[tokio::test(start_paused = true)]
    async fn refuses_an_image_that_is_not_waiting_and_queues_nothing(
        #[case] id_of: impl FnOnce(&AppState) -> u64,
        #[case] pane: u64,
    ) {
        let (fed, heard) = mpsc::channel();
        let ipc = connected_with(|fake| answering(Out::Stdout(PASTED.to_owned()), 0)(fake, &fed)).await;
        let id = id_of(&ipc.state());
        let answer = paste_image(&ipc.state(), pane, id).await;
        assert_eq!(answer, Err(CommandError::attached_gone()));
        assert_eq!(heard.try_iter().count(), 0);
    }
}

mod discarding {
    use super::*;

    #[test]
    fn forgets_the_images_it_names_and_keeps_the_rest() {
        let ipc = Ipc::new(Studio::new().reloaded());
        let state = ipc.state();
        let (gone, kept) = (stored(&state), stored(&state));
        assert_eq!(ipc.invoke("discard", json!({ "ids": [gone, 99] })), Ok(Value::Null));
        assert_eq!(state.attached().get(543, gone), None);
        assert!(state.attached().get(543, kept).is_some());
    }
}

mod picking_a_photo {
    use super::*;

    #[tokio::test(start_paused = true)]
    async fn answers_nothing_and_queues_nothing_when_cancelled() {
        assert_eq!(picking(Ok(None)).await, (Ok(None), Vec::new()));
    }

    #[tokio::test(start_paused = true)]
    async fn gives_the_reason_the_picker_failed() {
        let failed = Error::Picker("the photo cannot be decoded".to_owned());
        assert_eq!(picking(Err(failed.clone())).await, (Err(json_of(&CommandError::from(failed))), Vec::new()));
    }

    #[tokio::test(start_paused = true)]
    async fn says_when_the_photo_it_wrote_cannot_be_read() {
        let gone =
            Picked { path: "/nonexistent/photo.jpg".to_owned(), name: None, kind: Kind::Png, thumbnail_path: None };
        let (answer, fed) = picking(Ok(Some(gone))).await;
        assert_eq!(answer.expect_err("nothing to read")["code"], "picker");
        assert_eq!(fed, Vec::<Vec<Stdin>>::new());
    }
}

mod without_a_connection {
    use super::*;

    #[tokio::test]
    async fn says_it_is_not_connected_before_showing_the_picker() {
        let ipc = Ipc::new(Studio::new().reloaded());
        let shown = Arc::new(AtomicBool::new(false));
        let picker = Arc::clone(&shown);
        let answer = attach_image(&ipc.state(), 543, move || {
            picker.store(true, Ordering::SeqCst);
            Ok(None)
        })
        .await;
        assert_eq!(answer, Err(CommandError::not_connected()));
        assert!(!shown.load(Ordering::SeqCst));
    }
}
