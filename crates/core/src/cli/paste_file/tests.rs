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

mod picking {
    use super::*;

    #[test]
    fn reads_the_photo_the_picker_wrote() {
        let answer = json!({ "path": "/tmp/7F3A.jpg", "name": "IMG_0042.HEIC", "kind": "jpeg" });
        let picked = Picking::deserialize(answer).expect("an answer").picked();
        let expected = Picked {
            path: "/tmp/7F3A.jpg".to_owned(),
            name: Some("IMG_0042.HEIC".to_owned()),
            kind: Kind::Jpeg,
            thumbnail_path: None,
        };
        assert_eq!(picked, Some(expected));
    }

    #[test]
    fn reads_the_thumbnail_the_picker_wrote_beside_it() {
        let answer = json!({ "path": "/tmp/7F3A.jpg", "kind": "jpeg", "thumbnailPath": "/tmp/7F3A-small.jpg" });
        let picked = Picking::deserialize(answer).expect("an answer").picked().expect("a photo");
        assert_eq!(picked.thumbnail_path.as_deref(), Some("/tmp/7F3A-small.jpg"));
    }

    #[test]
    fn takes_a_photo_without_a_thumbnail() {
        let answer = json!({ "path": "/tmp/7F3A.jpg", "name": "IMG_0042.HEIC", "kind": "jpeg" });
        let picked = Picking::deserialize(answer).expect("an answer").picked().expect("a photo");
        assert_eq!(picked.thumbnail_path, None);
    }

    #[test]
    fn takes_a_null_thumbnail_as_none() {
        let answer = json!({ "path": "/tmp/7F3A.jpg", "kind": "jpeg", "thumbnailPath": null });
        let picked = Picking::deserialize(answer).expect("an answer").picked().expect("a photo");
        assert_eq!(picked.thumbnail_path, None);
    }

    #[test]
    fn takes_a_photo_without_a_name() {
        let picked =
            Picking::deserialize(json!({ "path": "/tmp/7F3A.png", "kind": "png" })).expect("an answer").picked();
        let expected = Picked { path: "/tmp/7F3A.png".to_owned(), name: None, kind: Kind::Png, thumbnail_path: None };
        assert_eq!(picked, Some(expected));
    }

    #[test]
    fn reads_a_cancel_as_no_photo() {
        assert_eq!(Picking::deserialize(json!({ "cancelled": true })).expect("an answer").picked(), None);
    }

    #[rstest]
    #[case::nothing(json!({}))]
    #[case::no_path(json!({ "name": "IMG_0042.HEIC", "kind": "jpeg" }))]
    #[case::no_kind(json!({ "path": "/tmp/7F3A.jpg" }))]
    #[case::a_kind_it_does_not_encode(json!({ "path": "/tmp/7F3A.heic", "kind": "heic" }))]
    #[case::not_an_object(json!(null))]
    fn refuses_an_answer_that_is_neither(#[case] answer: serde_json::Value) {
        assert!(Picking::deserialize(answer).is_err());
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
    #[case::only_an_extension(Some(".heic"), Kind::Jpeg, "pasted.jpg")]
    #[case::dots_without_an_extension(Some("v1.2"), Kind::Jpeg, "v1.2.jpg")]
    #[case::a_time_without_an_extension(
        Some("Screenshot 2026-11-09 at 10.42.13"),
        Kind::Png,
        "Screenshot 2026-11-09 at 10.42.13.png"
    )]
    #[case::an_extension_of_no_image(Some("notes.txt"), Kind::Png, "notes.txt.png")]
    #[case::a_heif_photo(Some("IMG_0042.heif"), Kind::Jpeg, "IMG_0042.jpg")]
    #[case::a_webp_image(Some("sticker.WebP"), Kind::Png, "sticker.png")]
    #[case::a_gif_image(Some("loop.gif"), Kind::Png, "loop.png")]
    #[case::a_tiff_scan(Some("scan.tiff"), Kind::Jpeg, "scan.jpg")]
    #[case::a_tif_scan(Some("scan.TIF"), Kind::Jpeg, "scan.jpg")]
    #[case::a_jpeg_spelt_out(Some("photo.jpeg"), Kind::Jpeg, "photo.jpg")]
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

mod thumbnails {
    use super::*;

    #[test]
    fn makes_a_jpeg_data_url_of_the_bytes() {
        assert_eq!(thumbnail_url(&[0xFF, 0xD8, 0xFF, 0xE0]), "data:image/jpeg;base64,/9j/4A==");
    }

    #[test]
    fn uses_standard_base64_with_its_plus_and_slash() {
        assert_eq!(thumbnail_url(&[0xFB, 0xFF]), "data:image/jpeg;base64,+/8=");
    }

    #[test]
    fn makes_an_empty_data_url_of_no_bytes() {
        assert_eq!(thumbnail_url(&[]), "data:image/jpeg;base64,");
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
