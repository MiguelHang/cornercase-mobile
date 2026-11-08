use super::*;

mod config {
    use super::*;

    #[test]
    fn starts_with_the_production_limits() {
        let config = Config::new("t".to_owned(), PathBuf::from("cornercase"));
        assert_eq!(
            (config.ping, config.silence, config.hello, config.handshake, config.connections, config.handshakes),
            (
                Duration::from_secs(15),
                Duration::from_secs(45),
                Duration::from_secs(10),
                Duration::from_secs(10),
                64,
                64
            )
        );
    }
}

mod local_addr {
    use super::*;

    fn config() -> Config {
        Config::new("t".to_owned(), PathBuf::from("cornercase"))
    }

    async fn listener() -> TcpListener {
        TcpListener::bind("127.0.0.1:0").await.expect("a listener")
    }

    #[tokio::test]
    async fn of_several_listeners_is_the_first_of_their_addresses_in_order() {
        let server = Server::from_listeners(vec![listener().await, listener().await], config());
        let addresses = server.local_addrs().expect("the addresses");
        assert_eq!(addresses.len(), 2);
        assert_ne!(addresses[0], addresses[1]);
        assert_eq!(server.local_addr().expect("an address"), addresses[0]);
    }

    #[test]
    fn is_an_error_without_listeners() {
        let server = Server::from_listeners(Vec::new(), config());
        assert_eq!(server.local_addr().map_err(|error| error.kind()), Err(io::ErrorKind::NotFound));
        assert_eq!(server.local_addrs().expect("no addresses"), Vec::<SocketAddr>::new());
    }
}

mod websocket {
    use super::*;

    #[test]
    fn reads_messages_of_a_mebibyte_at_most() {
        let config = websocket();
        assert_eq!((config.max_message_size, config.max_frame_size), (Some(1 << 20), Some(1 << 20)));
    }

    #[test]
    fn bounds_the_write_buffer_above_the_largest_line_escaped() {
        let config = websocket();
        assert!(config.max_write_buffer_size > 6 * LONGEST_LINE + config.write_buffer_size);
        assert!(config.max_write_buffer_size < usize::MAX);
    }
}
