use std::path::Path;

use cornercase_bridge::process;
use cornercase_bridge::test_util::collect;
use cornercase_core::protocol::BridgeMessage;

const VARIABLES: [&str; 4] = ["CORNERCASE_PANE", "CORNERCASE_SERVER", "CORNERCASE_REMOTE", "CORNERCASE_SOCKET"];
const SCRIPT: &str = r#"for name in "$@"; do eval "value=\${$name-unset}"; printf '%s=%s\n' "$name" "$value"; done"#;

#[tokio::test]
async fn the_child_inherits_the_environment_but_the_pane_variables() {
    for name in VARIABLES {
        unsafe { std::env::set_var(name, "set") };
    }
    let argv: Vec<String> = ["-c", SCRIPT, "sh"].into_iter().chain(VARIABLES).map(String::from).collect();
    let mut running = process::spawn(Path::new("/bin/sh"), &argv).expect("the shell starts");
    assert_eq!(
        collect(&mut running).await,
        [
            BridgeMessage::Stdout("CORNERCASE_PANE=unset".to_owned()),
            BridgeMessage::Stdout("CORNERCASE_SERVER=unset".to_owned()),
            BridgeMessage::Stdout("CORNERCASE_REMOTE=unset".to_owned()),
            BridgeMessage::Stdout("CORNERCASE_SOCKET=set".to_owned()),
            BridgeMessage::Exit(0),
        ]
    );
}
