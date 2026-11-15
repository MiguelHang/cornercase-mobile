use rstest::rstest;

use super::*;
use crate::machines::tests::{SECRET, TOKEN, cli};

const LAN: &str = "192.168.1.20";
const TAILSCALE_V6: &str = "fd7a:115c:a1e0::1";
const PAIRING: &str = "0f1e2d3c4b5a69788796a5b4c3d2e1f0";

fn secret() -> Token {
    Token::new(SECRET.to_owned())
}

fn read(text: &str) -> MachineCode {
    MachineCode::parse(text).expect("read the code")
}

mod parse {
    use super::*;

    #[test]
    fn reads_every_route_and_the_name_of_cornercase_s_code() {
        let read = read(concat!(
            "ws://192.168.1.20:47682/?code=0f1e2d3c4b5a69788796a5b4c3d2e1f0",
            "&also=192.168.64.1&also=fd7a:115c:a1e0::1&name=MacBook%20Pro%20de%20Miguel"
        ));
        let expected = vec![
            cli(LAN, 47682, Some(SECRET)),
            cli("192.168.64.1", 47682, Some(SECRET)),
            cli(TAILSCALE_V6, 47682, Some(SECRET)),
        ];
        assert_eq!(read.cli_routes(&secret()), expected);
        assert_eq!(read.name(), Some("MacBook Pro de Miguel"));
    }

    #[test]
    fn reads_every_route_of_a_typed_code() {
        let read = read(&format!(" ws://[fd7a:115c:a1e0::1]:47682/?code={PAIRING}&also=192.168.1.20&also=mac.local\n"));
        let expected = vec![
            cli(TAILSCALE_V6, 47682, Some(SECRET)),
            cli(LAN, 47682, Some(SECRET)),
            cli("mac.local", 47682, Some(SECRET)),
        ];
        assert_eq!(read.cli_routes(&secret()), expected);
        assert_eq!(read.name(), None);
    }

    #[rstest]
    #[case::ipv6_first("ws://[fd7a:115c:a1e0::1]:47682/?code=c", TAILSCALE_V6, 47682)]
    #[case::no_name("ws://10.0.0.1:1/?code=c", "10.0.0.1", 1)]
    #[case::spaces_around(" \tws://mac.local:8765/?code=c  ", "mac.local", 8765)]
    #[case::no_port("ws://mac.local/?code=c", "mac.local", 80)]
    #[case::ipv6_without_port("ws://[fd7a:115c:a1e0::1]/?code=c", TAILSCALE_V6, 80)]
    #[case::upper_case_scheme("WS://mac.local:8765/?code=c", "mac.local", 8765)]
    #[case::other_parameters("ws://mac.local:8765/?v=1&code=c", "mac.local", 8765)]
    fn reads_a_code_of_one_route(#[case] text: &str, #[case] host: &str, #[case] port: u16) {
        let read = read(text);
        assert_eq!(read.cli_routes(&secret()), vec![cli(host, port, Some(SECRET))]);
        assert_eq!(read.name(), None);
    }

    #[rstest]
    #[case::the_first("also=192.168.1.20")]
    #[case::twice("also=100.101.102.103&also=100.101.102.103")]
    fn skips_an_address_it_already_has(#[case] also: &str) {
        let read = read(&format!("ws://192.168.1.20:47682/?code={PAIRING}&{also}"));
        assert_eq!(read.cli_routes(&secret()).len(), if also.contains(LAN) { 1 } else { 2 });
    }

    #[rstest]
    #[case::plain("name=Mac%20de%20Miguel", Some("Mac de Miguel"))]
    #[case::lower_case_hex("name=%c3%a9", Some("é"))]
    #[case::literal_plus("name=a+b", Some("a+b"))]
    #[case::empty("name=", None)]
    #[case::bare_key("name", None)]
    #[case::blank("name=%20%20", None)]
    #[case::spaces_around("name=%20Mac%0A", Some("Mac"))]
    fn reads_the_name(#[case] pair: &str, #[case] name: Option<&str>) {
        assert_eq!(read(&format!("ws://mac.local:47682/?code={PAIRING}&{pair}")).name(), name);
    }

    #[test]
    fn reads_the_longest_name_cornercase_writes() {
        let read = read(&format!("ws://10.0.0.1:1/?code=c&name={}", "%C3%A9".repeat(64)));
        assert_eq!(read.name(), Some("é".repeat(64).as_str()));
    }

    #[test]
    fn reads_the_longest_code_cornercase_writes() {
        let text = concat!(
            "ws://192.168.100.200:65535/?code=cccccccccccccccccccccccccccccccc",
            "&also=172.16.100.200&also=fd7a:115c:a1e0:ab12:4843:cd96:6234:5678&name=",
            "xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx"
        );
        let read = read(text);
        assert_eq!(read.cli_routes(&secret()).len(), 3);
        assert_eq!(read.name(), Some("x".repeat(64).as_str()));
    }

    #[test]
    fn leaves_a_token_beside_the_pairing_code_aside() {
        let read = read(&format!("ws://mac.local:47682/?code={PAIRING}&token={TOKEN}"));
        assert_eq!(read.pairing_code().as_str(), PAIRING);
    }

    #[rstest]
    #[case::empty_also("also=", RouteError::AlsoEmpty)]
    #[case::bare_also("also", RouteError::AlsoEmpty)]
    #[case::empty_after_a_good_one("also=10.0.0.2&also=", RouteError::AlsoEmpty)]
    #[case::bracketed_ipv6("also=[fd7a:115c:a1e0::1]", RouteError::AlsoInvalid)]
    #[case::not_ipv6("also=fd7a::zz", RouteError::AlsoInvalid)]
    #[case::port("also=mac.local:8765", RouteError::AlsoInvalid)]
    #[case::user("also=me@mac.local", RouteError::AlsoInvalid)]
    #[case::path("also=mac.local/x", RouteError::AlsoInvalid)]
    #[case::escaped_also("also=mac%2Elocal", RouteError::AlsoInvalid)]
    #[case::underscore("also=mac_local", RouteError::AlsoInvalid)]
    #[case::lone_percent("name=100%", RouteError::NameEncoding)]
    #[case::one_hex_digit("name=%4", RouteError::NameEncoding)]
    #[case::not_hex_in_the_name("name=%zz", RouteError::NameEncoding)]
    #[case::signed_hex("name=%+1", RouteError::NameEncoding)]
    #[case::not_utf8("name=%FF", RouteError::NameEncoding)]
    #[case::cut_utf8("name=%C3", RouteError::NameEncoding)]
    #[case::name_twice("name=a&name=b", RouteError::NameTwice)]
    fn refuses_a_bad_address_or_name_with_its_own_reason_and_without_the_code(
        #[case] pairs: &str,
        #[case] reason: RouteError,
    ) {
        refuses_without_the_code(&format!("code={PAIRING}&{pairs}"), reason);
    }

    #[rstest]
    #[case::empty_code("code=", RouteError::EmptyCode)]
    #[case::bare_code("code", RouteError::EmptyCode)]
    #[case::code_twice(&format!("code={PAIRING}&code={PAIRING}"), RouteError::CodeTwice)]
    #[case::upper_case(&format!("code={}", PAIRING.to_uppercase()), RouteError::CodeCharacters)]
    #[case::not_hex(&format!("code={PAIRING}z"), RouteError::CodeCharacters)]
    #[case::escaped(&format!("code={PAIRING}%41"), RouteError::CodeCharacters)]
    #[case::base64url(&format!("code={PAIRING}-_"), RouteError::CodeCharacters)]
    #[case::no_code("also=10.0.0.2", RouteError::NoCode)]
    #[case::of_the_bridge(&format!("token={TOKEN}"), RouteError::BridgeCode)]
    #[case::of_the_bridge_with_an_empty_token("token=", RouteError::BridgeCode)]
    fn refuses_a_bad_pairing_code_with_its_own_reason_and_without_the_code(
        #[case] pairs: &str,
        #[case] reason: RouteError,
    ) {
        refuses_without_the_code(pairs, reason);
    }

    fn refuses_without_the_code(pairs: &str, reason: RouteError) {
        let refused = MachineCode::parse(&format!("ws://mac.local:47682/?{pairs}&also=10.0.0.2")).expect_err("refuse");
        assert_eq!(refused, reason);
        for text in [refused.to_string(), format!("{refused:?}")] {
            assert!(!text.contains("mac"), "{text}");
            assert!(!text.contains(PAIRING), "{text}");
            assert!(!text.contains(TOKEN), "{text}");
        }
    }

    #[rstest]
    #[case::wss("wss://mac.local:8765/", RouteError::Scheme)]
    #[case::http("http://mac.local:8765/", RouteError::Scheme)]
    #[case::no_scheme("/", RouteError::Scheme)]
    #[case::user("ws://user@mac.local:8765/", RouteError::UserInfo)]
    #[case::user_and_password("ws://user:pass@mac.local:8765/", RouteError::UserInfo)]
    #[case::user_before_ipv6("ws://user@[fd7a:115c:a1e0::1]:8765/", RouteError::UserInfo)]
    #[case::empty_host("ws://:8765/", RouteError::NoHost)]
    #[case::empty_brackets("ws://[]:8765/", RouteError::NoHost)]
    #[case::port_text("ws://mac.local:port/", RouteError::Port)]
    #[case::port_sign("ws://mac.local:+8765/", RouteError::Port)]
    #[case::port_too_high("ws://mac.local:65536/", RouteError::Port)]
    #[case::port_zero("ws://mac.local:0/", RouteError::Port)]
    #[case::port_empty("ws://mac.local:/", RouteError::Port)]
    #[case::other_path("ws://mac.local:8765/other", RouteError::Path)]
    #[case::empty_path("ws://mac.local:8765", RouteError::Path)]
    #[case::unclosed_bracket("ws://[fd7a:115c:a1e0::1:8765/", RouteError::NotAUrl)]
    fn refuses_a_bad_address_with_its_own_reason_and_without_it(#[case] url: &str, #[case] reason: RouteError) {
        let refused = MachineCode::parse(&format!("{url}?code={PAIRING}")).expect_err("refuse");
        assert_eq!(refused, reason);
        for text in [refused.to_string(), format!("{refused:?}")] {
            assert!(!text.contains("mac.local"), "{text}");
            assert!(!text.contains("fd7a"), "{text}");
            assert!(!text.contains(PAIRING), "{text}");
        }
    }

    #[rstest]
    #[case::not_a_url("not a code", RouteError::NotAUrl)]
    #[case::empty("", RouteError::NotAUrl)]
    #[case::only_spaces(" \n", RouteError::NotAUrl)]
    #[case::no_query("ws://mac.local:47682/", RouteError::NoCode)]
    fn refuses_what_is_not_a_code(#[case] text: &str, #[case] reason: RouteError) {
        assert_eq!(MachineCode::parse(text), Err(reason));
    }

    #[test]
    fn tells_a_code_of_the_bridge_to_scan_the_one_of_cornercase() {
        let refused = MachineCode::parse(&format!("ws://192.168.1.20:8765/?token={TOKEN}")).expect_err("refuse");
        assert_eq!(
            refused.to_string(),
            "this is a code of the old bridge: scan the one in cornercase's Settings › Remote › Phone app"
        );
    }
}

mod pairing_code {
    use super::*;

    #[test]
    fn hands_its_pairing_code_over() {
        assert_eq!(read(&format!("ws://mac.local:47682/?code={PAIRING}")).pairing_code().as_str(), PAIRING);
    }

    #[test]
    fn gives_the_address_of_every_host_in_order() {
        let read = read(&format!("ws://192.168.1.20:47682/?code={PAIRING}&also=fd7a:115c:a1e0::1&also=192.168.1.20"));
        let address = |host: &str| Address { host: host.to_owned(), port: 47682 };
        assert_eq!(read.addresses(), vec![address(LAN), address(TAILSCALE_V6)]);
    }
}

mod debug {
    use super::*;

    #[test]
    fn leaves_the_pairing_code_out() {
        let text = format!("{:?}", read(&format!("ws://mac.local:47682/?code={PAIRING}&also=10.0.0.2&name=studio")));
        assert!(text.contains("studio"), "{text}");
        assert!(!text.contains(PAIRING), "{text}");
    }
}
