use base64::Engine as _;
use base64::engine::general_purpose::STANDARD;
use futures_util::{SinkExt, StreamExt};
use serde::Deserialize;
use tokio::net::{TcpListener, TcpStream};
use tokio::task::JoinHandle;
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::tungstenite::handshake::server::{Callback, ErrorResponse, Request, Response};
use tokio_tungstenite::tungstenite::http::header::AUTHORIZATION;
use tokio_tungstenite::tungstenite::http::{self, StatusCode};
use tokio_tungstenite::{WebSocketStream, accept_hdr_async};

use crate::protocol::{BridgeMessage, ClientMessage};
use crate::transport::Stdin;

pub const CLI: &str = "/cli";

pub type Peer = WebSocketStream<TcpStream>;

#[derive(Debug, Default, PartialEq, Eq)]
pub struct Heard {
    pub argv: Vec<String>,
    pub stdin: Vec<Stdin>,
}

#[derive(Deserialize)]
#[serde(rename_all = "snake_case")]
enum Sent {
    Argv(Vec<String>),
    Stdin(String),
    StdinBase64(String),
    Eof(bool),
}

pub async fn listening(path: &str) -> (String, TcpListener) {
    let listener = TcpListener::bind("127.0.0.1:0").await.expect("a listener");
    let url = format!("ws://{}{path}", listener.local_addr().expect("its address"));
    (url, listener)
}

pub fn serve<S, F>(listener: TcpListener, secret: &str, script: S) -> JoinHandle<F::Output>
where
    S: FnOnce(Peer) -> F + Send + 'static,
    F: Future + Send + 'static,
    F::Output: Send + 'static,
{
    let bearer = format!("Bearer {secret}");
    tokio::spawn(async move {
        loop {
            let (stream, _) = listener.accept().await.expect("a connection");
            if let Ok(peer) = accept_hdr_async(stream, Bearer(bearer.clone())).await {
                return script(peer).await;
            }
        }
    })
}

pub async fn heard(peer: &mut Peer) -> Vec<String> {
    let eof = serde_json::to_string(&ClientMessage::Eof).expect("the end of the input");
    let mut heard = Vec::new();
    while let Some(Ok(message)) = peer.next().await {
        let Message::Text(text) = message else { continue };
        heard.push(text.as_str().to_owned());
        if text == eof {
            break;
        }
    }
    heard
}

pub fn request(heard: &[String]) -> Heard {
    let mut request = Heard::default();
    for text in heard {
        match serde_json::from_str(text).expect("a message of the client") {
            Sent::Argv(argv) => request.argv = argv,
            Sent::Stdin(text) => request.stdin.push(Stdin::Text(text)),
            Sent::StdinBase64(encoded) => request.stdin.push(Stdin::Bytes(STANDARD.decode(encoded).expect("base64"))),
            Sent::Eof(ended) => assert!(ended, "`eof` is always true"),
        }
    }
    request
}

pub async fn say(peer: &mut Peer, messages: &[BridgeMessage]) {
    for message in messages {
        peer.send(Message::text(serde_json::to_string(message).expect("a message"))).await.expect("sent");
    }
}

pub async fn closed_cleanly(peer: &mut Peer) -> bool {
    let mut closed = false;
    while let Some(message) = peer.next().await {
        match message {
            Ok(Message::Close(_)) => closed = true,
            Ok(_) => {}
            Err(_) => return false,
        }
    }
    closed
}

struct Bearer(String);

impl Callback for Bearer {
    fn on_request(self, request: &Request, response: Response) -> Result<Response, ErrorResponse> {
        if request.headers().get(AUTHORIZATION).is_some_and(|value| value.as_bytes() == self.0.as_bytes()) {
            Ok(response)
        } else {
            Err(http::Response::builder().status(StatusCode::FORBIDDEN).body(None).expect("a refusal"))
        }
    }
}
