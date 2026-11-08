use std::io;

use tokio::io::AsyncWriteExt;
use tokio::process::ChildStdin;
use tokio::sync::mpsc;

use cornercase_core::protocol::Input;

const CHUNKS: usize = 16;

pub struct Stdin {
    chunks: Option<mpsc::Sender<Vec<u8>>>,
}

impl Stdin {
    pub(super) fn new(stdin: Option<ChildStdin>) -> Self {
        Self { chunks: stdin.map(feed) }
    }

    pub async fn write(&mut self, input: Input) -> io::Result<()> {
        let chunk = match input {
            Input::Text(text) => [text.into_bytes(), vec![b'\n']].concat(),
            Input::Bytes(bytes) => bytes,
            Input::Eof => return self.chunks.take().map(drop).ok_or_else(closed),
        };
        let chunks = self.chunks.as_ref().ok_or_else(closed)?;
        chunks.send(chunk).await.map_err(|_| closed())
    }
}

fn feed(stdin: ChildStdin) -> mpsc::Sender<Vec<u8>> {
    let (sender, chunks) = mpsc::channel(CHUNKS);
    tokio::spawn(pour(stdin, chunks));
    sender
}

async fn pour(mut stdin: ChildStdin, mut chunks: mpsc::Receiver<Vec<u8>>) {
    while let Some(chunk) = chunks.recv().await {
        if stdin.write_all(&chunk).await.is_err() {
            return;
        }
    }
}

fn closed() -> io::Error {
    io::Error::new(io::ErrorKind::BrokenPipe, "the process's input is closed")
}
