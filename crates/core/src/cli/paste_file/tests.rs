use rstest::rstest;
use serde_json::json;

use super::*;

const PASTED: &str =
    include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/control/paste-file/0.13.19/pasted.json"));

fn words(words: &[&str]) -> Vec<String> {
    words.iter().map(|word| (*word).to_owned()).collect()
}

mod reading {
    use super::*;

    #[test]
    fn reads_what_cornercase_0_13_19_prints() {
        let expected = Pasted {
            pane: 543,
            path: "/Users/miguelhang/.local/state/cornercase/pasted/2026-11-09T19-40-12Z/IMG_0042.jpg".to_owned(),
            bytes: 48_213,
        };
        assert_eq!(serde_json::from_str::<Pasted>(PASTED).expect("a pasted file"), expected);
    }

    #[test]
    fn ignores_keys_it_does_not_know() {
        let line = json!({ "pane": 4, "path": "/tmp/shot.png", "bytes": 12, "kind": "png", "folder": { "kept": 7 } });
        let pasted = Pasted::deserialize(line).expect("a pasted file");
        assert_eq!(pasted, Pasted { pane: 4, path: "/tmp/shot.png".to_owned(), bytes: 12 });
    }

    #[test]
    fn leaves_missing_keys_empty() {
        let pasted = Pasted::deserialize(json!({ "pane": 4 })).expect("a pasted file");
        assert_eq!(pasted, Pasted { pane: 4, ..Pasted::default() });
    }
}

mod argv {
    use super::*;

    #[test]
    fn pastes_standard_input_into_one_pane_as_json() {
        assert_eq!(args(12, "shot.png"), words(&["paste-file", "--pane", "12", "--name=shot.png", "--json", "-"]));
    }

    #[rstest]
    #[case::a_leading_dash("-shot.png", "--name=-shot.png")]
    #[case::a_leading_double_dash("--json.png", "--name=--json.png")]
    #[case::spaces("my shot.jpg", "--name=my shot.jpg")]
    #[case::an_equals_sign("a=b.png", "--name=a=b.png")]
    fn keeps_the_name_glued_to_its_option(#[case] name: &str, #[case] option: &str) {
        assert_eq!(args(12, name), words(&["paste-file", "--pane", "12", option, "--json", "-"]));
    }
}

mod naming {
    use super::*;

    #[rstest]
    #[case::a_heic_photo(Some("IMG_0042.HEIC"), Kind::Jpeg, "IMG_0042.jpg")]
    #[case::a_jpeg_photo(Some("IMG_0042.JPG"), Kind::Jpeg, "IMG_0042.jpg")]
    #[case::a_screenshot(Some("Screenshot 2026-11-09.png"), Kind::Png, "Screenshot 2026-11-09.png")]
    #[case::a_png_turned_jpeg(Some("diagram.png"), Kind::Jpeg, "diagram.jpg")]
    #[case::no_extension(Some("IMG_0042"), Kind::Jpeg, "IMG_0042.jpg")]
    #[case::two_extensions(Some("shot.tar.heic"), Kind::Jpeg, "shot.tar.jpg")]
    #[case::spaces_around_it(Some("  shot.heic  "), Kind::Jpeg, "shot.jpg")]
    #[case::a_folder_before_it(Some("Photos/IMG_0042.HEIC"), Kind::Jpeg, "IMG_0042.jpg")]
    #[case::no_name_as_jpeg(None, Kind::Jpeg, "pasted.jpg")]
    #[case::no_name_as_png(None, Kind::Png, "pasted.png")]
    #[case::an_empty_name(Some(""), Kind::Png, "pasted.png")]
    #[case::only_spaces(Some("   "), Kind::Jpeg, "pasted.jpg")]
    #[case::only_a_folder(Some("/"), Kind::Jpeg, "pasted.jpg")]
    fn takes_the_extension_of_the_output_format(
        #[case] original: Option<&str>,
        #[case] kind: Kind,
        #[case] expected: &str,
    ) {
        assert_eq!(output_name(original, kind), expected);
    }

    #[rstest]
    #[case::png("png", Kind::Png)]
    #[case::jpeg("jpeg", Kind::Jpeg)]
    fn reads_the_kind_the_picker_names(#[case] name: &str, #[case] kind: Kind) {
        assert_eq!(Kind::deserialize(json!(name)).expect("a kind"), kind);
    }

    #[test]
    fn refuses_a_kind_it_does_not_encode() {
        assert!(Kind::deserialize(json!("heic")).is_err());
    }
}

mod checking {
    use super::*;

    #[test]
    fn refuses_an_empty_image() {
        assert_eq!(checked(Vec::new()), Err(Error::ImageEmpty));
    }

    #[test]
    fn refuses_one_byte_over_32_mib() {
        assert_eq!(checked(vec![0; MAX_IMAGE_BYTES + 1]), Err(Error::ImageTooLarge));
    }

    #[test]
    fn takes_exactly_32_mib() {
        let bytes = vec![7; MAX_IMAGE_BYTES];
        assert_eq!(checked(bytes.clone()), Ok(bytes));
    }

    #[test]
    fn takes_a_single_byte() {
        assert_eq!(checked(vec![1]), Ok(vec![1]));
    }

    #[test]
    fn caps_at_what_cornercase_takes() {
        assert_eq!(MAX_IMAGE_BYTES, 33_554_432);
    }
}
