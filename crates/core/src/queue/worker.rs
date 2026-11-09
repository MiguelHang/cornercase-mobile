use std::collections::VecDeque;
use std::future::Future;
use std::pin::pin;
use std::sync::Arc;

use futures_util::future::{self, Either};
use futures_util::{StreamExt, stream};
use tokio::sync::{mpsc, oneshot};

use super::batch;
use super::requests::{self, Asked, Requests};
use super::typist::{Event, Typed, Typist};
use super::{OnError, Outcome};
use crate::error::Error;
use crate::transport::{self, Stdin, Transport};

pub(crate) type Reply = oneshot::Sender<Result<Outcome, Error>>;

pub(crate) enum Job {
    Command(Command),
    Input(Event),
    Scroll(u64, i64),
}

pub(crate) struct Command {
    pub(crate) args: Vec<String>,
    pub(crate) stdin: Option<Stdin>,
    pub(crate) key: Option<&'static str>,
    pub(crate) reply: Option<Reply>,
}

enum Next {
    Job(Option<Job>),
    RequestsEnded,
    TypistEnded(Typed),
}

pub(crate) struct Worker {
    transport: Arc<dyn Transport>,
    on_error: OnError,
    jobs: mpsc::UnboundedReceiver<Job>,
    pending: VecDeque<Job>,
    requests: Option<Requests>,
    typist: Option<Typist>,
    no_requests: bool,
    no_typist: bool,
}

impl Worker {
    pub(crate) fn new(transport: Arc<dyn Transport>, on_error: OnError, jobs: mpsc::UnboundedReceiver<Job>) -> Self {
        Self {
            transport,
            on_error,
            jobs,
            pending: VecDeque::new(),
            requests: None,
            typist: None,
            no_requests: false,
            no_typist: false,
        }
    }

    pub(crate) async fn work(mut self) {
        while let Some(job) = self.next().await {
            match job {
                Job::Command(command) => self.coalesced(command).await,
                Job::Input(event) => self.input(event).await,
                Job::Scroll(pane, rows) => self.scroll(pane, rows).await,
            }
        }
        self.settle().await;
    }

    async fn next(&mut self) -> Option<Job> {
        if self.pending.is_empty() {
            let job = self.waited().await?;
            self.pending.push_back(job);
        }
        while let Ok(job) = self.jobs.try_recv() {
            self.pending.push_back(job);
        }
        self.pending.pop_front()
    }

    async fn waited(&mut self) -> Option<Job> {
        loop {
            match waited(&mut self.jobs, self.requests.as_mut(), self.typist.as_mut()).await {
                Next::Job(job) => return job,
                Next::RequestsEnded => self.requests = None,
                Next::TypistEnded(typed) => {
                    let events = self.dropped(typed);
                    self.type_once(events).await;
                }
            }
        }
    }

    async fn coalesced(&mut self, command: Command) {
        let command = batch::coalesced(&mut self.pending, command);
        self.command(command).await;
    }

    async fn scroll(&mut self, pane: u64, rows: i64) {
        let rows = batch::scrolled(&mut self.pending, pane, rows);
        if rows != 0 {
            self.command(batch::scroll(pane, rows)).await;
        }
    }

    async fn command(&mut self, command: Command) {
        self.settle().await;
        let result = self.run(&command.args, command.stdin).await;
        self.reply(command.reply, result.map(Outcome::Done));
    }

    async fn run(&mut self, args: &[String], stdin: Option<Stdin>) -> Result<String, Error> {
        let text = match stdin {
            Some(Stdin::Bytes(bytes)) => return once(self.transport.as_ref(), args, Some(Stdin::Bytes(bytes))).await,
            Some(Stdin::Text(text)) => Some(text),
            None => None,
        };
        match self.ask(&requests::line(args, text.as_deref())).await {
            Asked::Answered(result) => result,
            Asked::Unsent | Asked::Absent => once(self.transport.as_ref(), args, text.map(Stdin::Text)).await,
        }
    }

    async fn ask(&mut self, line: &str) -> Asked {
        if self.no_requests {
            return Asked::Absent;
        }
        let requests = self.requests.get_or_insert_with(|| Requests::start(Arc::clone(&self.transport)));
        let asked = requests.ask(line).await;
        if requests.is_over() {
            self.requests = None;
        }
        self.no_requests = asked == Asked::Absent;
        asked
    }

    async fn input(&mut self, (pane, event): Event) {
        let typed = self.typed(pane, &event).await;
        if typed == Typed::Done {
            return;
        }
        let mut events = self.dropped(typed);
        events.push((pane, event));
        self.gather(&mut events);
        self.type_once(events).await;
    }

    async fn typed(&mut self, pane: u64, event: &serde_json::Value) -> Typed {
        if self.no_typist {
            return Typed::Absent(Vec::new());
        }
        let (transport, on_error) = (&self.transport, &self.on_error);
        let typist = self.typist.get_or_insert_with(|| Typist::start(Arc::clone(transport), Arc::clone(on_error)));
        typist.send(pane, event).await
    }

    async fn settle(&mut self) {
        if let Some(typist) = &mut self.typist {
            let typed = typist.settle().await;
            let events = self.dropped(typed);
            self.type_once(events).await;
        }
    }

    fn dropped(&mut self, typed: Typed) -> Vec<Event> {
        if typed == Typed::Done {
            return Vec::new();
        }
        self.typist = None;
        match typed {
            Typed::Unsent(events) => events,
            Typed::Absent(events) => {
                self.no_typist = true;
                events
            }
            Typed::Done | Typed::Gone => Vec::new(),
        }
    }

    fn gather(&mut self, events: &mut Vec<Event>) {
        while let Some(Job::Input(event)) = self.pending.pop_front_if(|job| matches!(job, Job::Input(_))) {
            events.push(event);
        }
    }

    async fn type_once(&mut self, events: Vec<Event>) {
        type_once(self.transport.as_ref(), &self.on_error, events).await;
    }

    fn reply(&self, reply: Option<Reply>, result: Result<Outcome, Error>) {
        let unheard = match reply {
            Some(reply) => reply.send(result).err(),
            None => Some(result),
        };
        if let Some(Err(error)) = unheard {
            (self.on_error)(error);
        }
    }
}

async fn waited(
    jobs: &mut mpsc::UnboundedReceiver<Job>,
    requests: Option<&mut Requests>,
    typist: Option<&mut Typist>,
) -> Next {
    let job = pin!(jobs.recv());
    let requests = pin!(or_never(requests.map(Requests::ended)));
    let typist = pin!(or_never(typist.map(Typist::ended)));
    match future::select(job, future::select(requests, typist)).await {
        Either::Left((job, _)) => Next::Job(job),
        Either::Right((Either::Left(((), _)), _)) => Next::RequestsEnded,
        Either::Right((Either::Right((typed, _)), _)) => Next::TypistEnded(typed),
    }
}

async fn or_never<F: Future>(future: Option<F>) -> F::Output {
    match future {
        Some(future) => future.await,
        None => future::pending().await,
    }
}

async fn type_once(transport: &dyn Transport, on_error: &OnError, events: Vec<Event>) {
    for run in events.chunk_by(|(one, _), (other, _)| one == other) {
        let args = ["input", "--pane", &run[0].0.to_string(), "--json", "-"].map(str::to_owned);
        let lines: String = run.iter().map(|(_, event)| event.to_string() + "\n").collect();
        if let Err(error) = once(transport, &args, Some(Stdin::Text(lines))).await {
            on_error(error);
        }
    }
}

async fn once(transport: &dyn Transport, args: &[String], stdin: Option<Stdin>) -> Result<String, Error> {
    let stdin = stream::iter(stdin).boxed();
    transport::printed(transport::fed(transport, args.to_vec(), stdin).await?)
}
