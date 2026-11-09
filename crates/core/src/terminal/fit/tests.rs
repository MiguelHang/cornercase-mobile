use rstest::rstest;
use serde_json::Value;

use super::*;
use crate::cli::status::fixtures::VIEW;
use crate::transport::fake::FakeTransport;

const ARGV: [&str; 7] = ["view", "--tab", "3", "--cols", "120", "--rows", "40"];
const SIZE: Size = Size { cols: 120, rows: 40 };
const GRANTED: Verdict = Verdict::Granted { view: 41, projection: 3 };
const REFUSAL: &str = "view 41 cannot size a tab now: it has no size";

fn line(edit: fn(&mut Value)) -> String {
    let mut line: Value = serde_json::from_str(VIEW).expect("the view fixture");
    edit(&mut line);
    line.to_string()
}

fn unchanged(_: &mut Value) {}

fn without_control(line: &mut Value) {
    line["view"]["controls"] = Value::Bool(false);
}

fn on_another_tab(line: &mut Value) {
    line["view"]["shown"]["tab"] = Value::from(9);
}

fn on_no_tab(line: &mut Value) {
    line["view"]["shown"]["tab"] = Value::Null;
}

fn on_another_tab_without_control(line: &mut Value) {
    on_another_tab(line);
    without_control(line);
}

fn inactive_without_control(line: &mut Value) {
    line["view"]["active"] = Value::Bool(false);
    without_control(line);
}

fn unsized_without_control(line: &mut Value) {
    line["view"]["rows"] = Value::Null;
    without_control(line);
}

fn claim(fake: FakeTransport) -> Claim {
    Claim::open(Arc::new(fake), 3, SIZE)
}

fn printing(lines: &[String]) -> Claim {
    let lines: Vec<&str> = lines.iter().map(String::as_str).collect();
    claim(FakeTransport::default().open(&ARGV, &lines))
}

async fn verdicts(mut claim: Claim, count: usize) -> Vec<Verdict> {
    let mut verdicts = Vec::new();
    for _ in 0..count {
        verdicts.push(claim.next().await);
    }
    verdicts
}

mod a_claim {
    use super::*;

    #[tokio::test]
    async fn is_granted_by_the_first_line_of_the_view_launched_on_the_tab_at_its_size() {
        assert_eq!(verdicts(printing(&[line(unchanged)]), 1).await, [GRANTED]);
    }

    #[tokio::test]
    async fn hands_the_projection_of_each_later_line() {
        let raised = line(|line| line["view"]["projection"] = Value::from(4));
        assert_eq!(
            verdicts(printing(&[line(unchanged), raised, line(unchanged)]), 3).await,
            [GRANTED, Verdict::Projection(4), Verdict::Projection(3)]
        );
    }

    #[rstest]
    #[case::the_view_lost_control(without_control, Verdict::TakenBack)]
    #[case::the_view_settled_on_another_tab(on_another_tab, Verdict::MovedAway)]
    #[case::the_view_shows_no_tab(on_no_tab, Verdict::MovedAway)]
    #[case::moved_away_without_control(on_another_tab_without_control, Verdict::MovedAway)]
    #[case::an_inactive_view_lost_control(inactive_without_control, Verdict::Projection(3))]
    #[case::a_view_without_a_size_lost_control(unsized_without_control, Verdict::Projection(3))]
    #[tokio::test]
    async fn judges_each_line_after_the_grant(#[case] edit: fn(&mut Value), #[case] verdict: Verdict) {
        assert_eq!(verdicts(printing(&[line(unchanged), line(edit)]), 2).await, [GRANTED, verdict]);
    }

    #[rstest]
    #[case::taken_back(without_control, Verdict::TakenBack)]
    #[case::on_another_tab(on_another_tab, Verdict::MovedAway)]
    #[tokio::test]
    async fn is_not_granted_by_a_first_line_that_already_fails(#[case] edit: fn(&mut Value), #[case] verdict: Verdict) {
        assert_eq!(verdicts(printing(&[line(edit), line(unchanged)]), 2).await, [verdict, GRANTED]);
    }
}

mod a_refusal {
    use super::*;

    #[rstest]
    #[case::not_json("{\"view\": ")]
    #[case::not_a_view_line("{\"view\": 41}")]
    #[tokio::test]
    async fn comes_from_a_line_that_is_not_a_view_line(#[case] text: &str) {
        let verdict = claim(FakeTransport::default().open(&ARGV, &[text])).next().await;
        assert!(matches!(verdict, Verdict::Refused(Error::Json(_))), "{verdict:?}");
    }

    #[rstest]
    #[case::the_view_cannot_size_the_tab(1, REFUSAL, Error::Failed { code: 1, stderr: REFUSAL.to_owned() })]
    #[case::the_tab_is_not_found(1, "no tab 3", Error::Failed { code: 1, stderr: "no tab 3".to_owned() })]
    #[case::the_server_restarted(4, "boot changed", Error::StaleBoot("boot changed".to_owned()))]
    #[case::the_view_ended_before_its_first_line(0, "", Error::StreamEnded)]
    #[tokio::test]
    async fn carries_what_cornercase_printed_when_the_view_ends(
        #[case] code: i32,
        #[case] stderr: &str,
        #[case] error: Error,
    ) {
        let fake = FakeTransport::default().answer(&ARGV, &[], &[stderr], code);
        assert_eq!(claim(fake).next().await, Verdict::Refused(error));
    }

    #[tokio::test]
    async fn comes_from_a_lost_connection() {
        let fake = FakeTransport::default().fail(&ARGV, Error::Dropped);
        assert_eq!(claim(fake).next().await, Verdict::Refused(Error::Dropped));
    }

    #[tokio::test]
    async fn follows_the_lines_printed_before_the_view_ended() {
        let fixture = line(unchanged);
        let fake = FakeTransport::default().answer(&ARGV, &[&fixture], &[REFUSAL], 1);
        let refused = Verdict::Refused(Error::Failed { code: 1, stderr: REFUSAL.to_owned() });
        assert_eq!(verdicts(claim(fake), 3).await, [GRANTED, refused.clone(), refused]);
    }
}
