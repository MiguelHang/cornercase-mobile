use rstest::rstest;

use super::*;
use crate::machines::tests::{TOKEN, bridge};

mod address {
    use super::*;

    #[test]
    fn is_the_routes_host_and_port() {
        let route = bridge("mac.local", 8765, Some(TOKEN));
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
}

mod debug {
    use super::*;

    #[test]
    fn leaves_the_token_out() {
        let text = format!("{:?}", bridge("mac.local", 8765, Some(TOKEN)));
        assert!(text.contains("mac.local"), "{text}");
        assert!(!text.contains(TOKEN), "{text}");
    }
}

mod json {
    use super::*;

    #[test]
    fn is_the_kind_and_the_address_without_the_token() {
        let text = serde_json::to_string(&bridge("mac.local", 8765, Some(TOKEN))).expect("serialize the route");
        assert_eq!(text, r#"{"kind":"bridge","url":"ws://mac.local:8765/"}"#);
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
    #[case::token(r#""ws://mac.local:8765/?token=c2VjcmV0LXRva2Vu""#)]
    #[case::wss(r#""wss://mac.local:8765/""#)]
    #[case::path(r#""ws://mac.local:8765/other""#)]
    #[case::no_port(r#""ws://mac.local/""#)]
    #[case::port_zero(r#""ws://mac.local:0/""#)]
    #[case::empty_host(r#""ws://:8765/""#)]
    #[case::not_a_url(r#""not a url""#)]
    #[case::not_text("8765")]
    fn refuses_what_is_not_a_bare_address(#[case] url: &str) {
        let text = format!(r#"{{"kind":"bridge","url":{url}}}"#);
        let error = serde_json::from_str::<Route>(&text).expect_err("refuse the route").to_string();
        assert!(!error.contains("mac.local"), "{error}");
        assert!(!error.contains(TOKEN), "{error}");
    }
}
