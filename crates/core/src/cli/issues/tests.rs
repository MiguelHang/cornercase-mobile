use rstest::rstest;
use serde_json::json;

use super::*;
use crate::cli::issues::fixtures::{CONNECTED, NONE_CONNECTED, PARTIAL_LIST, WITH_COMMENTS};
use crate::error::Error;
use crate::transport::fake::FakeTransport;
use crate::transport::partial_json;

fn sources(text: &str) -> Sources {
    serde_json::from_str(text).expect("a project's issue trackers")
}

fn lists(text: &str) -> Lists {
    serde_json::from_str(text).expect("a list of issues")
}

fn read_issue(text: &str) -> ReadIssue {
    serde_json::from_str(text).expect("a read issue")
}

fn states(read: &Sources) -> Vec<(&str, SourceState)> {
    read.sources.iter().map(|source| (source.source.as_str(), source.state)).collect()
}

fn filter(source: Option<&str>, closed: bool, mine: bool, cached: bool) -> ListFilter {
    ListFilter { source: source.map(str::to_owned), closed, mine, cached }
}

mod the_sources {
    use super::*;

    #[test]
    fn read_each_tracker_with_its_state() {
        let read = sources(CONNECTED);
        assert_eq!(read.project, 7);
        let expected = [
            ("github", SourceState::Connected),
            ("shortcut", SourceState::Connected),
            ("linear", SourceState::Missing),
        ];
        assert_eq!(states(&read), expected);
    }

    #[test]
    fn read_a_connected_tracker_in_the_repository() {
        let expected = SourceInfo {
            source: "github".to_owned(),
            name: "GitHub".to_owned(),
            repository: true,
            state: SourceState::Connected,
            missing: vec![],
            reason: None,
        };
        assert_eq!(sources(CONNECTED).sources[0], expected);
    }

    #[test]
    fn read_what_each_tracker_misses_when_none_is_connected() {
        let read = sources(NONE_CONNECTED);
        let missing: Vec<_> = read.sources.iter().map(|source| source.missing.clone()).collect();
        assert_eq!(missing, [vec!["repository"], vec![], vec!["site", "email", "token"]]);
        assert!(read.sources.iter().all(|source| source.state != SourceState::Connected), "{read:?}");
    }

    #[test]
    fn read_why_a_tracker_is_failing() {
        let read = sources(NONE_CONNECTED);
        let failing = &read.sources[1];
        assert_eq!(failing.state, SourceState::Failing);
        assert_eq!(failing.reason.as_deref(), Some("Linear rejected the token: it may have expired"));
    }
}

mod a_list {
    use super::*;

    #[test]
    fn reads_each_issue_with_what_its_row_shows() {
        let expected = IssueInfo {
            source: "github".to_owned(),
            key: "#41".to_owned(),
            title: "Checkout forgets the coupon after a refresh".to_owned(),
            state: "open".to_owned(),
            closed: false,
            labels: vec!["bug".to_owned(), "checkout".to_owned()],
            author: "grace".to_owned(),
            updated_at: "2026-11-09T09:20:24Z".to_owned(),
            branch: "issue-41-checkout-forgets-the-coupon".to_owned(),
        };
        let read = lists(PARTIAL_LIST);
        assert_eq!((read.project, read.issues.len()), (7, 2));
        assert_eq!(read.issues[0], expected);
    }

    #[test]
    fn reads_a_closed_issue() {
        let read = lists(PARTIAL_LIST);
        let closed = &read.issues[1];
        assert_eq!((closed.key.as_str(), closed.state.as_str(), closed.closed), ("#38", "closed", true));
        assert!(closed.labels.is_empty(), "{closed:?}");
    }

    #[test]
    fn reads_how_many_each_tracker_listed_and_which_failed() {
        let read = lists(PARTIAL_LIST);
        let github = ListedInfo { source: "github".to_owned(), count: 2, cached: false, error: None };
        let shortcut = ListedInfo {
            source: "shortcut".to_owned(),
            count: 0,
            cached: false,
            error: Some("Shortcut rejected the token".to_owned()),
        };
        assert_eq!(read.sources, [github, shortcut]);
    }
}

mod a_read_issue {
    use super::*;

    #[test]
    fn reads_the_issue_its_body_and_the_prompt_cornercase_will_use() {
        let read = read_issue(WITH_COMMENTS);
        assert_eq!(
            (read.issue.key.as_str(), read.issue.branch.as_str()),
            ("#41", "issue-41-checkout-forgets-the-coupon")
        );
        assert!(read.body.starts_with("Steps:\n\n1. Add `SPRING10`"), "{}", read.body);
        assert_eq!(read.prompt, "https://github.com/acme/shop/issues/41");
    }

    #[test]
    fn reads_its_comments_in_order() {
        let first = CommentInfo {
            author: "ada".to_owned(),
            created_at: "2026-11-08T10:00:00Z".to_owned(),
            body: "It happens on Safari too.".to_owned(),
        };
        let read = read_issue(WITH_COMMENTS);
        let authors: Vec<_> = read.comments.iter().map(|comment| comment.author.as_str()).collect();
        assert_eq!(authors, ["ada", "grace"]);
        assert_eq!(read.comments[0], first);
    }
}

mod leniency {
    use super::*;

    #[rstest]
    #[case::connected("connected", SourceState::Connected)]
    #[case::missing("missing", SourceState::Missing)]
    #[case::failing("failing", SourceState::Failing)]
    #[case::a_new_state("paused", SourceState::Other)]
    fn reads_each_tracker_state(#[case] state: &str, #[case] expected: SourceState) {
        let line = json!({ "sources": [{ "source": "github", "state": state }] });
        assert_eq!(sources(&line.to_string()).sources[0].state, expected);
    }

    #[test]
    fn ignores_the_fields_it_does_not_use() {
        let line = json!({
            "boot": "4242-1700000000", "project": 3, "closed": true, "places": [{ "project": 3 }],
            "sources": [{ "source": "plane", "tab": true, "env": "PLANE_API_KEY", "people": ["me", "anyone"] }]
        });
        let expected = SourceInfo { source: "plane".to_owned(), ..SourceInfo::default() };
        assert_eq!(sources(&line.to_string()).sources, [expected]);
    }

    #[test]
    fn reads_missing_fields_as_their_defaults() {
        assert_eq!(sources("{}"), Sources::default());
        assert_eq!(lists("{}"), Lists::default());
        assert_eq!(read_issue("{}"), ReadIssue::default());
        let bare = sources(&json!({ "sources": [{}] }).to_string());
        assert_eq!((bare.sources[0].state, bare.sources[0].reason.as_deref()), (SourceState::Other, None));
    }

    #[test]
    fn reads_a_filter_from_the_app_with_missing_fields() {
        let read: ListFilter = serde_json::from_value(json!({ "mine": true })).expect("a filter");
        assert_eq!(read, filter(None, false, true, false));
    }
}

mod the_arguments {
    use super::*;

    #[test]
    fn ask_for_a_project_s_trackers() {
        assert_eq!(sources_args(7), ["issue", "sources", "--project", "7", "--json"]);
    }

    #[rstest]
    #[case::open_issues_of_every_tracker(filter(None, false, false, false), &[])]
    #[case::closed_ones_too(filter(None, true, false, false), &["--closed"])]
    #[case::mine(filter(None, false, true, false), &["--assignee", "me"])]
    #[case::from_the_cache(filter(None, false, false, true), &["--cached"])]
    #[case::one_tracker(filter(Some("gitlab"), false, false, false), &["--source=gitlab"])]
    #[case::a_blank_tracker_as_every_one(filter(Some(" "), false, false, false), &[])]
    #[case::every_filter(
        filter(Some(" linear "), true, true, true),
        &["--source=linear", "--closed", "--assignee", "me", "--cached"]
    )]
    fn list_with_the_filters(#[case] filter: ListFilter, #[case] flags: &[&str]) {
        let mut expected = vec!["issue", "list", "--project", "7"];
        expected.extend(flags);
        expected.push("--json");
        assert_eq!(list_args(7, &filter), expected);
    }

    #[rstest]
    #[case::a_github_key("github", "#41", &["--source=github", "--json", "--", "#41"])]
    #[case::a_key_like_an_option("shortcut", "-48", &["--source=shortcut", "--json", "--", "-48"])]
    #[case::a_key_with_spaces_around("jira", " ENG-42 ", &["--source=jira", "--json", "--", "ENG-42"])]
    #[case::no_tracker("", "#41", &["--json", "--", "#41"])]
    fn read_an_issue_with_its_key_after_the_dashes(#[case] source: &str, #[case] key: &str, #[case] rest: &[&str]) {
        let mut expected = vec!["issue", "read", "--project", "9"];
        expected.extend(rest);
        assert_eq!(read_args(9, source, key), expected);
    }
}

mod a_partial_list {
    use super::*;

    const LIST: [&str; 5] = ["issue", "list", "--project", "7", "--json"];

    async fn listed(fake: &FakeTransport) -> Result<Lists, Error> {
        partial_json(fake, list_args(7, &ListFilter::default())).await
    }

    #[tokio::test]
    async fn reads_the_issues_of_the_trackers_that_answered_when_one_failed_with_exit_3() {
        let fake =
            FakeTransport::default().answer(&LIST, &[PARTIAL_LIST], &["Shortcut: Shortcut rejected the token"], 3);
        let read = listed(&fake).await.expect("the partial list");
        assert_eq!(read.issues.len(), 2);
        assert_eq!(read.sources[1].error.as_deref(), Some("Shortcut rejected the token"));
    }

    #[tokio::test]
    async fn fails_with_the_stderr_when_the_project_is_unknown() {
        let fake = FakeTransport::default().answer(&LIST, &[], &["error: no project 7"], 1);
        assert_eq!(listed(&fake).await, Err(Error::Failed { code: 1, stderr: "error: no project 7".to_owned() }));
    }
}
