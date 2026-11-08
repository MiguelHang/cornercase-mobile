use std::io::Write;
use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

use rustix::io::Errno;
use rustix::process::{Pid, test_kill_process};
use tokio::sync::oneshot;
use tokio::task::JoinHandle;
use tokio::time::{Instant, sleep, timeout};

use cornercase_core::protocol::BridgeMessage;

use crate::process::Running;
use crate::server::{self, Config, Server};

const PATIENCE: Duration = Duration::from_secs(10);
const POLL: Duration = Duration::from_millis(10);

pub const TOKEN: &str = "s3cret";

pub const HUNG: &str = "echo $$ > \"${0%/*}/pid\"\necho ready\nexec sleep 30";

pub struct TempDir(PathBuf);

impl TempDir {
    pub fn new() -> Self {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let n = NEXT.fetch_add(1, Ordering::Relaxed);
        let dir = std::env::temp_dir().join(format!("cornercase-bridge-test-{}-{n}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("create the temp dir");
        Self(dir.canonicalize().expect("canonicalize the temp dir"))
    }

    pub fn path(&self) -> &Path {
        &self.0
    }
}

impl Default for TempDir {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

pub struct Served {
    pub address: SocketAddr,
    stop: oneshot::Sender<()>,
    running: JoinHandle<()>,
}

impl Served {
    pub async fn start(change: impl FnOnce(&mut Config)) -> Self {
        let mut config = Config::new(TOKEN.to_owned(), PathBuf::from("/nonexistent/cornercase"));
        change(&mut config);
        let address = "127.0.0.1:0".parse().expect("an address");
        Self::serving(server::bind(address, config).await.expect("the server binds"))
    }

    pub fn serving(server: Server) -> Self {
        let address = server.local_addr().expect("the server's address");
        let (stop, stopped) = oneshot::channel::<()>();
        let running = tokio::spawn(server.run(async {
            let _ = stopped.await;
        }));
        Self { address, stop, running }
    }

    pub async fn stop(self) -> String {
        let url = self.url(&format!("/?token={TOKEN}"));
        let _ = self.stop.send(());
        in_time(self.running).await.expect("the server stops");
        url
    }

    pub fn url(&self, target: &str) -> String {
        format!("ws://{}{target}", self.address)
    }
}

pub fn write_executable(path: &Path, contents: &str) {
    let mut child = Command::new("/bin/sh")
        .arg("-c")
        .arg("cat > \"$1\" && chmod 755 \"$1\"")
        .arg("sh")
        .arg(path)
        .stdin(Stdio::piped())
        .spawn()
        .expect("run sh");
    child.stdin.take().expect("stdin").write_all(contents.as_bytes()).expect("write the script");
    assert!(child.wait().expect("wait for sh").success(), "failed to write {}", path.display());
}

pub fn fake_cornercase(dir: &Path, script: &str) -> PathBuf {
    let path = dir.join("cornercase");
    write_executable(&path, &format!("#!/bin/sh\n{script}\n"));
    path
}

pub fn reaped(pid: i32) -> bool {
    Pid::from_raw(pid).is_some_and(|pid| matches!(test_kill_process(pid), Err(Errno::SRCH)))
}

pub fn number_in(dir: &Path, name: &str) -> Option<i32> {
    std::fs::read_to_string(dir.join(name)).ok()?.trim().parse().ok()
}

pub async fn collect(running: &mut Running) -> Vec<BridgeMessage> {
    let mut messages = Vec::new();
    while let Some(message) = timeout(PATIENCE, running.next()).await.expect("the process ends in time") {
        messages.push(message);
    }
    messages
}

pub async fn in_time<F: Future>(future: F) -> F::Output {
    timeout(PATIENCE, future).await.expect("it ends in time")
}

pub async fn wait_until(mut condition: impl AsyncFnMut() -> bool) {
    let deadline = Instant::now() + PATIENCE;
    while !condition().await {
        assert!(Instant::now() < deadline, "the condition holds in time");
        sleep(POLL).await;
    }
}
