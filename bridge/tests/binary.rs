use std::process::Command;

use rstest::rstest;

#[rstest]
#[case::option_is_missing(
    &["--cornercase", "/nonexistent"],
    "/usr/bin:/bin",
    Some(2),
    "cornercase-bridge: --cornercase: /nonexistent does not exist or is not executable\n"
)]
#[case::nothing_in_an_empty_path(&[], "", Some(2), "cornercase-bridge: no executable `cornercase` in the PATH ()\n")]
#[case::option_is_executable(&["--cornercase", "/bin/sh"], "", Some(0), "")]
fn starts_only_with_a_cornercase_binary(
    #[case] args: &[&str],
    #[case] path: &str,
    #[case] code: Option<i32>,
    #[case] stderr: &str,
) {
    let output = Command::new(env!("CARGO_BIN_EXE_cornercase-bridge"))
        .args(args)
        .env_remove("CORNERCASE_BIN")
        .env("PATH", path)
        .output()
        .expect("the bridge runs");
    assert_eq!((output.status.code(), String::from_utf8_lossy(&output.stderr).as_ref()), (code, stderr));
}
