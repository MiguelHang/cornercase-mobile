use rstest::rstest;

use super::*;

const SECRET_WORD: &str = "do-not-repeat-me";

mod encoded {
    use super::*;

    #[rstest]
    #[case::nothing(b"", "")]
    #[case::text(b"ok", "6f6b")]
    #[case::every_nibble(&[0x01, 0x23, 0x45, 0x67, 0x89, 0xab, 0xcd, 0xef], "0123456789abcdef")]
    #[case::bytes_that_are_not_utf8(&[0x00, 0xff, 0xfe, 0x80], "00fffe80")]
    fn writes_two_lower_case_digits_per_byte(#[case] value: &[u8], #[case] expected: &str) {
        assert_eq!(encoded(value), expected);
    }
}

mod decoded {
    use super::*;

    #[rstest]
    #[case::nothing(Vec::new())]
    #[case::text(b"a-token".to_vec())]
    #[case::bytes_that_are_not_utf8(vec![0xff, 0xff, 0x00, 0xfe, 0x80, 0x0a])]
    #[case::every_byte((0..=u8::MAX).collect())]
    fn reads_back_what_was_encoded(#[case] value: Vec<u8>) {
        assert_eq!(decoded(&encoded(&value)), Ok(value));
    }

    #[test]
    fn reads_upper_case_digits() {
        assert_eq!(decoded("FF0A"), Ok(vec![0xff, 0x0a]));
    }

    #[rstest]
    #[case::not_hex_digits(SECRET_WORD)]
    #[case::odd_length("abc")]
    #[case::one_digit("f")]
    #[case::a_sign("+f")]
    #[case::not_ascii("éé")]
    fn fails_without_repeating_the_value(#[case] hex: &str) {
        assert_eq!(decoded(hex), Err(Error::Secrets("a secret is not hex".to_owned())));
    }
}
