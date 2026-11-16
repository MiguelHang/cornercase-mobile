use std::time::Duration;

use rstest::rstest;
use tokio::sync::mpsc;
use tokio::time;

use crate::cli::surface::Size;
use crate::demo::DemoTransport;
use crate::demo::tests::entered;
use crate::terminal::grid::Line;
use crate::terminal::live::{Screen, Terminal};
use crate::terminal::shown::Update;
use crate::transport::{self, Output};

const SIZE: Size = Size { cols: 80, rows: 24 };

async fn painted(tab: u64, size: Size, fit: bool) -> Update {
    let (store, _) = entered().await;
    let (said, mut heard) = mpsc::unbounded_channel();
    let terminal = Terminal { tab, pane: None, size, fit };
    tokio::spawn(terminal.run(store, move |screen| drop(said.send(screen))));
    time::sleep(Duration::from_secs(1)).await;
    let screens: Vec<Screen> = std::iter::from_fn(|| heard.try_recv().ok()).collect();
    let [Screen::Update(update)] = screens.as_slice() else { panic!("the terminal showed {screens:?}") };
    update.clone()
}

fn text(line: &Line) -> String {
    line.iter().map(|run| run.text.as_str()).collect()
}

fn texts(update: &Update) -> Vec<String> {
    update.rows.iter().map(|(_, line)| text(line)).collect()
}

mod the_terminal {
    use super::*;

    #[rstest]
    #[tokio::test(start_paused = true)]
    async fn paints_every_tab_in_colour_followed_or_fitted(
        #[values(3, 5, 7, 9, 12, 16)] tab: u64,
        #[values(false, true)] fit: bool,
    ) {
        let update = painted(tab, SIZE, fit).await;

        let runs = update.rows.iter().flat_map(|(_, line)| line);
        let coloured = runs.filter(|run| run.style.fg.is_some() && !run.text.trim().is_empty()).count();
        assert!(update.replaced);
        assert_eq!(update.size, SIZE);
        assert!(coloured > 1, "only {coloured} coloured runs");
    }

    #[tokio::test(start_paused = true)]
    async fn shows_the_end_of_an_output_taller_than_itself() {
        let update = painted(3, Size { cols: 80, rows: 3 }, false).await;

        assert_eq!(texts(&update), ["", "> ", "  ⏵⏵ accept edits on (shift+tab to cycle)"]);
    }

    #[tokio::test(start_paused = true)]
    async fn cuts_the_lines_wider_than_itself() {
        let update = painted(9, Size { cols: 20, rows: 24 }, true).await;

        let rows = texts(&update);
        assert_eq!(rows[0], "demo@studio ~/src/ac");
        assert!(rows.iter().all(|row| row.chars().count() <= 20), "{rows:?}");
    }
}

mod a_view {
    use super::*;

    #[tokio::test]
    async fn that_was_never_opened_has_no_surface() {
        let argv = ["surface", "--view", "99", "--patches"].map(str::to_owned).to_vec();
        let answer = transport::output(&DemoTransport::new(), argv).await.expect("the demo answers");
        let stderr = "error: no view 99".to_owned();
        assert_eq!(answer, Output { stdout: String::new(), stderr, code: 1 });
    }
}
