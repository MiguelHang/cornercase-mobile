use rstest::rstest;

use super::*;
use crate::board::fixtures::{agent, child};

const EARLIER: &str = "2026-11-09T12:00:00.000Z";
const LATER: &str = "2026-11-09T12:05:00.000Z";
const CHAIN: [(u64, u32, Option<u64>); 3] = [(4, 0, None), (5, 1, Some(4)), (6, 2, Some(5))];

fn items(panes: &[Pane]) -> Vec<Item<'_>> {
    panes.iter().map(|pane| Item { pane, since: None }).collect()
}

fn shape(panes: &[Pane]) -> Vec<(u64, u32, Option<u64>)> {
    order(&items(panes)).iter().map(|node| (panes[node.at].id, node.depth, node.parent)).collect()
}

fn ids(nodes: &[Node], listed: &[Pane]) -> Vec<u64> {
    nodes.iter().map(|node| listed[node.at].id).collect()
}

fn sequence(listed: &[Pane]) -> Vec<u64> {
    ids(&order(&items(listed)), listed)
}

fn rank_of<'a>(items: &[Item<'a>], pane: u64) -> Rank<'a> {
    let nodes = order(items);
    nodes.iter().find(|node| items[node.at].pane.id == pane).expect("a node for the pane").rank
}

mod a_parent {
    use super::*;

    #[test]
    fn hangs_its_children_under_it_one_level_deeper() {
        let listed = [agent(4, "idle"), child(5, 4, "idle"), child(6, 5, "idle")];
        assert_eq!(shape(&listed), CHAIN);
    }

    #[test]
    fn listed_after_its_child_still_comes_first() {
        let listed = [child(5, 4, "idle"), agent(6, "idle"), agent(4, "idle")];
        assert_eq!(shape(&listed), vec![(6, 0, None), (4, 0, None), (5, 1, Some(4))]);
    }

    #[test]
    fn that_is_not_listed_leaves_its_child_a_root() {
        let listed = [agent(4, "idle"), child(5, 9, "idle")];
        assert_eq!(shape(&listed), vec![(4, 0, None), (5, 0, None)]);
    }

    #[test]
    fn that_is_the_pane_itself_counts_as_none() {
        let listed = [child(4, 4, "idle"), agent(5, "idle")];
        assert_eq!(shape(&listed), vec![(4, 0, None), (5, 0, None)]);
    }

    #[test]
    fn closed_in_the_middle_leaves_the_grandchild_a_root_of_its_own() {
        let listed = [agent(4, "idle"), child(6, 5, "idle")];
        assert_eq!(shape(&listed), vec![(4, 0, None), (6, 0, None)]);
    }
}

mod a_ring {
    use super::*;

    #[test]
    fn of_two_hangs_from_the_first_one_listed() {
        let listed = [child(4, 5, "idle"), child(5, 4, "idle")];
        assert_eq!(shape(&listed), vec![(4, 0, None), (5, 1, Some(4))]);
    }

    #[test]
    fn of_three_keeps_every_row_once() {
        let listed = [child(4, 6, "idle"), child(5, 4, "idle"), child(6, 5, "idle")];
        assert_eq!(shape(&listed), CHAIN);
    }

    #[test]
    fn takes_its_place_among_the_roots_in_the_session_order() {
        let listed = [child(4, 5, "idle"), child(5, 4, "idle"), agent(6, "idle")];
        assert_eq!(sequence(&listed), vec![4, 5, 6]);
    }
}

mod siblings {
    use super::*;

    #[test]
    fn put_the_louder_status_first() {
        let listed = [agent(4, "idle"), child(5, 4, "working"), child(6, 4, "waiting")];
        assert_eq!(sequence(&listed), vec![4, 6, 5]);
    }

    #[test]
    fn keep_the_session_order_on_a_tie() {
        let listed = [agent(4, "idle"), child(5, 4, "idle"), child(6, 4, "idle")];
        assert_eq!(sequence(&listed), vec![4, 5, 6]);
    }

    #[test]
    fn put_the_longest_in_its_status_first() {
        let listed = [agent(4, "idle"), child(5, 4, "waiting"), child(6, 4, "waiting")];
        let mut items = items(&listed);
        items[1].since = Some(LATER);
        items[2].since = Some(EARLIER);
        assert_eq!(ids(&order(&items), &listed), vec![4, 6, 5]);
    }

    #[test]
    fn are_ordered_by_their_own_branch() {
        let listed = [agent(4, "idle"), child(5, 4, "working"), child(6, 4, "idle"), child(7, 6, "failed")];
        assert_eq!(sequence(&listed), vec![4, 6, 7, 5]);
    }
}

mod a_root {
    use super::*;

    #[rstest]
    #[case::waiting("waiting")]
    #[case::failed("failed")]
    fn is_raised_by_a_child_that_needs_attention(#[case] status: &str) {
        let listed = [agent(4, "idle"), agent(5, "working"), child(6, 4, status)];
        assert_eq!(sequence(&listed), vec![4, 6, 5]);
    }

    #[rstest]
    #[case::done("done")]
    #[case::pending("pending")]
    #[case::working("working")]
    fn is_not_raised_by_a_child_that_does_not(#[case] status: &str) {
        let listed = [agent(4, "idle"), agent(5, "working"), child(6, 4, status)];
        assert_eq!(sequence(&listed), vec![5, 4, 6]);
    }

    #[test]
    fn is_raised_by_a_grandchild() {
        let listed = [agent(4, "idle"), child(5, 4, "idle"), child(6, 5, "waiting"), agent(7, "working")];
        assert_eq!(sequence(&listed), vec![4, 5, 6, 7]);
    }

    #[test]
    fn raised_alike_puts_the_one_raised_for_longest_first() {
        let listed = [agent(4, "idle"), child(5, 4, "waiting"), agent(6, "idle"), child(7, 6, "waiting")];
        let mut items = items(&listed);
        items[1].since = Some(LATER);
        items[3].since = Some(EARLIER);
        assert_eq!(ids(&order(&items), &listed), vec![6, 7, 4, 5]);
    }
}

mod the_rank {
    use super::*;

    #[test]
    fn of_a_branch_nobody_raises_is_its_own() {
        let listed = [agent(4, "working"), child(5, 4, "done")];
        let mut items = items(&listed);
        items[0].since = Some(EARLIER);
        assert_eq!(rank_of(&items, 4), Rank { status: Some("working"), since: Some(EARLIER) });
    }

    #[test]
    fn raised_takes_the_status_and_time_of_the_descendant_that_raises_it() {
        let listed = [agent(4, "working"), child(5, 4, "idle"), child(6, 5, "failed")];
        let mut items = items(&listed);
        items[0].since = Some(EARLIER);
        items[2].since = Some(LATER);
        assert_eq!(rank_of(&items, 4), Rank { status: Some("failed"), since: Some(LATER) });
        assert_eq!(rank_of(&items, 5), Rank { status: Some("failed"), since: Some(LATER) });
    }

    #[test]
    fn stays_its_own_when_it_is_louder_than_what_its_children_raise() {
        let listed = [agent(4, "waiting"), child(5, 4, "failed")];
        let mut items = items(&listed);
        items[0].since = Some(LATER);
        assert_eq!(rank_of(&items, 4), Rank { status: Some("waiting"), since: Some(LATER) });
    }

    #[test]
    fn takes_the_louder_of_two_children_that_raise_it() {
        let listed = [agent(4, "idle"), child(5, 4, "failed"), child(6, 4, "waiting")];
        let mut items = items(&listed);
        items[1].since = Some(EARLIER);
        items[2].since = Some(LATER);
        assert_eq!(rank_of(&items, 4), Rank { status: Some("waiting"), since: Some(LATER) });
    }

    #[test]
    fn alike_takes_the_longest_time() {
        let listed = [agent(4, "waiting"), child(5, 4, "waiting")];
        let mut items = items(&listed);
        items[0].since = Some(LATER);
        items[1].since = Some(EARLIER);
        assert_eq!(rank_of(&items, 4), Rank { status: Some("waiting"), since: Some(EARLIER) });
    }

    #[test]
    fn passes_up_only_what_needs_attention() {
        let listed = [agent(4, "idle"), child(5, 4, "working"), child(6, 5, "idle")];
        assert_eq!(rank_of(&items(&listed), 4), Rank { status: Some("idle"), since: None });
    }
}
