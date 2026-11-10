use cornercase_core::cli::start::Started;
use cornercase_core::error::Error;
use cornercase_core::queue::Outcome;
use cornercase_core::transport::fake::FakeTransport;
use rstest::rstest;
use serde_json::{Value, json};

use super::*;
use crate::commands::error::ErrorCode;
use crate::test_util::{BOOT, Ipc, Studio, connected_to, connected_with, serving, settle, snapshot};

const PROJECT: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../crates/core/tests/fixtures/control/flow-list/0.13.31/project.json"
));
const RUN: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../crates/core/tests/fixtures/control/flow-run/0.13.31/run.json"
));
const LIST: [&str; 5] = ["flow", "list", "--project", "8", "--json"];
const ISSUE_BRANCH: &str = "issue-482-login-form-forgets-the-email";
const NOTE: &str = "this project's flow passes --search to `coder`; they run without asking, so check them";

async fn listed(stdout: &str, stderr: &str, code: i32) -> Result<Value, Value> {
    let (stdout, stderr): (Vec<&str>, Vec<&str>) = (stdout.lines().collect(), stderr.lines().collect());
    let ipc = connected_with(|fake| fake.answer(&LIST, &stdout, &stderr, code)).await;
    ipc.invoke("issue_flows", json!({ "project": 8 }))
}

fn form(branch: &str) -> StartFlow {
    StartFlow {
        project: 8,
        name: "Ship it".to_owned(),
        key: "#482".to_owned(),
        source: "github".to_owned(),
        branch: branch.to_owned(),
        issue_branch: ISSUE_BRANCH.to_owned(),
    }
}

fn run_argv(middle: &[&'static str]) -> Vec<&'static str> {
    let head = ["flow", "run", "--project", "8", "--issue=#482", "--source=github"];
    head.into_iter().chain(middle.iter().copied()).chain(["--json", "--", "Ship it"]).collect()
}

async fn running(fake: FakeTransport, branch: &str) -> Result<FlowStarted, CommandError> {
    let studio = connected_to(fake);
    settle().await;
    run_flow(&studio.state, form(branch).argv()).await
}

async fn answering(stdout: &[&str], stderr: &[&str], code: i32) -> Result<FlowStarted, CommandError> {
    let fake = serving(&[snapshot(BOOT, 7, "first")]).answer(&run_argv(&[]), stdout, stderr, code);
    running(fake, ISSUE_BRANCH).await
}

fn orchestrator() -> FlowStarted {
    let started = Started { project: 8, workspace: 9, tab: 14, pane: 11, warning: Some(NOTE.to_owned()) };
    FlowStarted { started, agent: "claude".to_owned() }
}

mod the_flows {
    use super::*;

    #[tokio::test(start_paused = true)]
    async fn answer_the_project_s_flows_read_at_once() {
        let read = listed(PROJECT, "", 0).await.expect("the flows");
        let flows: Flows = serde_json::from_value(read).expect("the answer reads as flows");
        assert_eq!(flows, serde_json::from_str::<Flows>(PROJECT).expect("the fixture reads"));
        assert_eq!(flows.flows[0].name, "Ship it");
    }

    #[tokio::test(start_paused = true)]
    async fn fail_with_the_reason_cornercase_gave() {
        let refused = listed("", "Error: no project 8\n", 1).await;
        assert_eq!(refused, Err(json!({ "code": "failed", "message": "no project 8" })));
    }

    #[tokio::test(start_paused = true)]
    async fn say_an_answer_that_is_not_json() {
        let refused = listed("Ship it", "", 0).await;
        assert_eq!(refused.expect_err("not JSON")["code"], "json");
    }
}

mod a_run {
    use super::*;

    #[tokio::test(start_paused = true)]
    async fn sends_the_flow_on_the_issue_s_branch_and_opens_the_orchestrator() {
        assert_eq!(answering(&[RUN], &[], 0).await, Ok(orchestrator()));
    }

    #[tokio::test(start_paused = true)]
    async fn sends_another_branch_as_the_worktree() {
        let argv = run_argv(&["--worktree=fix/login-email"]);
        let fake = serving(&[snapshot(BOOT, 7, "first")]).answer(&argv, &[RUN], &[], 0);
        assert_eq!(running(fake, "fix/login-email").await, Ok(orchestrator()));
    }

    #[rstest]
    #[case::a_branch_checked_out_in_the_project(
        "branch main is checked out in the project's own folder, and a flow works in a worktree of its own: pick another branch"
    )]
    #[case::a_workspace_that_already_runs_one("workspace 17 already runs a flow, and a workspace runs one at a time")]
    #[case::a_flow_with_problems("flow Second opinion cannot run: `summary` is in no stage")]
    #[tokio::test(start_paused = true)]
    async fn says_what_cornercase_refused_without_its_prefix(#[case] reason: &str) {
        let error = answering(&[], &[&format!("Error: {reason}")], 1).await.expect_err("a refusal");
        assert_eq!(error, CommandError { code: ErrorCode::Failed, message: reason.to_owned() });
    }

    #[tokio::test(start_paused = true)]
    async fn says_an_answer_that_names_no_agent() {
        let error = answering(&[&json!({ "run": 10, "members": [] }).to_string()], &[], 0).await;
        assert_eq!(error.expect_err("no members").code, ErrorCode::Json);
    }

    #[tokio::test(start_paused = true)]
    async fn keeps_the_code_of_a_request_left_unanswered() {
        let fake = serving(&[snapshot(BOOT, 7, "first")]).fail(&run_argv(&[]), Error::Unanswered);
        let error = running(fake, ISSUE_BRANCH).await.expect_err("unanswered");
        assert_eq!(error.code, ErrorCode::Unanswered);
    }

    #[test]
    fn says_nothing_was_sent_when_the_queue_skipped_it() {
        assert_eq!(answered(Ok(Outcome::Skipped), flows::flow_started), Err(CommandError::not_sent()));
    }

    #[tokio::test]
    async fn refuses_without_a_connected_machine() {
        let error = run_flow(&Studio::new().state, form(ISSUE_BRANCH).argv()).await.expect_err("no machine");
        assert_eq!(error.code, ErrorCode::NotConnected);
    }
}

mod over_ipc {
    use super::*;

    #[test]
    fn reads_the_form_and_refuses_both_without_a_connected_machine() {
        let ipc = Ipc::new(Studio::new().reloaded());
        let listed = ipc.invoke("issue_flows", json!({ "project": 8 })).expect_err("no machine");
        let form = json!({
            "project": 8, "name": "Quick", "key": "#482", "source": "github", "branch": ISSUE_BRANCH,
            "issue_branch": ISSUE_BRANCH
        });
        let started = ipc.invoke("start_flow", json!({ "form": form })).expect_err("no machine");
        assert_eq!([&listed["code"], &started["code"]], [&json!("not_connected"); 2]);
    }
}
