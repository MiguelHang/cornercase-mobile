use cornercase_core::error::Error;
use cornercase_core::queue::Outcome;
use rstest::rstest;

use super::answered;
use crate::commands::error::CommandError;

mod answered {
    use super::*;

    #[rstest]
    #[case::done(Ok(Outcome::Done("printed".to_owned())), Ok("printed".to_owned()))]
    #[case::skipped(Ok(Outcome::Skipped), Err(CommandError::not_sent()))]
    #[case::unanswered(Err(Error::Unanswered), Err(CommandError::maybe_sent()))]
    #[case::dropped(Err(Error::Dropped), Err(CommandError::from(Error::Dropped)))]
    fn maps_what_the_queue_answered(
        #[case] outcome: Result<Outcome, Error>,
        #[case] mapped: Result<String, CommandError>,
    ) {
        assert_eq!(answered(outcome), mapped);
    }
}
