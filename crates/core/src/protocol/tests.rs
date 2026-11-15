use rstest::rstest;

use super::*;

mod server_message {
    use super::*;

    #[rstest]
    #[case::stdout(ServerMessage::Stdout("a".to_owned()), r#"{"stdout":"a"}"#)]
    #[case::stderr(ServerMessage::Stderr("b".to_owned()), r#"{"stderr":"b"}"#)]
    #[case::exit(ServerMessage::Exit(2), r#"{"exit":2}"#)]
    #[case::negative_exit(ServerMessage::Exit(-1), r#"{"exit":-1}"#)]
    fn is_an_object_with_one_snake_case_key(#[case] message: ServerMessage, #[case] json: &str) {
        assert_eq!(serde_json::to_string(&message).expect("serialize the message"), json);
    }

    #[rstest]
    #[case::stdout(ServerMessage::Stdout("line\n".to_owned()))]
    #[case::stderr(ServerMessage::Stderr("error: ü".to_owned()))]
    #[case::exit(ServerMessage::Exit(2))]
    #[case::negative_exit(ServerMessage::Exit(-1))]
    fn reads_back_what_it_writes(#[case] message: ServerMessage) {
        let json = serde_json::to_string(&message).expect("serialize the message");
        let read: ServerMessage = serde_json::from_str(&json).expect("parse the message");
        assert_eq!(read, message);
    }

    #[test]
    fn exits_with_the_wrong_usage_code_of_cornercase() {
        assert_eq!(USAGE_EXIT, 2);
    }
}

mod app_routes {
    use super::*;

    fn routes(addresses: &[(&str, u16)]) -> AppRoutes {
        let routes = addresses.iter().map(|&(host, port)| AppRoute { host: host.to_owned(), port });
        AppRoutes { routes: routes.collect() }
    }

    #[rstest]
    #[case::none(&[], r#"{"routes":[]}"#)]
    #[case::lan(&[("192.168.1.20", 8765)], r#"{"routes":[{"host":"192.168.1.20","port":8765}]}"#)]
    #[case::lan_and_tailscale(
        &[("192.168.1.20", 8765), ("100.101.102.103", 8765)],
        r#"{"routes":[{"host":"192.168.1.20","port":8765},{"host":"100.101.102.103","port":8765}]}"#
    )]
    #[case::ipv6_without_brackets(&[("fd7a:115c:a1e0::1", 9000)], r#"{"routes":[{"host":"fd7a:115c:a1e0::1","port":9000}]}"#)]
    fn lists_each_address_in_order_as_a_host_and_a_port(#[case] addresses: &[(&str, u16)], #[case] json: &str) {
        let routes = routes(addresses);
        assert_eq!(serde_json::to_string(&routes).expect("serialize the routes"), json);
        assert_eq!(serde_json::from_str::<AppRoutes>(json).expect("parse the routes"), routes);
    }
}

mod client_message {
    use super::*;

    #[rstest]
    #[case::argv(ClientMessage::Argv(vec!["status".to_owned(), "--json".to_owned()]), r#"{"argv":["status","--json"]}"#)]
    #[case::empty_argv(ClientMessage::Argv(Vec::new()), r#"{"argv":[]}"#)]
    #[case::stdin(ClientMessage::Stdin("hello\n".to_owned()), r#"{"stdin":"hello\n"}"#)]
    #[case::stdin_base64(ClientMessage::StdinBase64(vec![0xff, 0xfe, 0x00]), r#"{"stdin_base64":"//4A"}"#)]
    #[case::eof(ClientMessage::Eof, r#"{"eof":true}"#)]
    fn writes_one_key_with_bytes_in_standard_base64_and_eof_as_true(
        #[case] message: ClientMessage,
        #[case] json: &str,
    ) {
        assert_eq!(serde_json::to_string(&message).expect("serialize the message"), json);
    }
}
