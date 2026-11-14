use super::route::{RouteUrl, Token};
use super::*;

pub(super) const TOKEN: &str = "c2VjcmV0LXRva2Vu";
pub(super) const SECRET: &str = "9c1e2a7b.5d0f3c8e9a1b2c3d4e5f60718293a4b5c6d7e8f90a1b2c3d4e5f6071";

pub(super) fn bridge(host: &str, port: u16, token: Option<&str>) -> Route {
    Route::Bridge { url: url(host, port, token) }
}

pub(super) fn cli(host: &str, port: u16, secret: Option<&str>) -> Route {
    Route::Cli { url: url(host, port, secret) }
}

pub(super) async fn within<T>(work: impl Future<Output = T>) -> T {
    tokio::time::timeout(std::time::Duration::from_secs(5), work).await.expect("it gets there within five seconds")
}

fn url(host: &str, port: u16, token: Option<&str>) -> RouteUrl {
    let address = Address { host: host.to_owned(), port };
    RouteUrl { address, token: token.map(|token| Token::new(token.to_owned())) }
}

fn machine() -> Machine {
    Machine {
        id: MachineId(7),
        name: "studio".to_owned(),
        routes: vec![bridge("192.168.1.20", 8765, Some(TOKEN)), bridge("fd7a:115c:a1e0::1", 8765, Some(TOKEN))],
        last_route: Some(Address { host: "fd7a:115c:a1e0::1".to_owned(), port: 8765 }),
    }
}

mod debug {
    use super::*;

    #[test]
    fn leaves_the_tokens_out() {
        let text = format!("{:?}", machine());
        assert!(text.contains("studio"), "{text}");
        assert!(!text.contains(TOKEN), "{text}");
    }
}

mod json {
    use super::*;

    #[test]
    fn holds_the_addresses_and_the_last_route_without_the_tokens() {
        let text = serde_json::to_string(&machine()).expect("serialize the machine");
        assert!(!text.contains(TOKEN), "{text}");
        assert_eq!(
            text,
            concat!(
                r#"{"id":7,"name":"studio","routes":[{"kind":"bridge","url":"ws://192.168.1.20:8765/"},"#,
                r#"{"kind":"bridge","url":"ws://[fd7a:115c:a1e0::1]:8765/"}],"#,
                r#""last_route":"ws://[fd7a:115c:a1e0::1]:8765/"}"#
            )
        );
    }

    #[test]
    fn reads_back_the_machine_without_its_tokens() {
        let text = serde_json::to_string(&machine()).expect("serialize the machine");
        let read: Machine = serde_json::from_str(&text).expect("read the machine");
        let expected = Machine {
            routes: vec![bridge("192.168.1.20", 8765, None), bridge("fd7a:115c:a1e0::1", 8765, None)],
            ..machine()
        };
        assert_eq!(read, expected);
    }
}

mod tailscale {
    use rstest::rstest;

    use super::*;

    #[rstest]
    #[case::the_first_v4("100.64.0.1", true)]
    #[case::the_last_v4("100.127.255.254", true)]
    #[case::below_the_v4_range("100.63.255.255", false)]
    #[case::above_the_v4_range("100.128.0.1", false)]
    #[case::a_lan_v4("192.168.1.20", false)]
    #[case::the_tailnet_v6("fd7a:115c:a1e0::1", true)]
    #[case::another_v6("fd7a:115c:a1e1::1", false)]
    #[case::a_name("studio.local", false)]
    fn knows_the_tailnet_ranges(#[case] host: &str, #[case] expected: bool) {
        assert_eq!(crate::machines::tailscale(&Address { host: host.to_owned(), port: 8765 }), expected);
    }
}

mod reach {
    use rstest::rstest;

    use super::*;

    #[rstest]
    #[case::the_first_of_ten("10.0.0.1", Reach::Lan)]
    #[case::the_last_of_ten("10.255.255.255", Reach::Lan)]
    #[case::the_first_of_172("172.16.0.1", Reach::Lan)]
    #[case::the_last_of_172("172.31.255.255", Reach::Lan)]
    #[case::below_172("172.15.255.255", Reach::Other)]
    #[case::above_172("172.32.0.1", Reach::Other)]
    #[case::the_first_of_192("192.168.0.1", Reach::Lan)]
    #[case::the_last_of_192("192.168.255.255", Reach::Lan)]
    #[case::below_192("192.167.255.255", Reach::Other)]
    #[case::the_first_tailscale_v4("100.64.0.1", Reach::Tailscale)]
    #[case::the_last_tailscale_v4("100.127.255.254", Reach::Tailscale)]
    #[case::above_the_tailscale_v4("100.128.0.1", Reach::Other)]
    #[case::the_tailscale_v6("fd7a:115c:a1e0::1", Reach::Tailscale)]
    #[case::another_v6("fd00::1", Reach::Other)]
    #[case::the_loopback("127.0.0.1", Reach::Other)]
    #[case::a_name("studio.local", Reach::Other)]
    fn sorts_an_address_by_how_it_is_reached(#[case] host: &str, #[case] expected: Reach) {
        assert_eq!(crate::machines::reach(&Address { host: host.to_owned(), port: 8765 }), expected);
    }

    #[test]
    fn puts_the_lan_before_tailscale_and_tailscale_before_the_rest() {
        assert!(Reach::Lan < Reach::Tailscale);
        assert!(Reach::Tailscale < Reach::Other);
    }
}
