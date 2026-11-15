use std::fs;
use std::path::{Path, PathBuf};

use cornercase_core::test_util::TempDir;
use specta_typescript::Typescript;

use super::*;

const UPDATE: &str = "UPDATE_BINDINGS";

fn committed() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../src/lib/bindings.ts")
}

fn export(path: &Path) {
    specta::<Wry>()
        .dangerously_cast_bigints_to_number()
        .export(Typescript::default(), path)
        .expect("the bindings export");
}

fn generated() -> String {
    let dir = TempDir::new();
    let path = dir.path().join("bindings.ts");
    export(&path);
    fs::read_to_string(path).expect("the exported bindings read back")
}

mod the_bindings {
    use super::*;

    #[test]
    fn match_the_committed_file() {
        if std::env::var_os(UPDATE).is_some() {
            export(&committed());
        }
        let committed = fs::read_to_string(committed()).unwrap_or_default();
        assert!(
            committed == generated(),
            "src/lib/bindings.ts is out of date: run `{UPDATE}=1 cargo test -p cornercase-mobile bindings`"
        );
    }
}
