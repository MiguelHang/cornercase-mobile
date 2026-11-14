use rstest::rstest;

use super::*;
use crate::machines::tests::{SECRET, TOKEN, bridge, cli};

const NEW_TOKEN: &str = "bmV3LXRva2Vu";
const OTHER_TOKEN: &str = "b3RoZXItdG9rZW4";
const LAN: &str = "192.168.1.20";
const TAILNET: &str = "fd7a:115c:a1e0::1";
const GONE: &str = "10.0.0.9";
const BOOT: &str = "2026-11-09T08:00:00Z";
const NEW_TAILNET: &str = "100.101.102.103";
const PHONE_PORT: u16 = 47682;
const NEW_SECRET: &str = "3b7d9e1f.a1b2c3d4e5f60718293a4b5c6d7e8f90a1b2c3d4e5f60718293a4b5c6d7e8f90";

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
    same_machine(machines, std::slice::from_ref(route), name, probed, &seen(boots))
}

fn same_code(machines: &[Machine], hosts: &[&str], token: &str) -> Option<Same> {
    let routes: Vec<Route> = hosts.iter().map(|host| route(host, token)).collect();
    same_machine(machines, &routes, "new", None, &seen(&[]))
}

fn paired(hosts: &[&str], secret: &str) -> Vec<Route> {
    hosts.iter().map(|host| cli(host, PHONE_PORT, Some(secret))).collect()
}

fn same_paired(machines: &[Machine], hosts: &[&str], secret: &str) -> Option<Same> {
    same_machine(machines, &paired(hosts, secret), "new", None, &seen(&[]))
}

fn found(id: u64, why: Why) -> Same {
    Same { machine: MachineId(id), why }
}

mod by_address {
    use super::*;

    #[test]
    fn offers_to_update_the_token_of_the_route_at_the_same_address() {
        let same = same(&studio_and_laptop(), &route(LAN, OTHER_TOKEN), "new", None, &[]);
        assert_eq!(same, Some(found(0, Why::Address)));
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

    #[test]
    fn never_matches_a_cornercase_secret_at_another_address() {
        let machines = vec![machine(0, "Studio", paired(&[LAN], SECRET))];
        assert_eq!(same_paired(&machines, &[GONE], SECRET), None);
    }
}

mod cornercase_codes {
    use super::*;

    #[test]
    fn merge_by_the_tailnet_host_of_a_bridge_route_on_another_port() {
        assert_eq!(same_paired(&studio_and_laptop(), &[GONE, TAILNET], SECRET), Some(found(1, Why::Tailscale)));
    }

    #[test]
    fn merge_when_every_host_is_a_bridge_route_of_the_machine() {
        let machines = vec![machine(0, "Studio", vec![route(LAN, TOKEN), route(GONE, TOKEN)])];
        assert_eq!(same_paired(&machines, &[GONE, LAN], SECRET), Some(found(0, Why::AllAddresses)));
    }

    #[test]
    fn offer_a_single_lan_host_of_a_bridge_route() {
        assert_eq!(same_paired(&studio_and_laptop(), &[LAN], SECRET), Some(found(0, Why::Address)));
    }

    #[test]
    fn need_the_same_port_as_a_cornercase_route() {
        let machines = vec![machine(0, "Studio", paired(&[LAN], SECRET))];
        let same = same(&machines, &cli(LAN, 9000, Some(NEW_SECRET)), "new", None, &[]);
        assert_eq!(same, None);
    }

    #[rstest]
    #[case::with_the_same_secret(SECRET, Why::AlreadySaved)]
    #[case::with_a_new_secret(NEW_SECRET, Why::Tailscale)]
    fn paired_again_go_to_the_machine_paired_before(#[case] secret: &str, #[case] why: Why) {
        let machines = vec![machine(0, "Studio", paired(&[LAN, TAILNET], SECRET))];
        assert_eq!(same_paired(&machines, &[LAN, TAILNET], secret), Some(found(0, why)));
    }
}

mod by_tailscale {
    use super::*;

    #[rstest]
    #[case::v6(TAILNET, 1)]
    #[case::v4(NEW_TAILNET, 2)]
    fn merges_by_a_tailnet_address_saved_with_another_token(#[case] host: &str, #[case] id: u64) {
        let mut machines = studio_and_laptop();
        machines.push(machine(2, "mini", vec![route(NEW_TAILNET, NEW_TOKEN)]));
        assert_eq!(same_code(&machines, &[GONE, host], OTHER_TOKEN), Some(found(id, Why::Tailscale)));
    }

    #[test]
    fn comes_before_a_lan_address_of_another_machine() {
        let same = same_code(&studio_and_laptop(), &[LAN, TAILNET], OTHER_TOKEN);
        assert_eq!(same, Some(found(1, Why::Tailscale)));
    }
}

mod by_all_addresses {
    use super::*;

    fn studio() -> Vec<Machine> {
        vec![machine(0, "Studio", vec![route(LAN, TOKEN), route(GONE, TOKEN)])]
    }

    #[test]
    fn merges_when_every_address_of_the_code_is_saved_with_another_token() {
        assert_eq!(same_code(&studio(), &[GONE, LAN], OTHER_TOKEN), Some(found(0, Why::AllAddresses)));
    }

    #[test]
    fn offers_when_one_address_of_the_code_is_new() {
        assert_eq!(same_code(&studio(), &[GONE, LAN, "10.0.0.10"], OTHER_TOKEN), Some(found(0, Why::Address)));
    }

    #[test]
    fn offers_when_the_code_has_a_single_lan_address() {
        assert_eq!(same_code(&studio(), &[LAN], OTHER_TOKEN), Some(found(0, Why::Address)));
    }
}

mod codes {
    use super::*;

    #[test]
    fn are_already_saved_when_the_machine_has_every_route_with_its_token() {
        let machines = vec![machine(0, "Studio", vec![route(LAN, TOKEN), route(GONE, TOKEN)])];
        assert_eq!(same_code(&machines, &[GONE, LAN], TOKEN), Some(found(0, Why::AlreadySaved)));
    }

    #[test]
    fn merge_by_the_token_when_the_machine_misses_a_route() {
        assert_eq!(same_code(&studio_and_laptop(), &[LAN, NEW_TAILNET], TOKEN), Some(found(0, Why::Token)));
    }

    #[test]
    fn whose_routes_are_saved_as_two_machines_go_to_the_first_with_the_token() {
        let machines =
            vec![machine(0, "Studio", vec![route(LAN, TOKEN)]), machine(1, "Tailnet", vec![route(TAILNET, TOKEN)])];
        assert_eq!(same_code(&machines, &[TAILNET, LAN], TOKEN), Some(found(0, Why::Token)));
    }

    #[test]
    fn without_routes_match_no_saved_route() {
        assert_eq!(same_code(&studio_and_laptop(), &[], TOKEN), None);
    }

    #[rstest]
    #[case::already_saved(Why::AlreadySaved, true)]
    #[case::token(Why::Token, true)]
    #[case::tailscale(Why::Tailscale, true)]
    #[case::all_addresses(Why::AllAddresses, true)]
    #[case::address(Why::Address, false)]
    #[case::boot(Why::Boot, false)]
    #[case::name(Why::Name, false)]
    fn merge_without_asking_only_when_the_match_is_sure(#[case] why: Why, #[case] merges: bool) {
        assert_eq!(why.merges(), merges);
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
    #[case::the_token_before_the_address(route(LAN, NEW_TOKEN), "new", None, Some(found(1, Why::Token)))]
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
