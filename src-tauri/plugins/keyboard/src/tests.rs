use tauri::test::{mock_builder, mock_context, noop_assets};

use super::*;

#[test]
fn builds_into_an_app_off_the_phone() {
    let app = mock_builder().plugin(init()).build(mock_context(noop_assets()));
    assert!(app.is_ok());
}
