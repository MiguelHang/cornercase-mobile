use std::time::Duration;

use rstest::rstest;
use tokio::sync::mpsc;
use tokio::time;

use crate::changes::{Changes, Seen};
use crate::cli::changes::{self, ChangedFile, LineKind, Mark, Mode, Patch, State, Summary};
use crate::cli::files::{self, Read};
use crate::demo::DemoTransport;
use crate::demo::tests::entered;
use crate::error::Error;
use crate::store::Store;
use crate::transport::{self, partial_json};

const SHOP: u64 = 2;
const FORM: &str = "src/checkout/PaymentForm.tsx";

struct Followed {
    store: Store,
    heard: mpsc::UnboundedReceiver<Seen>,
}

impl Followed {
    async fn start(workspace: u64) -> Self {
        let (store, _) = entered().await;
        let (said, heard) = mpsc::unbounded_channel();
        let changes = Changes { workspace, mode: Mode::Uncommitted };
        tokio::spawn(changes.run(store.clone(), move |seen| drop(said.send(seen))));
        time::sleep(Duration::from_secs(1)).await;
        Self { store, heard }
    }

    fn summaries(&mut self) -> Vec<Summary> {
        let seen: Vec<Seen> = std::iter::from_fn(|| self.heard.try_recv().ok()).collect();
        seen.into_iter()
            .map(|seen| match seen {
                Seen::Summary(summary) => summary,
                other => panic!("the changes said {other:?}"),
            })
            .collect()
    }

    async fn mark(&self, path: &str, mark: &Mark) {
        let queue = self.store.queue().expect("the demo has a queue");
        queue.ask(changes::viewed_args(SHOP, path, mark), None).await.expect("the demo marks the file");
        time::sleep(Duration::from_secs(1)).await;
    }
}

fn file<'a>(summary: &'a Summary, path: &str) -> &'a ChangedFile {
    let mut files = summary.workspaces.iter().flat_map(|workspace| &workspace.files);
    files.find(|file| file.path == path).expect("the summary lists the file")
}

async fn patch(workspace: u64, path: &str, mode: Mode) -> Result<Patch, Error> {
    transport::json(&DemoTransport::new(), changes::file_args(workspace, path, mode)).await
}

async fn text(workspace: u64, path: &str) -> Vec<String> {
    let read: Read = partial_json(&DemoTransport::new(), files::read_args(workspace, path)).await.expect("the file");
    read.text.expect("a text file").lines().map(str::to_owned).collect()
}

mod the_list {
    use super::*;

    #[rstest]
    #[tokio::test(start_paused = true)]
    async fn shows_the_changed_files_of_each_workspace(#[values(2, 11, 15)] workspace: u64) {
        let mut followed = Followed::start(workspace).await;
        let [summary] = followed.summaries().try_into().unwrap_or_else(|seen| panic!("the changes said {seen:?}"));
        let [changed] = summary.workspaces.as_slice() else { panic!("the summary has {:?}", summary.workspaces) };
        assert_eq!((summary.state, changed.workspace), (State::Ready, workspace));
        assert_ne!(changed.files, []);
        assert_eq!(changed.added, changed.files.iter().map(|file| file.added).sum::<u64>());
    }

    #[tokio::test(start_paused = true)]
    async fn says_nothing_new_while_nothing_changes() {
        let mut followed = Followed::start(SHOP).await;
        followed.summaries();
        time::sleep(Duration::from_secs(30)).await;
        assert_eq!(followed.summaries(), []);
    }
}

mod viewing {
    use super::*;

    #[tokio::test(start_paused = true)]
    async fn marks_a_file_seen_in_the_list() {
        let mut followed = Followed::start(SHOP).await;
        let digest = file(&followed.summaries()[0], FORM).digest.clone();
        followed.mark(FORM, &Mark::Seen { digest }).await;
        let [summary] = followed.summaries().try_into().unwrap_or_else(|seen| panic!("the changes said {seen:?}"));
        assert!(file(&summary, FORM).viewed);
        assert_eq!(summary.revision, 2);
    }

    #[tokio::test(start_paused = true)]
    async fn clears_the_mark_again() {
        let mut followed = Followed::start(SHOP).await;
        let digest = file(&followed.summaries()[0], FORM).digest.clone();
        followed.mark(FORM, &Mark::Seen { digest }).await;
        followed.mark(FORM, &Mark::Clear { mode: Mode::Uncommitted }).await;
        let summaries = followed.summaries();
        assert!(!file(summaries.last().expect("a new summary"), FORM).viewed);
    }

    #[tokio::test(start_paused = true)]
    async fn shows_the_mark_in_the_diff() {
        let followed = Followed::start(SHOP).await;
        followed.mark(FORM, &Mark::Seen { digest: "seen".to_owned() }).await;
        let transport = followed.store.transport().expect("the demo is connected");
        let argv = changes::file_args(SHOP, FORM, Mode::Uncommitted);
        let shown: Patch = transport::json(transport.as_ref(), argv).await.expect("the diff");
        assert!(shown.file.viewed);
    }
}

mod a_diff {
    use super::*;

    #[tokio::test]
    async fn shows_the_removed_and_added_lines_with_their_numbers() {
        let patch = patch(SHOP, FORM, Mode::All).await.expect("the diff");
        let lines: Vec<_> = patch.hunks.iter().flat_map(|hunk| &hunk.lines).collect();
        let removed = lines.iter().find(|line| line.kind == LineKind::Removed).expect("a removed line");
        assert_eq!((removed.old, removed.new, removed.text.as_str()), (Some(12), None, "    <CardForm onPay={pay} />"));
        assert_eq!((patch.file.added, patch.file.removed, patch.mode), (5, 1, Mode::All));
    }

    #[rstest]
    #[case::the_shop(2)]
    #[case::the_fix(11)]
    #[case::the_package(15)]
    #[tokio::test(start_paused = true)]
    async fn agrees_with_the_files_on_every_new_line(#[case] workspace: u64) {
        let listed = listed(workspace).await;
        for changed in listed.into_iter().filter(|file| file.status != changes::Status::Deleted) {
            let patch = patch(workspace, &changed.path, Mode::Uncommitted).await.expect("the diff");
            let text = text(workspace, &changed.path).await;
            let shown =
                patch.hunks.iter().flat_map(|hunk| &hunk.lines).filter_map(|line| Some((line.new?, &line.text)));
            for (number, line) in shown {
                assert_eq!(text.get(usize::try_from(number - 1).unwrap_or_default()), Some(line), "{}", changed.path);
            }
        }
    }

    #[tokio::test]
    async fn fails_for_a_file_that_did_not_change() {
        let error = patch(SHOP, "README.md", Mode::Uncommitted).await.expect_err("README.md did not change");
        assert!(matches!(error, Error::Failed { code: 2, .. }), "{error:?}");
    }

    async fn listed(workspace: u64) -> Vec<ChangedFile> {
        let mut followed = Followed::start(workspace).await;
        followed.summaries().remove(0).workspaces.remove(0).files
    }
}
