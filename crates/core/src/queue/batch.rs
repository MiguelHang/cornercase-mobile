use std::collections::VecDeque;
use std::mem;

use super::Outcome;
use super::worker::{Command, Job};

pub(crate) fn coalesced(pending: &mut VecDeque<Job>, mut command: Command) -> Command {
    let Some(key) = command.key else {
        return command;
    };
    while let Some(Job::Command(next)) =
        pending.pop_front_if(|job| matches!(job, Job::Command(next) if next.key == Some(key)))
    {
        skip(mem::replace(&mut command, next));
    }
    command
}

pub(crate) fn scrolled(pending: &mut VecDeque<Job>, pane: u64, mut rows: i64) -> i64 {
    while let Some(Job::Scroll(_, more)) =
        pending.pop_front_if(|job| matches!(job, Job::Scroll(next, _) if *next == pane))
    {
        rows = rows.saturating_add(more);
    }
    rows
}

pub(crate) fn scroll(pane: u64, rows: i64) -> Command {
    let direction = if rows > 0 { "--up" } else { "--down" };
    let args = ["scroll", "--pane", &pane.to_string(), direction, &rows.unsigned_abs().to_string()];
    Command { args: args.map(str::to_owned).into(), stdin: None, key: None, reply: None }
}

pub(crate) fn skip(command: Command) {
    if let Some(reply) = command.reply {
        let _ = reply.send(Ok(Outcome::Skipped));
    }
}
