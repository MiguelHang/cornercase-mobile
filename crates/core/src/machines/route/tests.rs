use std::collections::HashSet;

use rstest::rstest;

use super::*;
use crate::machines::tests::{SECRET, cli};

mod address {
    use super::*;

    #[test]
    fn is_the_routes_host_and_port() {
        assert_eq!(
            cli("mac.local", 8765, Some(SECRET)).address(),
            &Address { host: "mac.local".to_owned(), port: 8765 }
        );
    }

    #[rstest]
    #[case::ipv4("192.168.1.20", "ws://192.168.1.20:8765/")]
    #[case::ipv6("fd7a:115c:a1e0::1", "ws://[fd7a:115c:a1e0::1]:8765/")]
    #[case::name("mac.local", "ws://mac.local:8765/")]
    fn shows_as_a_bare_ws_url(#[case] host: &str, #[case] shown: &str) {
        assert_eq!(cli(host, 8765, None).address().to_string(), shown);
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
    #[case::ipv4("192.168.1.20", "ws://192.168.1.20:7700/cli")]
    #[case::ipv6("fd7a:115c:a1e0::1", "ws://[fd7a:115c:a1e0::1]:7700/cli")]
    #[case::name("mac.local", "ws://mac.local:7700/cli")]
    fn is_the_cli_path_without_the_secret(#[case] host: &str, #[case] url: &str) {
        assert_eq!(cli(host, 7700, Some(SECRET)).connection_url(), url);
    }

    #[test]
    fn does_not_need_the_secret() {
        assert_eq!(cli("mac.local", 7700, None).connection_url(), "ws://mac.local:7700/cli");
    }
}

mod has_token {
    use super::*;

    #[rstest]
    #[case::with_one(Some(SECRET), true)]
    #[case::without_one(None, false)]
    fn says_whether_the_route_carries_its_secret(#[case] secret: Option<&str>, #[case] has: bool) {
        assert_eq!(cli("mac.local", 7700, secret).has_token(), has);
    }
}

mod cli {
    use super::*;

    #[test]
    fn keeps_the_address_and_the_secret_whole() {
        let address = Address { host: "mac.local".to_owned(), port: 7700 };
        let route = Route::cli(address.clone(), Some(Token::new(SECRET.to_owned())));
        assert_eq!((route.address(), route.token().map(Token::as_str)), (&address, Some(SECRET)));
    }
}

mod debug {
    use super::*;

    #[test]
    fn leaves_the_secret_out() {
        let text = format!("{:?}", cli("mac.local", 7700, Some(SECRET)));
        assert!(text.contains("mac.local"), "{text}");
        assert!(!text.contains(SECRET), "{text}");
    }

    #[test]
    fn names_the_token_type_without_its_value() {
        let text = format!("{:?}", Token::new(SECRET.to_owned()));
        assert!(text.starts_with("Token"), "{text}");
        assert!(!text.contains(SECRET), "{text}");
    }
}

mod json {
    use super::*;

    #[test]
    fn is_the_kind_and_the_address_without_the_secret() {
        let json = serde_json::to_string(&cli("mac.local", 7700, Some(SECRET))).expect("serialize the route");
        assert_eq!(json, r#"{"kind":"cli","url":"ws://mac.local:7700/"}"#);
    }

    #[rstest]
    #[case::ipv4("192.168.1.20")]
    #[case::ipv6("fd7a:115c:a1e0::1")]
    #[case::name("mac.local")]
    fn reads_back_the_address_without_its_secret(#[case] host: &str) {
        let text = serde_json::to_string(&cli(host, 7700, Some(SECRET))).expect("serialize the route");
        let read: Route = serde_json::from_str(&text).expect("read the route");
        assert_eq!(read, cli(host, 7700, None));
    }

    #[rstest]
    #[case::query(r#""ws://mac.local:8765/?code=0f1e2d3c""#)]
    #[case::wss(r#""wss://mac.local:8765/""#)]
    #[case::path(r#""ws://mac.local:8765/other""#)]
    #[case::no_port(r#""ws://mac.local/""#)]
    #[case::port_zero(r#""ws://mac.local:0/""#)]
    #[case::empty_host(r#""ws://:8765/""#)]
    #[case::not_a_url(r#""not a url""#)]
    #[case::not_text("8765")]
    fn refuses_what_is_not_a_bare_address(#[case] url: &str) {
        let text = format!(r#"{{"kind":"cli","url":{url}}}"#);
        let error = serde_json::from_str::<Route>(&text).expect_err("refuse the route").to_string();
        assert!(!error.contains("mac.local"), "{error}");
        assert!(!error.contains("0f1e2d3c"), "{error}");
    }

    #[rstest]
    #[case::ssh("ssh")]
    #[case::of_the_old_bridge("bridge")]
    fn refuses_a_kind_other_than_cli(#[case] kind: &str) {
        let text = format!(r#"{{"kind":"{kind}","url":"ws://mac.local:7700/"}}"#);
        assert!(serde_json::from_str::<Route>(&text).is_err());
    }
}
