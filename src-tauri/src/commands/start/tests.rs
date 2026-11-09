use cornercase_core::transport::fake::FakeTransport;
use serde_json::{Value, json};

use super::*;
use crate::commands::connection::start;
use crate::commands::error::ErrorCode;
use crate::test_util::{BOOT, Ipc, Recorder, Studio, launcher, serving, settle};

const ACCOUNTS: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../crates/core/tests/fixtures/control/account-list/0.13.19.json"
));

fn with_projects(projects: &Value) -> String {
    json!({ "boot": BOOT, "revision": 7, "version": "first", "projects": projects }).to_string()
}

fn two_projects() -> String {
    with_projects(&json!([
        { "id": 5, "name": "web", "custom_name": "Shop", "path": "/Users/me/web" },
        { "id": 2, "name": "api", "custom_name": null },
    ]))
}

async fn connected(fake: FakeTransport) -> Studio {
    let studio = Studio::new();
    let (launcher, _) = launcher(fake);
    start(&studio.state, Recorder::default(), &launcher, studio.id).expect("connected");
    settle().await;
    studio
}

fn listing(stdout: &str) -> FakeTransport {
    serving(&[two_projects()]).answer(&ACCOUNT_LIST, &[stdout], &[], 0)
}

fn choice(agent: &str, command: &str) -> AgentChoice {
    AgentChoice { agent: agent.to_owned(), command: command.to_owned(), installed: true }
}

mod projects {
    use super::*;

    #[tokio::test(start_paused = true)]
    async fn names_each_project_as_the_mac_does_in_the_order_of_the_session() {
        let studio = connected(serving(&[two_projects()])).await;
        let expected =
            [ProjectChoice { id: 5, name: "Shop".to_owned() }, ProjectChoice { id: 2, name: "api".to_owned() }];
        assert_eq!(projects(&studio.state), Ok(expected.to_vec()));
    }

    #[tokio::test(start_paused = true)]
    async fn lists_none_before_the_first_snapshot() {
        let studio = connected(serving(&[])).await;
        assert_eq!(projects(&studio.state), Ok(Vec::new()));
    }

    #[test]
    fn refuses_without_a_connected_machine() {
        let error = projects(&Studio::new().state).expect_err("no machine");
        assert_eq!(error.code, ErrorCode::NotConnected);
    }
}

mod agents {
    use super::*;

    #[tokio::test(start_paused = true)]
    async fn lists_the_agents_installed_on_the_mac_of_cornercase_0_13_19() {
        let studio = connected(listing(ACCOUNTS)).await;
        assert_eq!(installed(&studio.state).await, Ok(vec![choice("claude", "claude")]));
    }

    #[tokio::test(start_paused = true)]
    async fn keeps_only_the_installed_ones_in_the_order_cornercase_gives() {
        let answer = json!({ "agents": [
            { "agent": "codex", "command": "codex", "installed": true },
            { "agent": "claude", "command": "claude", "installed": false },
            { "agent": "cursor", "command": "cursor-agent", "installed": true },
        ] });
        let studio = connected(listing(&answer.to_string())).await;
        let expected = vec![choice("codex", "codex"), choice("cursor", "cursor-agent")];
        assert_eq!(installed(&studio.state).await, Ok(expected));
    }

    #[tokio::test(start_paused = true)]
    async fn passes_on_the_error_of_the_server() {
        let fake = serving(&[two_projects()]).answer(&ACCOUNT_LIST, &[], &["Error: accounts are locked"], 1);
        let studio = connected(fake).await;
        let error = installed(&studio.state).await.expect_err("a failure");
        assert_eq!(error.code, ErrorCode::Failed);
        assert!(error.message.contains("accounts are locked"), "{}", error.message);
    }

    #[tokio::test(start_paused = true)]
    async fn says_an_answer_that_is_not_json() {
        let studio = connected(listing("claude codex")).await;
        let error = installed(&studio.state).await.expect_err("not JSON");
        assert_eq!(error.code, ErrorCode::Json);
    }

    #[tokio::test]
    async fn refuses_without_a_connected_machine() {
        let error = installed(&Studio::new().state).await.expect_err("no machine");
        assert_eq!(error.code, ErrorCode::NotConnected);
    }
}

mod over_ipc {
    use super::*;

    #[tokio::test(start_paused = true)]
    async fn answers_the_projects_of_the_active_machine() {
        let studio = connected(serving(&[two_projects()])).await;
        let ipc = Ipc::new(studio.state);
        let expected = json!([{ "id": 5, "name": "Shop" }, { "id": 2, "name": "api" }]);
        assert_eq!(ipc.invoke("start_projects", json!({})), Ok(expected));
    }

    #[test]
    fn refuses_both_without_a_connected_machine() {
        let ipc = Ipc::new(Studio::new().reloaded());
        let projects = ipc.invoke("start_projects", json!({})).expect_err("no machine");
        let agents = ipc.invoke("installed_agents", json!({})).expect_err("no machine");
        assert_eq!((&projects["code"], &agents["code"]), (&json!("not_connected"), &json!("not_connected")));
    }
}
