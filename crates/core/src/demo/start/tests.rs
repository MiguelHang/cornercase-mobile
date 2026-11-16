use std::time::Duration;

use rstest::rstest;
use tokio::time;

use crate::cli::agents::{self, Login, StartAgents};
use crate::cli::last_message::{self, LastMessage};
use crate::cli::start::{self, Place, StartAgent, StartIssue, Started};
use crate::demo::tests::{entered, status};
use crate::error::Error;
use crate::queue::Outcome;
use crate::store::Store;
use crate::transport;

const SHOP: u64 = 1;
const MAIN: u64 = 2;
const THINKING_AT_MOST: Duration = Duration::from_secs(4);
const PROMPT: &str = "Add a coupon field to the cart";

fn form(place: Place) -> StartAgent {
    StartAgent {
        place,
        agent: "codex".to_owned(),
        model: String::new(),
        prompt: PROMPT.to_owned(),
        accounts: Vec::new(),
    }
}

async fn asked(store: &Store, argv: Vec<String>) -> Result<String, Error> {
    let queue = store.queue().expect("the demo has a queue");
    let Outcome::Done(text) = queue.ask(argv, None).await? else { panic!("the demo skipped the request") };
    Ok(text)
}

async fn launched(store: &Store, place: Place) -> Started {
    let text = asked(store, form(place).argv()).await.expect("the demo starts the agent");
    start::started(&text).expect("the demo says where the agent started")
}

mod the_choices {
    use super::*;

    async fn installed() -> StartAgents {
        let (store, _) = entered().await;
        let argv = ["account", "list", "--check", "--json"].map(str::to_owned).to_vec();
        agents::installed(&asked(&store, argv).await.expect("the demo lists the accounts")).expect("the agents")
    }

    #[tokio::test(start_paused = true)]
    async fn offer_the_installed_agents() {
        let names: Vec<String> = installed().await.agents.into_iter().map(|choice| choice.agent).collect();
        assert_eq!(names, ["claude", "codex"]);
    }

    #[tokio::test(start_paused = true)]
    async fn offer_the_accounts_signed_in_or_not() {
        let accounts = installed().await.accounts;
        let [claude] = accounts.as_slice() else { panic!("the demo offers {accounts:?}") };
        let offered: Vec<(&str, Login)> =
            claude.options.iter().map(|option| (option.label.as_str(), option.login)).collect();
        let expected = [("work", Login::SignedIn), ("personal", Login::SignedIn), ("side projects", Login::SignedOut)];
        assert_eq!((claude.default.as_str(), offered.as_slice()), ("personal", expected.as_slice()));
    }

    #[tokio::test(start_paused = true)]
    async fn offer_the_workspaces_of_each_project() {
        let (store, _) = entered().await;
        let session = store.session().expect("the demo has a session");
        let workspaces = session.projects.iter().map(|project| project.workspaces.len());
        assert!(workspaces.clone().all(|count| count > 0), "{session:?}");
        assert!(workspaces.sum::<usize>() >= 3);
    }
}

mod a_launch {
    use super::*;

    #[rstest]
    #[case::in_a_workspace(Place::Workspace { workspace: MAIN })]
    #[case::in_a_new_worktree(Place::Worktree { project: SHOP, branch: "add-coupons".to_owned() })]
    #[tokio::test(start_paused = true)]
    async fn adds_a_row_that_works_and_then_finishes(#[case] place: Place) {
        let (store, _) = entered().await;

        let started = launched(&store, place).await;
        time::sleep(Duration::from_secs(1)).await;
        let working = status(&store, started.pane);
        time::sleep(THINKING_AT_MOST).await;

        assert_eq!(working.as_deref(), Some("working"));
        assert_eq!(status(&store, started.pane).as_deref(), Some("done"));
    }

    #[tokio::test(start_paused = true)]
    async fn finishes_with_a_message() {
        let (store, _) = entered().await;
        let started = launched(&store, Place::Workspace { workspace: MAIN }).await;
        time::sleep(THINKING_AT_MOST).await;

        let transport = store.transport().expect("the demo is connected");
        let message: LastMessage =
            transport::json(transport.as_ref(), last_message::args(started.pane)).await.expect("a last message");

        assert_eq!((message.agent.as_str(), message.turn_over), ("codex", Some(true)));
        assert_ne!(message.text, "");
    }

    #[tokio::test(start_paused = true)]
    async fn puts_the_new_worktree_on_the_board_with_its_branch() {
        let (store, _) = entered().await;
        let started = launched(&store, Place::Worktree { project: SHOP, branch: "add-coupons".to_owned() }).await;
        time::sleep(Duration::from_secs(1)).await;

        let session = store.session().expect("the demo has a session");
        let mut workspaces = session.projects.iter().flat_map(|project| &project.workspaces);
        let workspace = workspaces.find(|workspace| workspace.id == started.workspace).expect("the new worktree");

        assert_eq!(
            (started.project, workspace.branch.as_deref(), workspace.worktree),
            (SHOP, Some("add-coupons"), true)
        );
        assert_eq!(workspace.path.as_deref(), Some("/Users/demo/src/acme-store-add-coupons"));
    }

    #[tokio::test(start_paused = true)]
    async fn gives_each_agent_its_own_tab_and_pane() {
        let (store, _) = entered().await;
        let first = launched(&store, Place::Workspace { workspace: MAIN }).await;
        let second = launched(&store, Place::Workspace { workspace: MAIN }).await;
        assert_ne!((first.tab, first.pane), (second.tab, second.pane));
        assert_eq!(store.agents().expect("a board").iter().filter(|row| row.pane == first.pane).count(), 1);
    }

    #[tokio::test(start_paused = true)]
    async fn starts_an_issue_like_a_prompt() {
        let (store, _) = entered().await;
        let issue = StartIssue {
            place: Place::Workspace { workspace: MAIN },
            agent: "claude".to_owned(),
            model: String::new(),
            key: "SHOP-12".to_owned(),
            source: "github".to_owned(),
            issue_branch: String::new(),
            accounts: Vec::new(),
        };
        let started = start::started(&asked(&store, issue.argv()).await.expect("the issue starts")).expect("a start");
        assert_eq!(started.workspace, MAIN);
    }

    #[tokio::test(start_paused = true)]
    async fn is_refused_in_a_workspace_the_demo_lacks() {
        let (store, _) = entered().await;
        let refused = asked(&store, form(Place::Workspace { workspace: 999 }).argv()).await;
        assert!(matches!(refused, Err(Error::Failed { code: 2, .. })), "{refused:?}");
    }
}
