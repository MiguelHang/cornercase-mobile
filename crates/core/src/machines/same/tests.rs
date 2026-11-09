use rstest::rstest;

use super::*;
use crate::machines::tests::{TOKEN, bridge};

const NEW_TOKEN: &str = "bmV3LXRva2Vu";
const OTHER_TOKEN: &str = "b3RoZXItdG9rZW4";
const LAN: &str = "192.168.1.20";
const TAILNET: &str = "fd7a:115c:a1e0::1";
const GONE: &str = "10.0.0.9";
const BOOT: &str = "2026-11-09T08:00:00Z";

fn route(host: &str, token: &str) -> Route {
    bridge(host, 8765, Some(token))
}

fn machine(id: u64, name: &str, routes: Vec<Route>) -> Machine {
    Machine { id: MachineId(id), name: name.to_owned(), routes, last_route: None }
}

fn studio_and_laptop() -> Vec<Machine> {
    vec![machine(0, "Studio", vec![route(LAN, TOKEN)]), machine(1, "laptop", vec![route(TAILNET, NEW_TOKEN)])]
}

fn seen(boots: &[(u64, &str)]) -> HashMap<MachineId, String> {
    boots.iter().map(|(id, boot)| (MachineId(*id), (*boot).to_owned())).collect()
}

fn same(machines: &[Machine], route: &Route, name: &str, probed: Option<&str>, boots: &[(u64, &str)]) -> Option<Same> {
    same_machine(machines, route, name, probed, &seen(boots))
}

fn found(id: u64, why: Why) -> Same {
    Same { machine: MachineId(id), why }
}

mod by_address {
    use super::*;

    #[test]
    fn offers_to_update_the_token_of_the_route_at_the_same_address() {
        let same = same(&studio_and_laptop(), &route(TAILNET, OTHER_TOKEN), "new", None, &[]);
        assert_eq!(same, Some(found(1, Why::Address)));
    }

    #[test]
    fn says_the_url_is_already_saved_when_the_token_is_the_same_too() {
        let same = same(&studio_and_laptop(), &route(TAILNET, NEW_TOKEN), "new", None, &[]);
        assert_eq!(same, Some(found(1, Why::AlreadySaved)));
    }

    #[test]
    fn offers_to_update_a_saved_route_whose_token_is_missing() {
        let machines = vec![machine(0, "Studio", vec![bridge(LAN, 8765, None)])];
        assert_eq!(same(&machines, &route(LAN, TOKEN), "new", None, &[]), Some(found(0, Why::Address)));
    }

    #[test]
    fn needs_the_same_port() {
        let same = same(&studio_and_laptop(), &bridge(LAN, 9000, Some(OTHER_TOKEN)), "new", None, &[]);
        assert_eq!(same, None);
    }
}

mod by_token {
    use super::*;

    #[test]
    fn finds_the_machine_whose_route_has_the_same_token_at_another_address() {
        let same = same(&studio_and_laptop(), &route(GONE, NEW_TOKEN), "new", None, &[]);
        assert_eq!(same, Some(found(1, Why::Token)));
    }

    #[test]
    fn skips_saved_routes_without_a_token() {
        let machines = vec![machine(0, "Studio", vec![bridge(LAN, 8765, None)])];
        assert_eq!(same(&machines, &route(GONE, TOKEN), "new", None, &[]), None);
    }
}

mod by_boot {
    use super::*;

    #[test]
    fn finds_the_machine_last_seen_with_the_boot_the_url_answered() {
        let same = same(&studio_and_laptop(), &route(GONE, OTHER_TOKEN), "new", Some(BOOT), &[(0, "older"), (1, BOOT)]);
        assert_eq!(same, Some(found(1, Why::Boot)));
    }

    #[test]
    fn is_skipped_when_the_url_did_not_answer() {
        let same = same(&studio_and_laptop(), &route(GONE, OTHER_TOKEN), "laptop", None, &[(0, BOOT)]);
        assert_eq!(same, Some(found(1, Why::Name)));
    }

    #[test]
    fn finds_nothing_when_no_machine_was_seen_with_that_boot() {
        let same = same(&studio_and_laptop(), &route(GONE, OTHER_TOKEN), "new", Some(BOOT), &[(0, "older")]);
        assert_eq!(same, None);
    }
}

mod by_name {
    use super::*;

    #[rstest]
    #[case::the_same("laptop", 1)]
    #[case::in_other_case("STUDIO", 0)]
    #[case::with_spaces_around("  Laptop \t", 1)]
    fn finds_the_machine_with_the_same_name(#[case] name: &str, #[case] id: u64) {
        let same = same(&studio_and_laptop(), &route(GONE, OTHER_TOKEN), name, None, &[]);
        assert_eq!(same, Some(found(id, Why::Name)));
    }

    #[test]
    fn needs_the_whole_name() {
        let same = same(&studio_and_laptop(), &route(GONE, OTHER_TOKEN), "lap", None, &[]);
        assert_eq!(same, None);
    }
}

mod priority {
    use super::*;

    #[rstest]
    #[case::the_address_before_the_token(route(TAILNET, TOKEN), "new", None, Some(found(1, Why::Address)))]
    #[case::the_token_before_the_boot(route(GONE, NEW_TOKEN), "new", Some(BOOT), Some(found(1, Why::Token)))]
    #[case::the_token_before_the_name(route(GONE, NEW_TOKEN), "Studio", None, Some(found(1, Why::Token)))]
    #[case::the_boot_before_the_name(
        route(GONE, OTHER_TOKEN),
        "Studio",
        Some("laptop-boot"),
        Some(found(1, Why::Boot))
    )]
    fn takes_the_first_criterion_that_matches(
        #[case] route: Route,
        #[case] name: &str,
        #[case] probed: Option<&str>,
        #[case] expected: Option<Same>,
    ) {
        let same = same(&studio_and_laptop(), &route, name, probed, &[(0, BOOT), (1, "laptop-boot")]);
        assert_eq!(same, expected);
    }
}

mod ties {
    use super::*;

    #[test]
    fn take_the_first_machine_with_the_same_token() {
        let machines = vec![
            machine(4, "Studio", vec![route(LAN, TOKEN)]),
            machine(2, "laptop", vec![route(TAILNET, NEW_TOKEN), route(GONE, TOKEN)]),
        ];
        assert_eq!(same(&machines, &route("studio.local", TOKEN), "new", None, &[]), Some(found(4, Why::Token)));
    }

    #[test]
    fn take_the_first_machine_seen_with_the_same_boot() {
        let same = same(&studio_and_laptop(), &route(GONE, OTHER_TOKEN), "new", Some(BOOT), &[(1, BOOT), (0, BOOT)]);
        assert_eq!(same, Some(found(0, Why::Boot)));
    }
}

mod no_match {
    use super::*;

    #[test]
    fn finds_nothing_for_a_new_machine() {
        let same = same(&studio_and_laptop(), &route(GONE, OTHER_TOKEN), "new", Some(BOOT), &[(0, "older")]);
        assert_eq!(same, None);
    }

    #[test]
    fn finds_nothing_without_saved_machines() {
        assert_eq!(same(&[], &route(LAN, TOKEN), "Studio", Some(BOOT), &[(0, BOOT)]), None);
    }
}
