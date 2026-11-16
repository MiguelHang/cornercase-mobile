use crate::cli::flows::{self, Flows};
use crate::cli::issues::{self, ListFilter, Lists, SourceState, Sources};
use crate::demo::DemoTransport;
use crate::transport::{self, partial_json};

const SHOP: u64 = 1;

#[tokio::test]
async fn the_issues_have_no_tracker_connected() {
    let sources: Sources = transport::json(&DemoTransport::new(), issues::sources_args(SHOP)).await.expect("no error");
    assert_eq!(sources.project, SHOP);
    assert!(!sources.sources.iter().any(|source| source.state == SourceState::Connected), "{sources:?}");
}

#[tokio::test]
async fn the_issue_list_is_empty() {
    let argv = issues::list_args(SHOP, &ListFilter::default());
    let lists: Lists = partial_json(&DemoTransport::new(), argv).await.expect("no error");
    assert_eq!((lists.project, lists.issues, lists.sources), (SHOP, Vec::new(), Vec::new()));
}

#[tokio::test]
async fn the_flows_are_empty() {
    let flows: Flows = transport::json(&DemoTransport::new(), flows::list_args(SHOP)).await.expect("no error");
    assert_eq!(flows, Flows::default());
}
