use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

use tokio::time::{Instant, sleep, timeout};

use cornercase_core::protocol::BridgeMessage;

use crate::process::Running;

const PATIENCE: Duration = Duration::from_secs(10);
const POLL: Duration = Duration::from_millis(10);

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
