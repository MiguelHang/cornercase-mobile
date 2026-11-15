use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

use futures_util::SinkExt;
use rstest::rstest;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::mpsc;
use tokio_tungstenite::accept_hdr_async;
use tokio_tungstenite::tungstenite::handshake::server::{Callback, ErrorResponse, Response};
use tokio_tungstenite::tungstenite::http;

use super::*;
use crate::machines::tests::within;

const CODE: &str = "0f1e2d3c4b5a69788796a5b4c3d2e1f0";
const SECRET: &str = "4f2a91c0.9c1e2a7b5d0f";
const AGENT: &str = "cornercase-mobile/0.1.0 (iOS)";
const REPLY: &str = r#"{"secret":"4f2a91c0.9c1e2a7b5d0f"}"#;

type Head = (String, Option<String>);
type Scripts = Arc<Mutex<VecDeque<Mac>>>;

#[derive(Debug, Clone)]
enum Mac {
    Refuses(u16),
    Closes(CloseCode),
    Answers(&'static str),
    AnswersLate(Duration),
    AnswersAndLeaves,
    AnswersThenCloses(CloseCode),
    PingsThenAnswers,
    Holds,
}

struct Handshake(Option<u16>, mpsc::UnboundedSender<Head>);

impl Callback for Handshake {
    fn on_request(self, request: &http::Request<()>, response: Response) -> Result<Response, ErrorResponse> {
        let agent =
            request.headers().get(USER_AGENT).map(|value| String::from_utf8_lossy(value.as_bytes()).into_owned());
        let _ = self.1.send((request.uri().to_string(), agent));
        match self.0 {
            Some(status) => Err(refusal(status)),
            None => Ok(response),
        }
    }
}

fn refusal(status: u16) -> ErrorResponse {
    http::Response::builder().status(status).body(None).expect("a refusal")
}

struct Server {
    port: u16,
    heads: mpsc::UnboundedReceiver<Head>,
}

impl Server {
    async fn start(hosts: &[&str], macs: Vec<Mac>) -> Self {
        let scripts: Scripts = Arc::new(Mutex::new(macs.into()));
        let (sender, heads) = mpsc::unbounded_channel();
        let mut port = 0;
        for host in hosts {
            let listener = TcpListener::bind((*host, port)).await.expect("a listener");
            port = listener.local_addr().expect("its address").port();
            tokio::spawn(serve(listener, Arc::clone(&scripts), sender.clone()));
        }
        Self { port, heads }
    }

    fn code(&self, also: &[&str]) -> MachineCode {
        let also: String = also.iter().flat_map(|host| ["&also=", host]).collect();
        MachineCode::parse(&format!("ws://127.0.0.1:{}/?code={CODE}{also}", self.port)).expect("a pairing code")
    }

    fn address(&self, host: &str) -> Address {
        Address { host: host.to_owned(), port: self.port }
    }
}

async fn serve(listener: TcpListener, scripts: Scripts, heads: mpsc::UnboundedSender<Head>) {
    while let Ok((stream, _)) = listener.accept().await {
        let mac = scripts.lock().expect("the scripts").pop_front().unwrap_or(Mac::Refuses(403));
        tokio::spawn(answer(stream, mac, heads.clone()));
    }
}

async fn answer(stream: TcpStream, mac: Mac, heads: mpsc::UnboundedSender<Head>) {
    if matches!(mac, Mac::Holds) {
        let _held = stream;
        std::future::pending::<()>().await;
        return;
    }
    let refusal = if let Mac::Refuses(status) = mac { Some(status) } else { None };
    let Ok(mut socket) = accept_hdr_async(stream, Handshake(refusal, heads)).await else { return };
    let messages = match mac {
        Mac::Closes(code) => vec![close(code)],
        Mac::Answers(text) => vec![Message::text(text), close(CloseCode::Normal)],
        Mac::AnswersLate(delay) => {
            time::sleep(delay).await;
            vec![Message::text(REPLY), close(CloseCode::Normal)]
        }
        Mac::AnswersAndLeaves => vec![Message::text(REPLY)],
        Mac::AnswersThenCloses(code) => vec![Message::text(REPLY), close(code)],
        Mac::PingsThenAnswers => vec![Message::Ping(Vec::new().into()), Message::text(REPLY), close(CloseCode::Normal)],
        Mac::Refuses(_) | Mac::Holds => Vec::new(),
    };
    for message in messages {
        let _ = socket.send(message).await;
    }
}

fn close(code: CloseCode) -> Message {
    Message::Close(Some(CloseFrame { code, reason: "".into() }))
}

async fn pair_with(code: &MachineCode) -> Result<Token, PairError> {
    pair(code, AGENT, PairConfig::default()).await
}

async fn paired(macs: Vec<Mac>) -> Result<Token, PairError> {
    let server = Server::start(&["127.0.0.1"], macs).await;
    pair_with(&server.code(&[])).await
}

fn secret() -> Token {
    Token::new(SECRET.to_owned())
}

mod config {
    use super::*;

    #[test]
    fn waits_ten_seconds_for_the_mac_by_default() {
        assert_eq!(PairConfig::default().deadline, Duration::from_secs(10));
    }
}

mod pair {
    use super::*;

    #[tokio::test]
    async fn takes_the_secret_the_mac_hands_over() {
        assert_eq!(paired(vec![Mac::Answers(REPLY)]).await, Ok(secret()));
    }

    #[tokio::test]
    async fn sends_the_code_in_the_query_and_the_app_in_the_user_agent() {
        let mut server = Server::start(&["127.0.0.1"], vec![Mac::Answers(REPLY)]).await;
        let _ = pair_with(&server.code(&[])).await;
        let head = within(server.heads.recv()).await.expect("the head of the upgrade");
        assert_eq!(head, (format!("/?code={CODE}"), Some(AGENT.to_owned())));
    }

    #[tokio::test]
    async fn skips_a_ping_before_the_secret() {
        assert_eq!(paired(vec![Mac::PingsThenAnswers]).await, Ok(secret()));
    }

    #[rstest]
    #[case::refused_before_the_upgrade(Mac::Refuses(403))]
    #[case::closed_after_the_upgrade(Mac::Closes(CloseCode::Policy))]
    #[tokio::test]
    async fn pairs_once_with_a_code_of_one_use(#[case] second: Mac) {
        let server = Server::start(&["127.0.0.1", "::1"], vec![Mac::Answers(REPLY), second]).await;
        assert_eq!(pair_with(&server.code(&["::1"])).await, Ok(secret()));
    }

    #[tokio::test]
    async fn waits_for_the_secret_past_another_route_s_refusal() {
        let late = Mac::AnswersLate(Duration::from_millis(100));
        let server = Server::start(&["127.0.0.1", "::1"], vec![Mac::Refuses(403), late]).await;
        assert_eq!(pair_with(&server.code(&["::1"])).await, Ok(secret()));
    }

    #[rstest]
    #[case::forbidden(Mac::Refuses(403), Error::PairingRefused)]
    #[case::lost_race(Mac::Closes(CloseCode::Policy), Error::PairingRefused)]
    #[case::not_saved(Mac::Closes(CloseCode::Error), Error::PairingFailed)]
    #[tokio::test]
    async fn tells_a_spent_code_from_a_pairing_the_mac_could_not_save(#[case] mac: Mac, #[case] error: Error) {
        assert_eq!(paired(vec![mac]).await, Err(PairError::Answered(error)));
    }

    #[rstest]
    #[case::not_json("4f2a91c0.9c1e2a7b5d0f")]
    #[case::no_secret(r#"{"id":"4f2a91c0"}"#)]
    #[case::not_a_string(r#"{"secret":42}"#)]
    #[case::empty(r#"{"secret":""}"#)]
    #[case::no_dot(r#"{"secret":"4f2a91c09c1e2a7b5d0f"}"#)]
    #[case::no_id(r#"{"secret":".9c1e2a7b5d0f"}"#)]
    #[case::no_key(r#"{"secret":"4f2a91c0."}"#)]
    #[case::two_dots(r#"{"secret":"4f2a.91c0.9c1e"}"#)]
    #[case::not_hex(r#"{"secret":"4f2a91c0.9c1e2a7b5d0z"}"#)]
    #[case::spaces(r#"{"secret":"4f2a91c0. 9c1e2a7b5d0f"}"#)]
    #[tokio::test]
    async fn returns_no_secret_out_of_its_form(#[case] reply: &'static str) {
        assert_eq!(paired(vec![Mac::Answers(reply)]).await, Err(PairError::Answered(Error::MalformedSecret)));
    }

    #[tokio::test]
    async fn keeps_no_secret_the_mac_did_not_close_on() {
        let failure = Error::Protocol("the Mac did not close the pairing after the secret".to_owned());
        let server = Server::start(&["127.0.0.1"], vec![Mac::AnswersAndLeaves]).await;
        let expected = PairError::NoAnswer(vec![(server.address("127.0.0.1"), failure)]);
        assert_eq!(pair_with(&server.code(&[])).await, Err(expected));
    }

    #[rstest]
    #[case::policy(CloseCode::Policy)]
    #[case::error(CloseCode::Error)]
    #[case::away(CloseCode::Away)]
    #[tokio::test]
    async fn keeps_no_secret_the_mac_closed_on_with_another_code(#[case] code: CloseCode) {
        let failure = Error::Protocol("the Mac did not close the pairing after the secret".to_owned());
        let server = Server::start(&["127.0.0.1"], vec![Mac::AnswersThenCloses(code)]).await;
        let expected = PairError::NoAnswer(vec![(server.address("127.0.0.1"), failure)]);
        assert_eq!(pair_with(&server.code(&[])).await, Err(expected));
    }

    #[tokio::test]
    async fn keeps_no_secret_in_a_reply_past_its_size_limit() {
        let reply: &'static str =
            Box::leak(format!(r#"{{"secret":"4f2a91c0.{}"}}"#, "a".repeat(5000)).into_boxed_str());
        assert!(matches!(paired(vec![Mac::Answers(reply)]).await, Err(PairError::NoAnswer(_))));
    }

    #[tokio::test]
    async fn gives_up_on_a_mac_that_never_answers() {
        let server = Server::start(&["127.0.0.1"], vec![Mac::Holds]).await;
        let config = PairConfig { deadline: Duration::from_millis(100) };
        let expected = PairError::NoAnswer(vec![(server.address("127.0.0.1"), Error::Timeout)]);
        assert_eq!(pair(&server.code(&[]), AGENT, config).await, Err(expected));
    }

    #[tokio::test]
    async fn tells_the_failure_of_a_route_that_refused_another_way() {
        let server = Server::start(&["127.0.0.1"], vec![Mac::Refuses(503)]).await;
        let expected = PairError::NoAnswer(vec![(server.address("127.0.0.1"), Error::Rejected { status: 503 })]);
        assert_eq!(pair_with(&server.code(&[])).await, Err(expected));
    }

    #[tokio::test]
    async fn refuses_an_app_name_that_cannot_go_in_a_header() {
        let server = Server::start(&["127.0.0.1"], Vec::new()).await;
        let refused = pair(&server.code(&[]), "cornercase-mobile\n", PairConfig::default()).await;
        let failure = Error::Connect("the app's name cannot go in a header".to_owned());
        assert_eq!(refused, Err(PairError::NoAnswer(vec![(server.address("127.0.0.1"), failure)])));
    }

    #[rstest]
    #[case::refused(Mac::Refuses(403))]
    #[case::malformed(Mac::Answers(r#"{"secret":"4f2a91c0.zz"}"#))]
    #[case::unanswered(Mac::Refuses(503))]
    #[tokio::test]
    async fn shows_neither_the_code_nor_the_secret(#[case] mac: Mac) {
        let refused = paired(vec![mac]).await.expect_err("no secret");
        for text in [refused.to_string(), format!("{refused:?}")] {
            assert!(!text.contains(CODE), "{text}");
            assert!(!text.contains("4f2a91c0"), "{text}");
        }
    }

    #[tokio::test]
    async fn shows_nothing_of_the_secret_it_takes() {
        let taken = paired(vec![Mac::Answers(REPLY)]).await;
        assert!(!format!("{taken:?}").contains("4f2a91c0"), "{taken:?}");
    }
}

mod verdict {
    use super::*;

    fn address(host: &str) -> Address {
        Address { host: host.to_owned(), port: 47682 }
    }

    #[rstest]
    #[case::refused_and_lost(vec![Some(Error::Timeout), Some(Error::PairingRefused)], Error::PairingRefused)]
    #[case::refused_and_unsaved(vec![Some(Error::PairingRefused), Some(Error::PairingFailed)], Error::PairingFailed)]
    #[case::refused_and_malformed(vec![Some(Error::PairingRefused), Some(Error::MalformedSecret)], Error::MalformedSecret)]
    #[case::unsaved_and_malformed(vec![Some(Error::MalformedSecret), Some(Error::PairingFailed)], Error::PairingFailed)]
    #[case::pending_and_refused(vec![None, Some(Error::PairingRefused)], Error::PairingRefused)]
    fn takes_what_the_mac_answered_on_any_route(#[case] failures: Vec<Option<Error>>, #[case] answer: Error) {
        assert_eq!(verdict(vec![address("10.0.0.1"), address("100.64.0.1")], failures), PairError::Answered(answer));
    }

    #[test]
    fn counts_a_route_still_waiting_as_timed_out() {
        let failures = vec![Some(Error::Connect("refused".to_owned())), None];
        let expected =
            vec![(address("10.0.0.1"), Error::Connect("refused".to_owned())), (address("100.64.0.1"), Error::Timeout)];
        assert_eq!(verdict(vec![address("10.0.0.1"), address("100.64.0.1")], failures), PairError::NoAnswer(expected));
    }
}
