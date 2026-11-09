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

const STARTED: &str =
    include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../crates/core/tests/fixtures/control/start/0.13.19.json"));
const CREATED: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../crates/core/tests/fixtures/control/new-workspace/0.13.19.json"
));
const START: [&str; 8] =
    ["start", "claude", "--project", "5", "--worktree=fix-login", "--model=opus", "--prompt=Fix the login", "--placed"];

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

fn start_argv() -> Vec<&'static str> {
    START.into_iter().chain(["--json"]).collect()
}

fn form() -> StartAgent {
    StartAgent {
        project: 5,
        agent: "claude".to_owned(),
        model: "opus".to_owned(),
        prompt: "  Fix the login\n".to_owned(),
        worktree: "fix-login".to_owned(),
    }
}

fn ids(project: u64, workspace: u64, tab: u64, pane: u64) -> Started {
    Started { project, workspace, tab, pane, warning: None }
}

async fn starting(stdout: &[&str], stderr: &[&str], code: i32) -> Result<Started, CommandError> {
    let fake = serving(&[two_projects()]).answer(&start_argv(), stdout, stderr, code);
    let studio = connected(fake).await;
    run(&studio.state, form().argv()).await
}

mod agent {
    use super::*;

    #[tokio::test(start_paused = true)]
    async fn sends_the_form_as_cornercase_s_start_and_answers_its_ids() {
        assert_eq!(starting(&[STARTED], &[], 0).await, Ok(ids(3, 12, 40, 57)));
    }

    #[tokio::test(start_paused = true)]
    async fn passes_on_the_warning_cornercase_gives() {
        let answer = json!({ "project": 3, "workspace": 12, "tab": 40, "pane": 57, "warning": "setup failed" });
        let expected = Started { warning: Some("setup failed".to_owned()), ..ids(3, 12, 40, 57) };
        assert_eq!(starting(&[&answer.to_string()], &[], 0).await, Ok(expected));
    }

    #[tokio::test(start_paused = true)]
    async fn says_what_cornercase_refused_without_its_prefix() {
        let refused = ["Error: `fix-login` is already checked out in another worktree"];
        let error = starting(&[], &refused, 1).await.expect_err("a refusal");
        let message = "`fix-login` is already checked out in another worktree";
        assert_eq!(error, CommandError { code: ErrorCode::Failed, message: message.to_owned() });
    }

    #[tokio::test(start_paused = true)]
    async fn says_how_cornercase_ended_when_it_gave_no_reason() {
        let error = starting(&[], &[], 2).await.expect_err("a failure");
        assert_eq!(error, Error::Failed { code: 2, stderr: String::new() }.into());
    }

    #[tokio::test(start_paused = true)]
    async fn says_an_answer_that_is_not_json() {
        let error = starting(&["started"], &[], 0).await.expect_err("not JSON");
        assert_eq!(error.code, ErrorCode::Json);
    }

    #[tokio::test(start_paused = true)]
    async fn keeps_the_code_of_a_request_left_unanswered() {
        let fake = serving(&[two_projects()]).fail(&start_argv(), Error::Unanswered);
        let studio = connected(fake).await;
        let error = run(&studio.state, form().argv()).await.expect_err("unanswered");
        assert_eq!(error.code, ErrorCode::Unanswered);
    }

    #[tokio::test]
    async fn refuses_without_a_connected_machine() {
        let error = run(&Studio::new().state, form().argv()).await.expect_err("no machine");
        assert_eq!(error.code, ErrorCode::NotConnected);
    }
}

mod workspace {
    use super::*;

    #[tokio::test(start_paused = true)]
    async fn sends_the_form_as_cornercase_s_new_workspace_and_answers_its_ids() {
        let argv = ["new-workspace", "--project", "2", "--worktree", "--json", "--", "fix-login"];
        let studio = connected(serving(&[two_projects()]).answer(&argv, &[CREATED], &[], 0)).await;
        let form = NewWorkspace { project: 2, name: " fix-login ".to_owned(), worktree: true };
        assert_eq!(run(&studio.state, form.argv()).await, Ok(ids(7, 13, 41, 58)));
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

    #[test]
    fn reads_both_forms_and_refuses_them_without_a_connected_machine() {
        let ipc = Ipc::new(Studio::new().reloaded());
        let agent = json!({ "project": 5, "agent": "claude", "model": "", "prompt": "Fix", "worktree": "" });
        let started = ipc.invoke("start_agent", json!({ "form": agent })).expect_err("no machine");
        let workspace = json!({ "project": 5, "name": "fix", "worktree": false });
        let created = ipc.invoke("new_workspace", json!({ "form": workspace })).expect_err("no machine");
        assert_eq!((&started["code"], &created["code"]), (&json!("not_connected"), &json!("not_connected")));
    }
}
