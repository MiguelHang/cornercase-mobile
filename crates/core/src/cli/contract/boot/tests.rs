use super::*;
use crate::cli::contract::tests::REAL;

fn boot(text: &str) -> Boot {
    Boot(text.to_owned())
}

mod observe {
    use super::*;

    #[test]
    fn takes_the_first_boot_it_sees_as_the_same_server() {
        assert_eq!(BootWatch::default().observe(boot("28715-1")), Observed::Same);
    }

    #[test]
    fn says_same_while_the_boot_does_not_change() {
        let mut watch = BootWatch::default();
        watch.observe(boot("28715-1"));
        assert_eq!(watch.observe(boot("28715-1")), Observed::Same);
    }

    #[test]
    fn says_restarted_once_when_the_boot_changes() {
        let mut watch = BootWatch::default();
        watch.observe(boot("28715-1"));
        assert_eq!(watch.observe(boot("30102-2")), Observed::Restarted);
        assert_eq!(watch.observe(boot("30102-2")), Observed::Same);
    }

    #[test]
    fn says_same_for_the_same_mac_reached_by_another_route() {
        let first: Descriptor = REAL.parse().expect("the real contract over one route");
        let second: Descriptor = REAL.parse().expect("the real contract over another route");
        let mut watch = BootWatch::default();
        watch.observe(first.boot());
        assert_eq!(watch.observe(second.boot()), Observed::Same);
    }
}

mod descriptor {
    use super::*;

    #[test]
    fn is_the_boot_the_server_says() {
        let descriptor: Descriptor = REAL.parse().expect("the real contract");
        assert_eq!(descriptor.boot(), boot("28715-1794075132409911000"));
    }
}
