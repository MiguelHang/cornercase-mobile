pub(crate) const FRAME: &str =
    include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/control/surface/v1/frame.json"));
pub(crate) const PATCH: &str =
    include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/control/surface/v1/patch.json"));
pub(crate) const ASK_ONE: &str =
    include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/control/surface/v1/question/ask_one.json"));
pub(crate) const ASK_LONG: &str =
    include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/control/surface/v1/question/ask_long.json"));
pub(crate) const ASK_TABS: &str =
    include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/control/surface/v1/question/ask_tabs.json"));
pub(crate) const PERMISSION: &str =
    include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/control/surface/v1/question/permission.json"));
pub(crate) const PERMISSION_GUTTER: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/tests/fixtures/control/surface/v1/question/permission_gutter.json"
));
pub(crate) const WORKING: &str =
    include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/control/surface/v1/question/working.json"));
