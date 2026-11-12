use cornercase_core::cli::start::Place;
use cornercase_core::transport::fake::FakeTransport;
use rstest::rstest;
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
const START: [&str; 8] =
    ["start", "claude", "--project", "5", "--worktree=fix-login", "--model=opus", "--prompt=Fix the login", "--placed"];

fn with_projects(projects: &Value) -> String {
    json!({ "boot": BOOT, "revision": 7, "version": "first", "projects": projects }).to_string()
}

fn two_projects() -> String {
    with_projects(&json!([
        { "id": 5, "name": "web", "custom_name": "Shop", "path": "/Users/me/web", "workspaces": [
            { "id": 12, "name": "main", "custom_name": null, "worktree": false },
            { "id": 9, "name": "fix-login", "custom_name": "Login", "worktree": true },
        ] },
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
    use crate::view::WorkspaceChoice;

    fn choice_of(id: u64, name: &str) -> WorkspaceChoice {
        WorkspaceChoice { id, name: name.to_owned() }
    }

    #[tokio::test(start_paused = true)]
    async fn names_each_project_and_its_workspaces_as_the_mac_does_in_the_order_of_the_session_with_the_server_s_boot()
    {
        let studio = connected(serving(&[two_projects()])).await;
        let shop = ProjectChoice {
            id: 5,
            name: "Shop".to_owned(),
            workspaces: vec![choice_of(12, "main"), choice_of(9, "Login")],
        };
        let expected = [shop, ProjectChoice { id: 2, name: "api".to_owned(), workspaces: Vec::new() }];
        let choices = StartChoices { boot: Some(BOOT.to_owned()), projects: expected.to_vec() };
        assert_eq!(projects(&studio.state), Ok(choices));
    }

    #[tokio::test(start_paused = true)]
    async fn gives_the_boot_without_any_agent_on_the_board() {
        let studio = connected(serving(&[with_projects(&json!([]))])).await;
        assert_eq!(projects(&studio.state), Ok(StartChoices { boot: Some(BOOT.to_owned()), projects: Vec::new() }));
    }

    #[tokio::test(start_paused = true)]
    async fn lists_none_before_the_first_snapshot() {
        let studio = connected(serving(&[])).await;
        assert_eq!(projects(&studio.state).map(|choices| choices.projects), Ok(Vec::new()));
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
        assert_eq!(error, CommandError { code: ErrorCode::Failed, message: "accounts are locked".to_owned() });
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
        place: Place::Worktree { project: 5, branch: "fix-login".to_owned() },
        agent: "claude".to_owned(),
        model: "opus".to_owned(),
        prompt: "  Fix the login\n".to_owned(),
        accounts: vec![],
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

    #[tokio::test(start_paused = true)]
    async fn sends_a_workspace_that_exists_as_where_the_tab_opens() {
        let argv =
            ["start", "claude", "--workspace", "12", "--model=opus", "--prompt=Fix the login", "--placed", "--json"];
        let studio = connected(serving(&[two_projects()]).answer(&argv, &[STARTED], &[], 0)).await;
        let form = StartAgent { place: Place::Workspace { workspace: 12 }, ..form() };
        assert_eq!(run(&studio.state, form.argv()).await, Ok(ids(3, 12, 40, 57)));
    }

    #[tokio::test]
    async fn refuses_without_a_connected_machine() {
        let error = run(&Studio::new().state, form().argv()).await.expect_err("no machine");
        assert_eq!(error.code, ErrorCode::NotConnected);
    }
}

mod issue {
    use super::*;

    fn form(place: Place) -> StartIssue {
        StartIssue {
            place,
            agent: "claude".to_owned(),
            model: String::new(),
            key: "#41".to_owned(),
            source: "github".to_owned(),
            issue_branch: "issue-41-coupon".to_owned(),
            accounts: vec![],
        }
    }

    async fn started_in(place: Place, argv: &[&str]) -> Result<Started, CommandError> {
        let studio = connected(serving(&[two_projects()]).answer(argv, &[STARTED], &[], 0)).await;
        run(&studio.state, form(place).argv()).await
    }

    async fn started_on(branch: &str, argv: &[&str]) -> Result<Started, CommandError> {
        started_in(Place::Worktree { project: 5, branch: branch.to_owned() }, argv).await
    }

    #[tokio::test(start_paused = true)]
    async fn sends_cornercase_s_start_on_the_issue_s_own_branch_and_answers_its_ids() {
        let argv = ["start", "claude", "--project", "5", "--issue=#41", "--source=github", "--placed", "--json"];
        assert_eq!(started_on("issue-41-coupon", &argv).await, Ok(ids(3, 12, 40, 57)));
    }

    #[tokio::test(start_paused = true)]
    async fn sends_another_branch_as_the_worktree() {
        let argv = [
            "start",
            "claude",
            "--project",
            "5",
            "--issue=#41",
            "--source=github",
            "--worktree=fix",
            "--placed",
            "--json",
        ];
        assert_eq!(started_on("fix", &argv).await, Ok(ids(3, 12, 40, 57)));
    }

    #[tokio::test(start_paused = true)]
    async fn sends_a_workspace_that_exists_without_a_branch() {
        let argv = ["start", "claude", "--workspace", "12", "--issue=#41", "--source=github", "--placed", "--json"];
        assert_eq!(started_in(Place::Workspace { workspace: 12 }, &argv).await, Ok(ids(3, 12, 40, 57)));
    }
}

mod answered {
    use super::*;

    fn read(text: &str) -> Result<String, Error> {
        if text == "bad" { Err(Error::Json("not JSON".to_owned())) } else { Ok(text.to_owned()) }
    }

    #[rstest]
    #[case::done(Ok(Outcome::Done("ids".to_owned())), Ok("ids".to_owned()))]
    #[case::unreadable(Ok(Outcome::Done("bad".to_owned())), Err(Error::Json("not JSON".to_owned()).into()))]
    #[case::skipped(Ok(Outcome::Skipped), Err(CommandError::not_sent()))]
    #[case::unanswered(Err(Error::Unanswered), Err(Error::Unanswered.into()))]
    #[case::refused(
        Err(Error::Failed { code: 1, stderr: "Error: no such project\n".to_owned() }),
        Err(CommandError { code: ErrorCode::Failed, message: "no such project".to_owned() })
    )]
    fn maps_what_the_queue_answered(
        #[case] outcome: Result<Outcome, Error>,
        #[case] mapped: Result<String, CommandError>,
    ) {
        assert_eq!(answered(outcome, read), mapped);
    }
}

mod over_ipc {
    use super::*;

    #[tokio::test(start_paused = true)]
    async fn answers_the_projects_of_the_active_machine() {
        let studio = connected(serving(&[two_projects()])).await;
        let ipc = Ipc::new(studio.state);
        let projects = json!([
            { "id": 5, "name": "Shop", "workspaces": [{ "id": 12, "name": "main" }, { "id": 9, "name": "Login" }] },
            { "id": 2, "name": "api", "workspaces": [] },
        ]);
        assert_eq!(ipc.invoke("start_projects", json!({})), Ok(json!({ "boot": BOOT, "projects": projects })));
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
        let agent = json!({
            "place": { "kind": "workspace", "workspace": 12 }, "agent": "claude", "model": "", "prompt": "Fix",
            "accounts": [{ "agent": "claude", "account": "work" }]
        });
        let started = ipc.invoke("start_agent", json!({ "form": agent })).expect_err("no machine");
        let issue = json!({
            "place": { "kind": "worktree", "project": 5, "branch": "issue-41" }, "agent": "claude", "model": "",
            "key": "#41", "source": "github", "issue_branch": "issue-41", "accounts": []
        });
        let from_issue = ipc.invoke("start_issue", json!({ "form": issue })).expect_err("no machine");
        let codes = [&started["code"], &from_issue["code"]];
        assert_eq!(codes, [&json!("not_connected"); 2]);
    }
}
