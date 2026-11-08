use rstest::rstest;

use super::*;

fn ip(text: &str) -> IpAddr {
    text.parse().expect("a valid IP address")
}

fn addresses(list: &[(&str, &str)]) -> Vec<(String, IpAddr)> {
    list.iter().map(|(interface, address)| ((*interface).to_owned(), ip(address))).collect()
}

fn target(kind: Kind, interface: &str, address: &str) -> Target {
    Target { kind, interface: interface.to_owned(), ip: ip(address) }
}

mod eligible {
    use super::*;

    #[rstest]
    #[case::ten_first("10.0.0.0", Some(Kind::Lan))]
    #[case::ten_last("10.255.255.255", Some(Kind::Lan))]
    #[case::below_172_16("172.15.255.255", None)]
    #[case::first_of_172_16("172.16.0.0", Some(Kind::Lan))]
    #[case::last_of_172_16("172.31.255.255", Some(Kind::Lan))]
    #[case::above_172_31("172.32.0.0", None)]
    #[case::below_192_168("192.167.255.255", None)]
    #[case::first_of_192_168("192.168.0.0", Some(Kind::Lan))]
    #[case::last_of_192_168("192.168.255.255", Some(Kind::Lan))]
    #[case::above_192_168("192.169.0.0", None)]
    #[case::below_100_64("100.63.255.255", None)]
    #[case::first_of_100_64("100.64.0.0", Some(Kind::Tailscale))]
    #[case::tailscale_address("100.101.102.103", Some(Kind::Tailscale))]
    #[case::last_of_100_64("100.127.255.255", Some(Kind::Tailscale))]
    #[case::above_100_127("100.128.0.0", None)]
    #[case::tailscale_v6_first("fd7a:115c:a1e0::", Some(Kind::Tailscale))]
    #[case::tailscale_v6_inside("fd7a:115c:a1e0:ffff::1", Some(Kind::Tailscale))]
    #[case::tailscale_v6_last("fd7a:115c:a1e0:ffff:ffff:ffff:ffff:ffff", Some(Kind::Tailscale))]
    #[case::next_v6_prefix("fd7a:115c:a1e1::1", None)]
    #[case::previous_v6_prefix("fd7a:115c:a1df:ffff::1", None)]
    #[case::other_ula("fd00::1", None)]
    #[case::mapped_lan("::ffff:10.0.0.1", None)]
    #[case::mapped_tailscale("::ffff:100.64.0.1", None)]
    #[case::loopback_v4("127.0.0.1", None)]
    #[case::loopback_v4_other("127.255.255.254", None)]
    #[case::loopback_v6("::1", None)]
    #[case::link_local_v4("169.254.1.1", None)]
    #[case::link_local_v6("fe80::1", None)]
    #[case::unspecified_v4("0.0.0.0", None)]
    #[case::unspecified_v6("::", None)]
    #[case::broadcast("255.255.255.255", None)]
    #[case::multicast_v4("224.0.0.1", None)]
    #[case::multicast_v6("ff02::1", None)]
    #[case::public_next_to_ten("11.0.0.1", None)]
    #[case::public_v4("8.8.8.8", None)]
    #[case::public_v6("2001:db8::1", None)]
    fn classifies(#[case] address: &str, #[case] expected: Option<Kind>) {
        assert_eq!(eligible(ip(address)), expected);
    }
}

mod kind {
    use super::*;

    #[rstest]
    #[case::lan(Kind::Lan, "LAN")]
    #[case::tailscale(Kind::Tailscale, "Tailscale")]
    fn has_a_label(#[case] kind: Kind, #[case] label: &str) {
        assert_eq!(kind.label(), label);
    }
}

mod targets {
    use super::*;

    #[rstest]
    #[case::no_addresses(&[])]
    #[case::no_eligible_address(&[("lo0", "127.0.0.1"), ("lo0", "::1"), ("en0", "fe80::1"), ("en1", "8.8.8.8")])]
    fn refuses_without_an_eligible_address(#[case] list: &[(&str, &str)]) {
        assert_eq!(targets(&addresses(list)), Err(NoAddress));
    }

    #[test]
    fn says_why_it_refuses() {
        assert_eq!(NoAddress.to_string(), "no LAN or Tailscale address to listen on");
    }

    #[test]
    fn keeps_only_the_eligible_addresses() {
        let list = addresses(&[("lo0", "127.0.0.1"), ("en0", "192.168.1.20"), ("en0", "fe80::1"), ("en1", "1.2.3.4")]);
        assert_eq!(targets(&list), Ok(vec![target(Kind::Lan, "en0", "192.168.1.20")]));
    }

    #[test]
    fn listens_once_on_an_address_of_two_interfaces() {
        let list = addresses(&[("en0", "10.0.0.5"), ("bridge0", "10.0.0.5")]);
        assert_eq!(targets(&list), Ok(vec![target(Kind::Lan, "en0", "10.0.0.5")]));
    }

    #[test]
    fn puts_the_lan_first_then_orders_by_address() {
        let list = addresses(&[
            ("utun4", "fd7a:115c:a1e0::1"),
            ("utun4", "100.100.1.2"),
            ("en0", "192.168.1.20"),
            ("bridge100", "10.0.0.1"),
            ("en1", "172.16.0.9"),
        ]);
        let expected = vec![
            target(Kind::Lan, "bridge100", "10.0.0.1"),
            target(Kind::Lan, "en1", "172.16.0.9"),
            target(Kind::Lan, "en0", "192.168.1.20"),
            target(Kind::Tailscale, "utun4", "100.100.1.2"),
            target(Kind::Tailscale, "utun4", "fd7a:115c:a1e0::1"),
        ];
        assert_eq!(targets(&list), Ok(expected));
    }

    #[test]
    fn gives_the_same_order_whatever_the_order_of_the_interfaces() {
        let list = addresses(&[("en0", "192.168.1.20"), ("utun4", "100.100.1.2"), ("en1", "10.0.0.1")]);
        let mut reversed = list.clone();
        reversed.reverse();
        assert_eq!(targets(&list), targets(&reversed));
    }
}

mod interfaces {
    use super::*;

    #[test]
    fn list_the_loopback_address() {
        let interfaces = interfaces().expect("the interfaces");
        assert!(interfaces.iter().any(|(_, address)| *address == ip("127.0.0.1")), "{interfaces:?}");
    }
}

mod listen {
    use std::net::TcpStream;

    use super::*;

    fn addresses_of(listeners: &[TcpListener]) -> Vec<SocketAddr> {
        listeners.iter().map(|listener| listener.local_addr().expect("the address")).collect()
    }

    #[tokio::test]
    async fn opens_every_address_on_the_port_the_first_one_picked() {
        let listeners = listen(&[ip("127.0.0.1"), ip("::1")], 0).await.expect("the listeners");
        let addresses = addresses_of(&listeners);
        let port = addresses[0].port();
        assert_ne!(port, 0);
        assert_eq!(addresses, [SocketAddr::new(ip("127.0.0.1"), port), SocketAddr::new(ip("::1"), port)]);
    }

    #[tokio::test]
    async fn opens_every_address_on_the_port_it_is_given() {
        let port =
            listen(&[ip("::1")], 0).await.expect("a free port").remove(0).local_addr().expect("its address").port();
        let listeners = listen(&[ip("::1"), ip("127.0.0.1")], port).await.expect("the listeners");
        assert_eq!(
            addresses_of(&listeners),
            [SocketAddr::new(ip("::1"), port), SocketAddr::new(ip("127.0.0.1"), port)]
        );
    }

    #[tokio::test]
    async fn names_the_address_it_could_not_listen_on_and_closes_the_others() {
        let taken = std::net::TcpListener::bind("127.0.0.1:0").expect("a taken port");
        let port = taken.local_addr().expect("its address").port();
        let error = listen(&[ip("::1"), ip("127.0.0.1")], port).await.expect_err("the port is taken");
        assert_eq!(error.kind(), io::ErrorKind::AddrInUse);
        let wanted = format!("could not listen on 127.0.0.1:{port}: ");
        assert!(error.to_string().starts_with(&wanted), "{error}");
        assert!(TcpStream::connect(SocketAddr::new(ip("::1"), port)).is_err(), "the listener on ::1 is closed");
    }
}
