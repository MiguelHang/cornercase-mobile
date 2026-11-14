use rstest::rstest;

use super::*;
use crate::machines::tests::{SECRET, TOKEN, bridge, cli};

const LAN: &str = "192.168.1.20";
const TAILSCALE: &str = "100.101.102.103";
const TAILSCALE_V6: &str = "fd7a:115c:a1e0::1";
const PAIRING: &str = "0f1e2d3c4b5a69788796a5b4c3d2e1f0";

fn ip(text: &str) -> IpAddr {
    text.parse().expect("an IP address")
}

fn code(first: &str) -> MachineCode {
    MachineCode::new(ip(first), 8765, TOKEN).expect("a machine code")
}

mod new {
    use super::*;

    #[test]
    fn makes_a_code_of_one_route_without_a_name() {
        let made = code(LAN);
        assert_eq!(made.routes(), vec![bridge(LAN, 8765, Some(TOKEN))]);
        assert_eq!(made.name(), None);
    }

    #[rstest]
    #[case::port_zero(0, TOKEN, RouteError::Port)]
    #[case::empty_token(8765, "", RouteError::EmptyToken)]
    #[case::plus(8765, "c2VjcmV0+LXRva2Vu", RouteError::TokenCharacters)]
    #[case::slash(8765, "c2VjcmV0/LXRva2Vu", RouteError::TokenCharacters)]
    #[case::padding(8765, "c2VjcmV0LXRva2Vu=", RouteError::TokenCharacters)]
    fn refuses_what_no_route_could_carry(#[case] port: u16, #[case] token: &str, #[case] reason: RouteError) {
        assert_eq!(MachineCode::new(ip(LAN), port, token), Err(reason));
    }
}

mod with_also {
    use super::*;

    #[test]
    fn adds_routes_in_order_with_the_same_port_and_token() {
        let made = code(LAN).with_also(ip(TAILSCALE)).with_also(ip(TAILSCALE_V6));
        let expected = vec![
            bridge(LAN, 8765, Some(TOKEN)),
            bridge(TAILSCALE, 8765, Some(TOKEN)),
            bridge(TAILSCALE_V6, 8765, Some(TOKEN)),
        ];
        assert_eq!(made.routes(), expected);
    }

    #[rstest]
    #[case::the_first(LAN)]
    #[case::another(TAILSCALE)]
    fn skips_an_address_the_code_already_has(#[case] again: &str) {
        let made = code(LAN).with_also(ip(TAILSCALE)).with_also(ip(again));
        assert_eq!(made.routes().len(), 2);
    }
}

mod with_name {
    use super::*;

    #[test]
    fn keeps_the_name_without_the_spaces_around_it() {
        assert_eq!(code(LAN).with_name("  MacBook Pro \n").name(), Some("MacBook Pro"));
    }

    #[rstest]
    #[case::empty("")]
    #[case::blank(" \t")]
    fn leaves_out_a_name_with_nothing_in_it(#[case] name: &str) {
        assert_eq!(code(LAN).with_name("studio").with_name(name).name(), None);
    }

    #[rstest]
    #[case::ascii(&"m".repeat(70), &"m".repeat(64))]
    #[case::accents(&"é".repeat(70), &"é".repeat(64))]
    #[case::exactly_the_longest(&"m".repeat(64), &"m".repeat(64))]
    #[case::a_space_where_it_is_cut(&format!("{} tail", "m".repeat(63)), &"m".repeat(63))]
    fn keeps_at_most_the_longest_name_a_machine_can_have(#[case] name: &str, #[case] kept: &str) {
        assert_eq!(code(LAN).with_name(name).name(), Some(kept));
    }
}

mod display {
    use super::*;

    #[rstest]
    #[case::one_address(code(LAN), "ws://192.168.1.20:8765/?token=c2VjcmV0LXRva2Vu")]
    #[case::ipv6_first(code(TAILSCALE_V6), "ws://[fd7a:115c:a1e0::1]:8765/?token=c2VjcmV0LXRva2Vu")]
    #[case::two_addresses(
        code(LAN).with_also(ip(TAILSCALE)),
        "ws://192.168.1.20:8765/?token=c2VjcmV0LXRva2Vu&also=100.101.102.103"
    )]
    #[case::ipv6_also_without_brackets(
        code(LAN).with_also(ip(TAILSCALE)).with_also(ip(TAILSCALE_V6)),
        "ws://192.168.1.20:8765/?token=c2VjcmV0LXRva2Vu&also=100.101.102.103&also=fd7a:115c:a1e0::1"
    )]
    #[case::name(
        code(LAN).with_name("Mac de Miguel"),
        "ws://192.168.1.20:8765/?token=c2VjcmV0LXRva2Vu&name=Mac%20de%20Miguel"
    )]
    #[case::unreserved_name(code(LAN).with_name("a-b.c_d~e"), "ws://192.168.1.20:8765/?token=c2VjcmV0LXRva2Vu&name=a-b.c_d~e")]
    #[case::symbols_in_the_name(
        code(LAN).with_name("a&b#c%d+e=f/g"),
        "ws://192.168.1.20:8765/?token=c2VjcmV0LXRva2Vu&name=a%26b%23c%25d%2Be%3Df%2Fg"
    )]
    #[case::accent(code(LAN).with_name("é"), "ws://192.168.1.20:8765/?token=c2VjcmV0LXRva2Vu&name=%C3%A9")]
    fn is_the_first_route_then_the_other_addresses_then_the_name(#[case] made: MachineCode, #[case] text: &str) {
        assert_eq!(made.to_string(), text);
    }
}

mod parse {
    use super::*;

    #[rstest]
    #[case::one_address(code(LAN))]
    #[case::two_addresses(code(LAN).with_also(ip(TAILSCALE)))]
    #[case::three_addresses(code(LAN).with_also(ip(TAILSCALE)).with_also(ip(TAILSCALE_V6)))]
    #[case::ipv6_first(code(TAILSCALE_V6).with_also(ip(LAN)))]
    #[case::spaces(code(LAN).with_name("MacBook Pro de Miguel"))]
    #[case::accents(code(LAN).with_name("Ordenador de José Muñoz"))]
    #[case::ampersand(code(LAN).with_name("Tom & Jerry"))]
    #[case::hash(code(LAN).with_name("Mac #2"))]
    #[case::percent(code(LAN).with_name("100% Mac %41"))]
    #[case::plus_and_equals(code(LAN).with_name("a+b=c"))]
    #[case::emoji(code(LAN).with_name("Mac 🚀 de Ana"))]
    #[case::everything(code(LAN).with_also(ip(TAILSCALE_V6)).with_name("Zoë's Mac & co. #1 🍎"))]
    fn reads_back_what_it_wrote(#[case] made: MachineCode) {
        assert_eq!(MachineCode::parse(&made.to_string()), Ok(made));
    }

    #[test]
    fn reads_every_route_of_a_typed_code() {
        let text = " ws://[fd7a:115c:a1e0::1]:8765/?token=c2VjcmV0LXRva2Vu&also=192.168.1.20&also=mac.local\n";
        let read = MachineCode::parse(text).expect("read the code");
        let expected = vec![
            bridge(TAILSCALE_V6, 8765, Some(TOKEN)),
            bridge(LAN, 8765, Some(TOKEN)),
            bridge("mac.local", 8765, Some(TOKEN)),
        ];
        assert_eq!(read.routes(), expected);
        assert_eq!(read.name(), None);
    }

    #[test]
    fn reads_a_route_url_as_a_code_of_one_route_without_a_name() {
        let read = MachineCode::parse("ws://mac.local:8765/?token=c2VjcmV0LXRva2Vu").expect("read the url");
        assert_eq!(read.routes(), vec![bridge("mac.local", 8765, Some(TOKEN))]);
        assert_eq!(read.name(), None);
    }

    #[rstest]
    #[case::the_first("also=192.168.1.20")]
    #[case::twice("also=100.101.102.103&also=100.101.102.103")]
    fn skips_an_address_it_already_has(#[case] also: &str) {
        let read = MachineCode::parse(&format!("ws://192.168.1.20:8765/?token={TOKEN}&{also}")).expect("read");
        assert_eq!(read.routes().len(), if also.contains(LAN) { 1 } else { 2 });
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
        let read = MachineCode::parse(&format!("ws://mac.local:8765/?token={TOKEN}&{pair}")).expect("read");
        assert_eq!(read.name(), name);
    }

    #[rstest]
    #[case::dash("a-b-c")]
    #[case::underscore("a_b_c")]
    fn accepts_the_base64url_symbols_in_the_token(#[case] token: &str) {
        let read = MachineCode::parse(&format!("ws://mac.local:8765/?token={token}&also=10.0.0.2")).expect("read");
        assert_eq!(read.routes(), vec![bridge("mac.local", 8765, Some(token)), bridge("10.0.0.2", 8765, Some(token))]);
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
    #[case::escaped("also=mac%2Elocal", RouteError::AlsoInvalid)]
    #[case::underscore("also=mac_local", RouteError::AlsoInvalid)]
    #[case::lone_percent("name=100%", RouteError::NameEncoding)]
    #[case::one_hex_digit("name=%4", RouteError::NameEncoding)]
    #[case::not_hex("name=%zz", RouteError::NameEncoding)]
    #[case::signed_hex("name=%+1", RouteError::NameEncoding)]
    #[case::not_utf8("name=%FF", RouteError::NameEncoding)]
    #[case::cut_utf8("name=%C3", RouteError::NameEncoding)]
    #[case::name_twice("name=a&name=b", RouteError::NameTwice)]
    #[case::token_twice("also=10.0.0.2&token=a", RouteError::TokenTwice)]
    fn refuses_with_its_own_reason_and_without_the_code(#[case] pairs: &str, #[case] reason: RouteError) {
        let refused = MachineCode::parse(&format!("ws://mac.local:8765/?token={TOKEN}&{pairs}")).expect_err("refuse");
        assert_eq!(refused, reason);
        for text in [refused.to_string(), format!("{refused:?}")] {
            assert!(!text.contains("mac"), "{text}");
            assert!(!text.contains(TOKEN), "{text}");
        }
    }

    #[rstest]
    #[case::plus("ws://mac.local:8765/?token=c2VjcmV0+LXRva2Vu&also=10.0.0.2", RouteError::TokenCharacters)]
    #[case::slash("ws://mac.local:8765/?token=c2VjcmV0/LXRva2Vu&also=10.0.0.2", RouteError::TokenCharacters)]
    #[case::padding("ws://mac.local:8765/?token=c2VjcmV0LXRva2Vu=&also=10.0.0.2", RouteError::TokenCharacters)]
    #[case::no_token("ws://mac.local:8765/?also=10.0.0.2", RouteError::NoToken)]
    #[case::wss("wss://mac.local:8765/?token=c2VjcmV0LXRva2Vu&also=10.0.0.2", RouteError::Scheme)]
    #[case::path("ws://mac.local:8765/x?token=c2VjcmV0LXRva2Vu&also=10.0.0.2", RouteError::Path)]
    #[case::not_a_url("not a code", RouteError::NotAUrl)]
    fn refuses_what_a_route_url_refuses(#[case] text: &str, #[case] reason: RouteError) {
        assert_eq!(MachineCode::parse(text), Err(reason));
    }
}

mod pairing_code {
    use super::*;

    fn secret() -> Token {
        Token::new(SECRET.to_owned())
    }

    fn read(text: &str) -> MachineCode {
        MachineCode::parse(text).expect("read the code")
    }

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

    #[rstest]
    #[case::ipv6_first("ws://[fd7a:115c:a1e0::1]:47682/?code=c", TAILSCALE_V6, 47682)]
    #[case::no_name("ws://10.0.0.1:1/?code=c", "10.0.0.1", 1)]
    fn reads_a_code_of_one_route(#[case] text: &str, #[case] host: &str, #[case] port: u16) {
        let read = read(text);
        assert_eq!(read.cli_routes(&secret()), vec![cli(host, port, Some(SECRET))]);
        assert_eq!(read.name(), None);
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
    fn gives_no_route_of_the_bridge() {
        assert_eq!(read(&format!("ws://mac.local:47682/?code={PAIRING}&also=10.0.0.2")).routes(), Vec::new());
    }

    #[rstest]
    #[case::empty("code=", RouteError::EmptyCode)]
    #[case::bare_key("code", RouteError::EmptyCode)]
    #[case::twice(&format!("code={PAIRING}&code={PAIRING}"), RouteError::CodeTwice)]
    #[case::token_first(&format!("token={TOKEN}&code={PAIRING}"), RouteError::TokenAndCode)]
    #[case::token_after(&format!("code={PAIRING}&token={TOKEN}"), RouteError::TokenAndCode)]
    #[case::token_and_an_empty_code(&format!("token={TOKEN}&code="), RouteError::TokenAndCode)]
    #[case::upper_case(&format!("code={}", PAIRING.to_uppercase()), RouteError::CodeCharacters)]
    #[case::not_hex(&format!("code={PAIRING}z"), RouteError::CodeCharacters)]
    #[case::escaped(&format!("code={PAIRING}%41"), RouteError::CodeCharacters)]
    #[case::base64url(&format!("code={PAIRING}-_"), RouteError::CodeCharacters)]
    fn refuses_with_its_own_reason_and_without_the_code(#[case] pairs: &str, #[case] reason: RouteError) {
        let refused = MachineCode::parse(&format!("ws://mac.local:47682/?{pairs}&also=10.0.0.2")).expect_err("refuse");
        assert_eq!(refused, reason);
        for text in [refused.to_string(), format!("{refused:?}")] {
            assert!(!text.contains("mac"), "{text}");
            assert!(!text.contains(PAIRING), "{text}");
        }
    }

    #[test]
    fn leaves_the_code_out_of_its_text() {
        let read = read(&format!("ws://mac.local:47682/?code={PAIRING}&also=10.0.0.2&name=studio"));
        for text in [read.to_string(), format!("{read:?}")] {
            assert!(text.contains("studio"), "{text}");
            assert!(!text.contains(PAIRING), "{text}");
        }
        assert_eq!(read.to_string(), "ws://mac.local:47682/?code=redacted&also=10.0.0.2&name=studio");
    }

    #[test]
    fn gives_routes_of_cornercase_from_a_bridge_code_too() {
        assert_eq!(code(LAN).cli_routes(&secret()), vec![cli(LAN, 8765, Some(SECRET))]);
    }
}

mod old_app {
    use super::*;

    #[test]
    fn reads_the_first_route_of_a_code() {
        let made = code(LAN).with_also(ip(TAILSCALE)).with_also(ip(TAILSCALE_V6)).with_name("Mac & co #1 🍎");
        assert_eq!(Route::parse(&made.to_string()), Ok(bridge(LAN, 8765, Some(TOKEN))));
    }
}

mod debug {
    use super::*;

    #[test]
    fn leaves_the_token_out() {
        let text = format!("{:?}", code(LAN).with_name("studio"));
        assert!(text.contains("studio"), "{text}");
        assert!(!text.contains(TOKEN), "{text}");
    }
}
