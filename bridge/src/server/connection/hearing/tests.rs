use rstest::rstest;
use tokio::time::timeout;

use super::*;

const SILENCE: Duration = Duration::from_millis(30);
const LONG: Duration = Duration::from_secs(60);

fn listening(silence: Duration) -> (Ears, Hearing, watch::Sender<bool>) {
    let (stop, stopping) = watch::channel(false);
    let (ears, hearing) = listen(silence, stopping);
    (ears, hearing, stop)
}

mod ending {
    use super::*;

    #[tokio::test]
    async fn is_what_the_reader_reports() {
        let (ears, mut hearing, _stop) = listening(SILENCE);
        ears.end(Ending::Closed);
        assert!(matches!(hearing.ending().await, Ending::Closed));
    }

    #[tokio::test]
    async fn is_silence_once_nothing_is_heard_for_the_deadline() {
        let start = Instant::now();
        let (_ears, mut hearing, _stop) = listening(SILENCE);
        assert!(matches!(hearing.ending().await, Ending::Silent(SILENCE)));
        assert!(start.elapsed() >= SILENCE);
    }

    #[tokio::test]
    async fn counts_the_silence_from_the_last_frame_heard() {
        let (ears, mut hearing, _stop) = listening(SILENCE);
        let start = Instant::now();
        ears.heard();
        assert!(matches!(hearing.ending().await, Ending::Silent(_)));
        assert!(start.elapsed() >= SILENCE);
    }

    #[tokio::test]
    async fn waits_while_the_reader_writes_to_the_process() {
        let (ears, mut hearing, _stop) = listening(SILENCE);
        ears.busy();
        assert!(timeout(SILENCE * 3, hearing.ending()).await.is_err());
        ears.heard();
        assert!(matches!(hearing.ending().await, Ending::Silent(_)));
    }

    #[tokio::test]
    async fn is_silence_when_the_reader_is_gone_without_a_word() {
        let (ears, mut hearing, _stop) = listening(LONG);
        drop(ears);
        assert!(matches!(hearing.ending().await, Ending::Silent(_)));
    }

    #[tokio::test]
    async fn is_the_stop_once_the_bridge_stops() {
        let (_ears, mut hearing, stop) = listening(LONG);
        stop.send_replace(true);
        assert!(matches!(hearing.ending().await, Ending::Stopped));
    }

    #[tokio::test]
    async fn is_the_stop_while_the_reader_writes_to_the_process() {
        let (ears, mut hearing, stop) = listening(SILENCE);
        ears.busy();
        stop.send_replace(true);
        assert!(matches!(hearing.ending().await, Ending::Stopped));
    }
}

mod display {
    use super::*;

    #[rstest]
    #[case::finished(Ending::Finished, "the command ended")]
    #[case::refused(Ending::Refused(Refusal::new("x")), "the client sent a bad message")]
    #[case::closed(Ending::Closed, "the client closed it")]
    #[case::broken(Ending::Broken(Error::ConnectionClosed), "the connection broke: Connection closed normally")]
    #[case::silent(Ending::Silent(SILENCE), "the client was silent for 30ms")]
    #[case::stopped(Ending::Stopped, "the bridge is stopping")]
    fn says_why_the_connection_ended(#[case] ending: Ending, #[case] said: &str) {
        assert_eq!(ending.to_string(), said);
    }
}
