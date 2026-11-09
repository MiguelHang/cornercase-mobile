use std::sync::Arc;

use futures_util::future::BoxFuture;
use futures_util::stream::BoxStream;
use tokio::sync::mpsc;

use super::FakeTransport;
use crate::error::Error;
use crate::transport::{Exit, OutSink, Stdin, Transport};

impl FakeTransport {
    pub(crate) fn launched(self) -> (Arc<dyn Transport>, mpsc::UnboundedReceiver<String>) {
        let (argv, heard) = mpsc::unbounded_channel();
        (Arc::new(Launched { fake: self, argv }), heard)
    }
}

struct Launched {
    fake: FakeTransport,
    argv: mpsc::UnboundedSender<String>,
}

impl Transport for Launched {
    fn run(
        &self,
        argv: Vec<String>,
        stdin: BoxStream<'static, Stdin>,
        out: OutSink,
    ) -> BoxFuture<'_, Result<Exit, Error>> {
        self.argv.send(argv.join(" ")).expect("the test reads what was launched");
        self.fake.run(argv, stdin, out)
    }
}
