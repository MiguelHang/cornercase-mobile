use super::*;

mod config {
    use super::*;

    #[test]
    fn starts_with_the_production_limits() {
        let config = Config::new("t".to_owned(), PathBuf::from("cornercase"));
        assert_eq!(
            (config.ping, config.silence, config.hello, config.handshake, config.connections),
            (Duration::from_secs(15), Duration::from_secs(45), Duration::from_secs(10), Duration::from_secs(10), 64)
        );
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
