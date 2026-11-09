use cornercase_core::error::Error;
use tauri::Manager;
use tauri::test::{MockRuntime, mock_builder, mock_context, noop_assets};

use super::*;

mod off_the_phone {
    use super::*;

    #[test]
    fn says_the_picker_is_only_on_ios() {
        let app = mock_builder().plugin(init()).build(mock_context(noop_assets())).expect("a mock app");
        let picked = app.state::<Photos<MockRuntime>>().pick();
        assert_eq!(picked, Err(Error::Picker("only the iOS app has a photo picker".to_owned())));
    }
}
