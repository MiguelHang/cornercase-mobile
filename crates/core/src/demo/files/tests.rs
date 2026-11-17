use rstest::rstest;

use crate::cli::changes::Status;
use crate::cli::files::{self, EntryKind, Found, Listing, Read, ReadKind};
use crate::demo::DemoTransport;
use crate::error::Error;
use crate::transport::partial_json;

const SHOP: u64 = 2;

async fn listed(workspace: u64, paths: &[&str]) -> Listing {
    partial_json(&DemoTransport::new(), files::list_args(workspace, paths)).await.expect("the demo lists the folders")
}

async fn read(workspace: u64, path: &str) -> Result<Read, Error> {
    partial_json(&DemoTransport::new(), files::read_args(workspace, path)).await
}

async fn searched(query: &str) -> Found {
    partial_json(&DemoTransport::new(), files::search_args(SHOP, query)).await.expect("the demo searches the names")
}

mod the_listing {
    use super::*;

    #[tokio::test]
    async fn shows_the_folders_first_then_the_files_of_the_root() {
        let listing = listed(SHOP, &[""]).await;
        let [root] = listing.folders.as_slice() else { panic!("the listing has {:?}", listing.folders) };
        let names: Vec<(&str, EntryKind)> =
            root.entries.iter().map(|entry| (entry.name.as_str(), entry.kind)).collect();
        let folders = [("emails", EntryKind::Folder), ("public", EntryKind::Folder), ("src", EntryKind::Folder)];
        assert_eq!(names[..3], folders);
        assert!(names[3..].iter().all(|(_, kind)| *kind == EntryKind::File), "{names:?}");
        assert_eq!((root.dir.as_str(), root.error.as_deref()), ("/Users/demo/src/acme-store", None));
    }

    #[tokio::test]
    async fn marks_the_changed_files_and_the_folders_that_hold_them() {
        let listing = listed(SHOP, &["", "src/checkout"]).await;
        let status = |folder: usize, name: &str| {
            listing.folders[folder].entries.iter().find(|entry| entry.name == name).and_then(|entry| entry.status)
        };
        assert_eq!(status(0, "src"), Some(Status::Modified));
        assert_eq!(status(0, "public"), None);
        assert_eq!(status(1, "PaymentForm.tsx"), Some(Status::Modified));
        assert_eq!(status(1, "ApplePayButton.tsx"), Some(Status::Added));
        assert_eq!(status(1, "CardForm.tsx"), None);
    }

    #[tokio::test]
    async fn hides_the_dot_files_and_sizes_the_files() {
        let listing = listed(SHOP, &[""]).await;
        let entry = |name: &str| listing.folders[0].entries.iter().find(|entry| entry.name == name).cloned();
        let ignored = entry(".gitignore").expect("the root has a .gitignore");
        assert!(ignored.hidden);
        assert_eq!(ignored.bytes, Some(24));
        assert!(!entry("README.md").expect("the root has a README").hidden);
        assert_eq!(entry("src").expect("the root has src").bytes, None);
    }

    #[tokio::test]
    async fn says_a_missing_folder_is_missing() {
        let listing = listed(SHOP, &["nowhere"]).await;
        assert_eq!(listing.folders[0].error.as_deref(), Some("no such folder"));
        assert_eq!(listing.folders[0].entries, []);
    }

    #[rstest]
    #[tokio::test]
    async fn has_files_in_every_workspace_with_changes(#[values(2, 11, 15)] workspace: u64) {
        assert_ne!(listed(workspace, &[""]).await.folders[0].entries, []);
    }
}

mod a_read {
    use super::*;

    #[tokio::test]
    async fn opens_a_text_file_with_its_whole_text() {
        let read = read(SHOP, "src/checkout/PaymentForm.tsx").await.expect("the demo reads the file");
        let text = read.text.clone().expect("the file has text");
        assert_eq!(read.kind, ReadKind::Text);
        assert_eq!(read.file, "/Users/demo/src/acme-store/src/checkout/PaymentForm.tsx");
        assert_eq!(read.bytes, u64::try_from(text.len()).ok());
        assert_eq!(text.lines().count(), 18);
        assert!(text.contains("<ApplePayButton amount={total} onPay={pay} />"), "{text}");
    }

    #[tokio::test]
    async fn calls_a_folder_a_folder() {
        let read = read(SHOP, "src").await.expect("the demo reads the folder");
        assert_eq!((read.kind, read.text), (ReadKind::Folder, None));
    }

    #[tokio::test]
    async fn fails_for_a_file_the_workspace_lacks() {
        let error = read(SHOP, "gone.txt").await.expect_err("there is no such file");
        assert!(matches!(error, Error::Failed { code: 2, .. }), "{error:?}");
    }
}

mod a_search {
    use super::*;

    #[tokio::test]
    async fn finds_the_names_and_marks_the_letters_that_match() {
        let found = searched("payment").await;
        let [name] = found.names.as_slice() else { panic!("the search found {:?}", found.names) };
        let marked: String = name
            .path
            .chars()
            .enumerate()
            .filter(|(at, _)| name.indices.contains(&u32::try_from(*at).unwrap_or_default()))
            .map(|(_, letter)| letter)
            .collect();
        assert_eq!((name.path.as_str(), marked.as_str()), ("src/checkout/PaymentForm.tsx", "Payment"));
        assert_eq!(found.matches, 1);
    }

    #[tokio::test]
    async fn finds_every_name_that_holds_the_query() {
        let found = searched("ApplePay").await;
        let paths: Vec<&str> = found.names.iter().map(|name| name.path.as_str()).collect();
        assert_eq!(paths, ["src/checkout/ApplePayButton.tsx", "src/checkout/ApplePayButton.test.tsx"]);
    }

    #[tokio::test]
    async fn finds_nothing_for_a_query_no_name_holds() {
        let found = searched("invoice").await;
        assert_eq!((found.names.len(), found.matches), (0, 0));
    }
}
