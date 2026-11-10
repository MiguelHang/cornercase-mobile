use rstest::rstest;

use super::*;
use crate::machines::tests::{TOKEN, bridge};
use crate::protocol::BridgeRoute;

const LAN: &str = "192.168.1.20";
const TAILSCALE: &str = "100.101.102.103";
const TAILSCALE_V6: &str = "fd7a:115c:a1e0::1";
const OTHER_TOKEN: &str = "b3RoZXItdG9rZW4";

fn answer(routes: &[(&str, u16)]) -> BridgeRoutes {
    let routes = routes.iter().map(|&(host, port)| BridgeRoute { host: host.to_owned(), port });
    BridgeRoutes { routes: routes.collect() }
}

fn winner() -> Route {
    bridge(LAN, 8765, Some(TOKEN))
}

mod learned_routes {
    use super::*;

    #[test]
    fn gives_every_address_the_winner_s_token() {
        let learned = learned_routes(&answer(&[(LAN, 8765), (TAILSCALE, 8765)]), &bridge(LAN, 8765, Some(OTHER_TOKEN)));
        assert_eq!(learned, [bridge(LAN, 8765, Some(OTHER_TOKEN)), bridge(TAILSCALE, 8765, Some(OTHER_TOKEN))]);
    }

    #[test]
    fn gives_nothing_for_an_empty_answer() {
        assert_eq!(learned_routes(&answer(&[]), &winner()), []);
    }

    #[test]
    fn keeps_the_port_the_bridge_answered() {
        assert_eq!(learned_routes(&answer(&[(TAILSCALE, 9000)]), &winner()), [bridge(TAILSCALE, 9000, Some(TOKEN))]);
    }

    #[test]
    fn takes_an_ipv6_address_without_brackets() {
        let learned = learned_routes(&answer(&[(TAILSCALE_V6, 8765)]), &winner());
        assert_eq!(learned, [bridge(TAILSCALE_V6, 8765, Some(TOKEN))]);
        assert_eq!(learned[0].address().to_string(), format!("ws://[{TAILSCALE_V6}]:8765/"));
    }

    #[rstest]
    #[case::empty_host("", 8765)]
    #[case::port_zero(TAILSCALE, 0)]
    #[case::brackets("[fd7a:115c:a1e0::1]", 8765)]
    #[case::a_space("100.101.102 .103", 8765)]
    #[case::a_path("100.101.102.103/x", 8765)]
    #[case::user_info("me@100.101.102.103", 8765)]
    fn leaves_out_what_is_no_address(#[case] host: &str, #[case] port: u16) {
        let learned = learned_routes(&answer(&[(host, port), (TAILSCALE, 8765)]), &winner());
        assert_eq!(learned, [bridge(TAILSCALE, 8765, Some(TOKEN))]);
    }

    #[test]
    fn gives_nothing_when_the_winner_has_no_token() {
        assert_eq!(learned_routes(&answer(&[(TAILSCALE, 8765)]), &bridge(LAN, 8765, None)), []);
    }
}
