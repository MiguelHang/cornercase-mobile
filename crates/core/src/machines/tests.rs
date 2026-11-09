use super::route::{BridgeUrl, Token};
use super::*;

pub(super) const TOKEN: &str = "c2VjcmV0LXRva2Vu";

pub(super) fn bridge(host: &str, port: u16, token: Option<&str>) -> Route {
    let address = Address { host: host.to_owned(), port };
    Route::Bridge { url: BridgeUrl { address, token: token.map(|token| Token::new(token.to_owned())) } }
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
