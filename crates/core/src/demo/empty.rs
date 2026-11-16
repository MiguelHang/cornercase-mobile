use super::dispatch::{Answer, json, number};
use crate::cli::flows::Flows;
use crate::cli::issues::{Lists, Sources};

pub(super) fn answer(argv: &[String]) -> Answer {
    let word = |at: usize| argv.get(at).map(String::as_str);
    match (word(0), word(1), number(argv, "--project")) {
        (Some("issue"), Some("sources"), Some(project)) => {
            Answer::printed(json(&Sources { project, ..Sources::default() }))
        }
        (Some("issue"), Some("list"), Some(project)) => Answer::printed(json(&Lists { project, ..Lists::default() })),
        (Some("flow"), Some("list"), Some(_)) => Answer::printed(json(&Flows::default())),
        _ => Answer::unknown(argv),
    }
}

#[cfg(test)]
mod tests;
