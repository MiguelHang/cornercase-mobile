use std::collections::HashSet;

use rstest::rstest;

use super::*;
use crate::machines::tests::{SECRET, TOKEN, bridge, cli};

mod address {
    use super::*;

    #[rstest]
    #[case::bridge(bridge("mac.local", 8765, Some(TOKEN)))]
    #[case::cli(cli("mac.local", 8765, Some(SECRET)))]
    fn is_the_routes_host_and_port(#[case] route: Route) {
        assert_eq!(route.address(), &Address { host: "mac.local".to_owned(), port: 8765 });
    }

    #[rstest]
    #[case::ipv4("192.168.1.20", "ws://192.168.1.20:8765/")]
    #[case::ipv6("fd7a:115c:a1e0::1", "ws://[fd7a:115c:a1e0::1]:8765/")]
    #[case::name("mac.local", "ws://mac.local:8765/")]
    fn shows_as_a_bare_ws_url(#[case] host: &str, #[case] shown: &str) {
        assert_eq!(bridge(host, 8765, None).address().to_string(), shown);
    }
}

mod parse {
    use super::*;

    #[rstest]
    #[case::ipv4("ws://192.168.1.20:8765/?token=c2VjcmV0LXRva2Vu", "192.168.1.20", 8765)]
    #[case::ipv6("ws://[fd7a:115c:a1e0::1]:8765/?token=c2VjcmV0LXRva2Vu", "fd7a:115c:a1e0::1", 8765)]
    #[case::name("ws://mac.local:8765/?token=c2VjcmV0LXRva2Vu", "mac.local", 8765)]
    #[case::trailing_newline("ws://mac.local:8765/?token=c2VjcmV0LXRva2Vu\n", "mac.local", 8765)]
    #[case::spaces_around(" \tws://mac.local:8765/?token=c2VjcmV0LXRva2Vu  ", "mac.local", 8765)]
    #[case::no_port("ws://mac.local/?token=c2VjcmV0LXRva2Vu", "mac.local", 80)]
    #[case::ipv6_without_port("ws://[fd7a:115c:a1e0::1]/?token=c2VjcmV0LXRva2Vu", "fd7a:115c:a1e0::1", 80)]
    #[case::upper_case_scheme("WS://mac.local:8765/?token=c2VjcmV0LXRva2Vu", "mac.local", 8765)]
    #[case::other_parameters("ws://mac.local:8765/?v=1&token=c2VjcmV0LXRva2Vu", "mac.local", 8765)]
    fn reads_the_address_and_the_token(#[case] url: &str, #[case] host: &str, #[case] port: u16) {
        assert_eq!(Route::parse(url), Ok(bridge(host, port, Some(TOKEN))));
    }

    #[rstest]
    #[case::dash("a-b-c")]
    #[case::underscore("a_b_c")]
    #[case::only_symbols("-_-_")]
    fn accepts_the_base64url_symbols_in_the_token(#[case] token: &str) {
        let route = Route::parse(&format!("ws://mac.local:8765/?token={token}")).expect("read the url");
        assert_eq!(route, bridge("mac.local", 8765, Some(token)));
    }

    #[test]
    fn gives_back_the_url_the_bridge_printed() {
        let url = "ws://[fd7a:115c:a1e0::1]:8765/?token=c2VjcmV0LXRva2Vu";
        let route = Route::parse(url).expect("read the url");
        assert_eq!(route.connection_url(), Ok(url.to_owned()));
    }

    #[rstest]
    #[case::wss("wss://mac.local:8765/?token=c2VjcmV0LXRva2Vu", RouteError::Scheme)]
    #[case::http("http://mac.local:8765/?token=c2VjcmV0LXRva2Vu", RouteError::Scheme)]
    #[case::no_scheme("/?token=c2VjcmV0LXRva2Vu", RouteError::Scheme)]
    #[case::user("ws://user@mac.local:8765/?token=c2VjcmV0LXRva2Vu", RouteError::UserInfo)]
    #[case::user_and_password("ws://user:pass@mac.local:8765/?token=c2VjcmV0LXRva2Vu", RouteError::UserInfo)]
    #[case::user_before_ipv6("ws://user@[fd7a:115c:a1e0::1]:8765/?token=c2VjcmV0LXRva2Vu", RouteError::UserInfo)]
    #[case::empty_host("ws://:8765/?token=c2VjcmV0LXRva2Vu", RouteError::NoHost)]
    #[case::empty_brackets("ws://[]:8765/?token=c2VjcmV0LXRva2Vu", RouteError::NoHost)]
    #[case::port_text("ws://mac.local:port/?token=c2VjcmV0LXRva2Vu", RouteError::Port)]
    #[case::port_sign("ws://mac.local:+8765/?token=c2VjcmV0LXRva2Vu", RouteError::Port)]
    #[case::port_too_high("ws://mac.local:65536/?token=c2VjcmV0LXRva2Vu", RouteError::Port)]
    #[case::port_zero("ws://mac.local:0/?token=c2VjcmV0LXRva2Vu", RouteError::Port)]
    #[case::port_empty("ws://mac.local:/?token=c2VjcmV0LXRva2Vu", RouteError::Port)]
    #[case::other_path("ws://mac.local:8765/other?token=c2VjcmV0LXRva2Vu", RouteError::Path)]
    #[case::empty_path("ws://mac.local:8765?token=c2VjcmV0LXRva2Vu", RouteError::Path)]
    #[case::no_query("ws://mac.local:8765/", RouteError::NoToken)]
    #[case::no_token("ws://mac.local:8765/?v=c2VjcmV0LXRva2Vu", RouteError::NoToken)]
    #[case::empty_token("ws://mac.local:8765/?token=", RouteError::EmptyToken)]
    #[case::bare_token_key("ws://mac.local:8765/?token", RouteError::EmptyToken)]
    #[case::token_twice("ws://mac.local:8765/?token=c2VjcmV0LXRva2Vu&token=c2VjcmV0LXRva2Vu", RouteError::TokenTwice)]
    #[case::padding("ws://mac.local:8765/?token=c2VjcmV0LXRva2Vu=", RouteError::TokenCharacters)]
    #[case::plus("ws://mac.local:8765/?token=c2VjcmV0+LXRva2Vu", RouteError::TokenCharacters)]
    #[case::escaped("ws://mac.local:8765/?token=c2VjcmV0%2FLXRva2Vu", RouteError::TokenCharacters)]
    #[case::not_a_url("not a url c2VjcmV0LXRva2Vu", RouteError::NotAUrl)]
    #[case::unclosed_bracket("ws://[fd7a:115c:a1e0::1:8765/?token=c2VjcmV0LXRva2Vu", RouteError::NotAUrl)]
    #[case::empty("", RouteError::NotAUrl)]
    #[case::only_spaces(" \n", RouteError::NotAUrl)]
    fn refuses_with_its_own_reason_and_without_the_url(#[case] url: &str, #[case] reason: RouteError) {
        let refused = Route::parse(url).expect_err("refuse the url");
        assert_eq!(refused, reason);
        for text in [refused.to_string(), format!("{refused:?}")] {
            assert!(!text.contains("mac.local"), "{text}");
            assert!(!text.contains("fd7a"), "{text}");
            assert!(!text.contains(TOKEN), "{text}");
        }
    }
}

mod route_error {
    use super::*;

    #[test]
    fn has_a_message_of_its_own_for_each_reason() {
        let reasons = [
            RouteError::NotAUrl,
            RouteError::Scheme,
            RouteError::UserInfo,
            RouteError::NoHost,
            RouteError::Port,
            RouteError::Path,
            RouteError::NoToken,
            RouteError::EmptyToken,
            RouteError::TokenTwice,
            RouteError::TokenCharacters,
            RouteError::NoCode,
            RouteError::BridgeCode,
            RouteError::EmptyCode,
            RouteError::CodeTwice,
            RouteError::CodeCharacters,
            RouteError::AlsoEmpty,
            RouteError::AlsoInvalid,
            RouteError::NameEncoding,
            RouteError::NameTwice,
        ];
        let messages: HashSet<String> = reasons.iter().map(ToString::to_string).collect();
        assert_eq!(messages.len(), reasons.len());
    }

    #[test]
    fn says_that_only_ws_works() {
        assert!(RouteError::Scheme.to_string().contains("ws://"));
    }
}

mod connection_url {
    use super::*;

    #[rstest]
    #[case::ipv4("192.168.1.20", "ws://192.168.1.20:8765/?token=c2VjcmV0LXRva2Vu")]
    #[case::ipv6("fd7a:115c:a1e0::1", "ws://[fd7a:115c:a1e0::1]:8765/?token=c2VjcmV0LXRva2Vu")]
    #[case::name("mac.local", "ws://mac.local:8765/?token=c2VjcmV0LXRva2Vu")]
    fn puts_the_token_after_the_address(#[case] host: &str, #[case] url: &str) {
        assert_eq!(bridge(host, 8765, Some(TOKEN)).connection_url(), Ok(url.to_owned()));
    }

    #[test]
    fn needs_the_token() {
        assert_eq!(bridge("mac.local", 8765, None).connection_url(), Err(Error::NoToken));
    }

    #[rstest]
    #[case::ipv4("192.168.1.20", "ws://192.168.1.20:7700/cli")]
    #[case::ipv6("fd7a:115c:a1e0::1", "ws://[fd7a:115c:a1e0::1]:7700/cli")]
    #[case::name("mac.local", "ws://mac.local:7700/cli")]
    fn of_cornercase_is_its_cli_path_without_the_secret(#[case] host: &str, #[case] url: &str) {
        assert_eq!(cli(host, 7700, Some(SECRET)).connection_url(), Ok(url.to_owned()));
    }

    #[test]
    fn of_cornercase_does_not_need_the_secret() {
        assert_eq!(cli("mac.local", 7700, None).connection_url(), Ok("ws://mac.local:7700/cli".to_owned()));
    }
}

mod bearer {
    use super::*;

    #[rstest]
    #[case::of_the_bridge(bridge("mac.local", 8765, Some(TOKEN)), Ok(None))]
    #[case::of_the_bridge_without_a_token(bridge("mac.local", 8765, None), Ok(None))]
    #[case::of_cornercase(cli("mac.local", 7700, Some(SECRET)), Ok(Some(SECRET)))]
    #[case::of_cornercase_without_a_secret(cli("mac.local", 7700, None), Err(Error::NoToken))]
    fn is_the_secret_of_cornercase_alone(#[case] route: Route, #[case] expected: Result<Option<&str>, Error>) {
        assert_eq!(route.bearer(), expected);
    }
}

mod has_token {
    use super::*;

    #[rstest]
    #[case::with_one(Some(TOKEN), true)]
    #[case::without_one(None, false)]
    fn says_whether_the_route_carries_its_token(#[case] token: Option<&str>, #[case] has: bool) {
        assert_eq!(bridge("mac.local", 8765, token).has_token(), has);
    }

    #[rstest]
    #[case::with_one(Some(SECRET), true)]
    #[case::without_one(None, false)]
    fn says_whether_a_cornercase_route_carries_its_secret(#[case] secret: Option<&str>, #[case] has: bool) {
        assert_eq!(cli("mac.local", 7700, secret).has_token(), has);
    }
}

mod set_token {
    use super::*;

    #[rstest]
    #[case::bridge(bridge("mac.local", 8765, None), TOKEN)]
    #[case::cli(cli("mac.local", 7700, None), SECRET)]
    fn keeps_the_secret_whole(#[case] mut route: Route, #[case] secret: &str) {
        route.set_token(Some(Token::new(secret.to_owned())));
        assert_eq!(route.token().map(Token::as_str), Some(secret));
    }

    #[test]
    fn keeps_the_kind_of_route() {
        let mut route = cli("mac.local", 7700, None);
        route.set_token(Some(Token::new(SECRET.to_owned())));
        assert_eq!(route, cli("mac.local", 7700, Some(SECRET)));
    }
}

mod debug {
    use super::*;

    #[rstest]
    #[case::bridge(bridge("mac.local", 8765, Some(TOKEN)), TOKEN)]
    #[case::cli(cli("mac.local", 7700, Some(SECRET)), SECRET)]
    fn leaves_the_token_out(#[case] route: Route, #[case] secret: &str) {
        let text = format!("{route:?}");
        assert!(text.contains("mac.local"), "{text}");
        assert!(!text.contains(secret), "{text}");
    }

    #[test]
    fn names_the_token_type_without_its_value() {
        let text = format!("{:?}", Token::new(TOKEN.to_owned()));
        assert!(text.starts_with("Token"), "{text}");
        assert!(!text.contains(TOKEN), "{text}");
    }
}

mod json {
    use super::*;

    #[rstest]
    #[case::bridge(bridge("mac.local", 8765, Some(TOKEN)), r#"{"kind":"bridge","url":"ws://mac.local:8765/"}"#)]
    #[case::cli(cli("mac.local", 7700, Some(SECRET)), r#"{"kind":"cli","url":"ws://mac.local:7700/"}"#)]
    fn is_the_kind_and_the_address_without_the_token(#[case] route: Route, #[case] json: &str) {
        assert_eq!(serde_json::to_string(&route).expect("serialize the route"), json);
    }

    #[rstest]
    #[case::ipv4("192.168.1.20")]
    #[case::ipv6("fd7a:115c:a1e0::1")]
    #[case::name("mac.local")]
    fn reads_back_the_address_without_a_token(#[case] host: &str) {
        let text = serde_json::to_string(&bridge(host, 8765, Some(TOKEN))).expect("serialize the route");
        let read: Route = serde_json::from_str(&text).expect("read the route");
        assert_eq!(read, bridge(host, 8765, None));
    }

    #[rstest]
    #[case::ipv4("192.168.1.20")]
    #[case::ipv6("fd7a:115c:a1e0::1")]
    #[case::name("mac.local")]
    fn reads_back_a_cornercase_route_without_its_secret(#[case] host: &str) {
        let text = serde_json::to_string(&cli(host, 7700, Some(SECRET))).expect("serialize the route");
        let read: Route = serde_json::from_str(&text).expect("read the route");
        assert_eq!(read, cli(host, 7700, None));
    }

    #[rstest]
    #[case::token(r#""ws://mac.local:8765/?token=c2VjcmV0LXRva2Vu""#)]
    #[case::wss(r#""wss://mac.local:8765/""#)]
    #[case::path(r#""ws://mac.local:8765/other""#)]
    #[case::no_port(r#""ws://mac.local/""#)]
    #[case::port_zero(r#""ws://mac.local:0/""#)]
    #[case::empty_host(r#""ws://:8765/""#)]
    #[case::not_a_url(r#""not a url""#)]
    #[case::not_text("8765")]
    fn refuses_what_is_not_a_bare_address(#[case] url: &str, #[values("bridge", "cli")] kind: &str) {
        let text = format!(r#"{{"kind":"{kind}","url":{url}}}"#);
        let error = serde_json::from_str::<Route>(&text).expect_err("refuse the route").to_string();
        assert!(!error.contains("mac.local"), "{error}");
        assert!(!error.contains(TOKEN), "{error}");
    }

    #[test]
    fn refuses_an_unknown_kind() {
        let text = r#"{"kind":"ssh","url":"ws://mac.local:7700/"}"#;
        assert!(serde_json::from_str::<Route>(text).is_err());
    }
}
