use super::*;
use crate::commands::error::ErrorCode;

const PANE: u64 = 7;
const OTHER_PANE: u64 = 8;

fn store(images: &AttachedImages, pane: u64, name: &str) -> u64 {
    images.store(pane, name.to_owned(), name.as_bytes().to_vec()).expect("the image is stored")
}

fn filled(pane: u64) -> (AttachedImages, Vec<u64>) {
    let images = AttachedImages::default();
    let ids = (0..MAX_PER_PANE).map(|index| store(&images, pane, &format!("photo-{index}.jpg"))).collect();
    (images, ids)
}

mod storing {
    use super::*;

    #[test]
    fn keeps_the_image_until_it_is_taken() {
        let images = AttachedImages::default();
        let id = images.store(PANE, "photo.jpg".to_owned(), vec![1, 2, 3]).expect("the image is stored");
        let taken = images.take(PANE, id).expect("the image is there");
        assert_eq!(taken, Attached { id, pane: PANE, name: "photo.jpg".to_owned(), bytes: vec![1, 2, 3] });
        assert_eq!(images.take(PANE, id), None);
    }

    #[test]
    fn gives_every_image_its_own_id() {
        let images = AttachedImages::default();
        let first = store(&images, PANE, "a.jpg");
        let second = store(&images, OTHER_PANE, "b.jpg");
        let third = store(&images, PANE, "c.jpg");
        assert!(first != second && second != third && first != third, "{first} {second} {third}");
    }

    #[test]
    fn never_reuses_an_id_after_a_discard_or_a_clear() {
        let images = AttachedImages::default();
        let discarded = store(&images, PANE, "a.jpg");
        images.discard(&[discarded]);
        let cleared = store(&images, PANE, "b.jpg");
        images.clear();
        let fresh = store(&images, PANE, "c.jpg");
        assert!(fresh != discarded && fresh != cleared, "{discarded} {cleared} {fresh}");
    }
}

mod the_limit_per_pane {
    use super::*;

    #[test]
    fn refuses_one_more_image_in_a_full_pane() {
        let (images, _) = filled(PANE);
        let error = images.store(PANE, "one-more.jpg".to_owned(), vec![1]).expect_err("the pane is full");
        assert_eq!(error.code, ErrorCode::AttachedFull);
        assert!(error.message.contains(&MAX_PER_PANE.to_string()), "{}", error.message);
    }

    #[test]
    fn leaves_room_in_another_pane() {
        let (images, _) = filled(PANE);
        assert!(images.store(OTHER_PANE, "other.jpg".to_owned(), vec![1]).is_ok());
    }

    #[test]
    fn makes_room_again_once_an_image_is_discarded() {
        let (images, ids) = filled(PANE);
        let other = store(&images, OTHER_PANE, "other.jpg");
        images.discard(&[ids[0], other]);
        assert!(images.store(PANE, "again.jpg".to_owned(), vec![1]).is_ok());
    }
}

mod the_limit_in_bytes {
    use super::*;

    #[test]
    fn takes_images_up_to_it() {
        let images = AttachedImages::default();
        images.store(PANE, "big.png".to_owned(), vec![0; MAX_BYTES - 1]).expect("under the limit");
        assert!(images.store(OTHER_PANE, "small.png".to_owned(), vec![0]).is_ok());
    }

    #[test]
    fn counts_the_images_of_every_pane() {
        let images = AttachedImages::default();
        images.store(PANE, "big.png".to_owned(), vec![0; MAX_BYTES]).expect("at the limit");
        let error = images.store(OTHER_PANE, "small.png".to_owned(), vec![0]).expect_err("over the limit");
        assert_eq!(error.code, ErrorCode::AttachedFull);
    }

    #[test]
    fn frees_what_a_cleared_store_held() {
        let images = AttachedImages::default();
        images.store(PANE, "big.png".to_owned(), vec![0; MAX_BYTES]).expect("at the limit");
        images.clear();
        assert!(images.store(PANE, "small.png".to_owned(), vec![0]).is_ok());
    }
}

mod taking {
    use super::*;

    #[test]
    fn leaves_the_image_of_another_pane() {
        let images = AttachedImages::default();
        let id = store(&images, PANE, "photo.jpg");
        assert_eq!(images.take(OTHER_PANE, id), None);
        assert!(images.take(PANE, id).is_some());
    }

    #[test]
    fn finds_nothing_for_an_unknown_id() {
        let images = AttachedImages::default();
        store(&images, PANE, "photo.jpg");
        assert_eq!(images.take(PANE, 99), None);
    }
}

mod discarding {
    use super::*;

    #[test]
    fn ignores_unknown_ids_and_keeps_the_rest() {
        let images = AttachedImages::default();
        let kept = store(&images, PANE, "kept.jpg");
        let gone = store(&images, PANE, "gone.jpg");
        images.discard(&[gone, 99, 100]);
        assert_eq!(images.take(PANE, gone), None);
        assert!(images.take(PANE, kept).is_some());
    }

    #[test]
    fn does_nothing_on_an_empty_store() {
        let images = AttachedImages::default();
        images.discard(&[1]);
        images.clear();
        assert!(images.store(PANE, "photo.jpg".to_owned(), vec![1]).is_ok());
    }
}
