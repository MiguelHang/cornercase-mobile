pub(crate) const BUILTINS: &str =
    include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/control/flow-list/0.13.31/builtins.json"));
pub(crate) const PROJECT: &str =
    include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/control/flow-list/0.13.31/project.json"));
pub(crate) const RUN: &str =
    include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/control/flow-run/0.13.31/run.json"));
pub(crate) const NO_ORCHESTRATOR: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/tests/fixtures/control/flow-run/0.13.31/no-orchestrator-edited.json"
));
