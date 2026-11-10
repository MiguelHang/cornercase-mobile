use rstest::rstest;

use super::*;
use crate::network::Kind;

const URL: &str = "ws://192.168.1.20:8765/?token=AbCdEfGhIjKlMnOpQrStUvWxYz0123456789-_AbCdE";
const FOREGROUND: &str = "\x1b[38;5;";
const BACKGROUND: &str = "m\x1b[48;5;";

fn ip(text: &str) -> IpAddr {
    text.parse().expect("a valid IP address")
}

fn module(colour: &str) -> bool {
    match colour {
        "16" => true,
        "231" => false,
        other => panic!("{other} is not one of the two colours"),
    }
}

fn cells(line: &str) -> Vec<(bool, bool)> {
    let body = line.strip_suffix(RESET).expect("the line ends with the reset");
    let (cells, rest) = body.rsplit_once(HALF).expect("at least one half block");
    assert_eq!(rest, "", "nothing after the last half block");
    cells
        .split(HALF)
        .map(|cell| {
            let colours = cell.strip_prefix(FOREGROUND).and_then(|colours| colours.strip_suffix('m'));
            let (top, bottom) = colours.and_then(|colours| colours.split_once(BACKGROUND)).expect("two colours");
            (module(top), module(bottom))
        })
        .collect()
}

fn drawn(url: &str) -> Vec<Vec<bool>> {
    let mut rows = Vec::new();
    for line in lines(url) {
        let (top, bottom): (Vec<bool>, Vec<bool>) = cells(&line).into_iter().unzip();
        rows.extend([top, bottom]);
    }
    rows
}

fn encoded(url: &str) -> Vec<Vec<bool>> {
    let code = QrCode::with_error_correction_level(url.as_bytes(), EcLevel::L).expect("the URL fits");
    let width = code.width();
    let side = width + 2 * QUIET;
    let dark: Vec<bool> = code.to_colors().into_iter().map(|module| module == Color::Dark).collect();
    let mut rows = vec![vec![false; side]; side.next_multiple_of(2)];
    for (index, module) in dark.into_iter().enumerate() {
        rows[QUIET + index / width][QUIET + index % width] = module;
    }
    rows
}

fn width_of(url: &str) -> usize {
    QrCode::with_error_correction_level(url.as_bytes(), EcLevel::L).expect("the URL fits").width()
}

mod url {
    use super::*;

    #[rstest]
    #[case::lan("192.168.1.20", 8765, "ws://192.168.1.20:8765/?token=t0k-en_")]
    #[case::tailscale_v4("100.101.102.103", 8765, "ws://100.101.102.103:8765/?token=t0k-en_")]
    #[case::tailscale_v6("fd7a:115c:a1e0::1", 8765, "ws://[fd7a:115c:a1e0::1]:8765/?token=t0k-en_")]
    #[case::loopback_v4("127.0.0.1", 40123, "ws://127.0.0.1:40123/?token=t0k-en_")]
    #[case::loopback_v6("::1", 40123, "ws://[::1]:40123/?token=t0k-en_")]
    fn puts_the_address_port_and_token_together(#[case] address: &str, #[case] port: u16, #[case] wanted: &str) {
        assert_eq!(url(ip(address), port, "t0k-en_"), wanted);
    }
}

mod lines {
    use super::*;

    #[rstest]
    #[case::lan(URL)]
    #[case::tailscale_v6(
        "ws://[fd7a:115c:a1e0:ab12:4843:cd96:6258:b240]:65535/?token=AbCdEfGhIjKlMnOpQrStUvWxYz0123456789-_AbCdE"
    )]
    fn are_as_wide_as_the_code_and_its_quiet_zone(#[case] url: &str) {
        let side = width_of(url) + 2 * QUIET;
        let lines = lines(url);
        assert_eq!(lines.len(), side.div_ceil(2));
        assert!(lines.iter().all(|line| cells(line).len() == side), "{lines:?}");
    }

    #[test]
    fn draw_the_modules_of_the_url_top_in_front_and_bottom_behind() {
        assert_eq!(drawn(URL), encoded(URL));
    }

    #[test]
    fn begin_and_end_with_the_quiet_zone() {
        let lines = lines(URL);
        for line in [lines.first(), lines.last()].map(|line| line.expect("a line")) {
            assert!(cells(line).iter().all(|&(top, bottom)| !top && !bottom), "{line:?}");
        }
    }

    #[test]
    fn use_only_half_blocks_two_colours_and_the_reset() {
        let line = &lines(URL)[QUIET];
        let stripped = line
            .replace(RESET, "")
            .replace(&format!("{FOREGROUND}16m"), "")
            .replace(&format!("{FOREGROUND}231m"), "")
            .replace("\x1b[48;5;16m", "")
            .replace("\x1b[48;5;231m", "");
        assert!(stripped.chars().all(|character| character.to_string() == HALF), "{stripped:?}");
        assert!(line.contains(&format!("{FOREGROUND}16m")), "the line has dark modules");
    }

    #[test]
    fn are_empty_when_the_url_does_not_fit_in_a_code() {
        assert_eq!(lines(&"a".repeat(3000)), Vec::<String>::new());
    }
}

mod block {
    use super::*;

    const TOKEN: &str = "AbCdEfGhIjKlMnOpQrStUvWxYz0123456789-_AbCdE";
    const LAN: &str = "ws://192.168.1.20:8765/?token=AbCdEfGhIjKlMnOpQrStUvWxYz0123456789-_AbCdE";
    const TAILSCALE: &str = "ws://100.101.102.103:8765/?token=AbCdEfGhIjKlMnOpQrStUvWxYz0123456789-_AbCdE";

    fn target(kind: Kind, interface: &str, address: &str) -> Target {
        Target { kind, interface: interface.to_owned(), ip: ip(address) }
    }

    fn printed(name: Option<&str>, targets: &[Target], port: u16) -> Vec<String> {
        let places: Vec<_> = targets.iter().map(|target| (Place::Network(target), target.ip)).collect();
        block(name, &places, port, TOKEN).expect("a valid token")
    }

    fn assert_block(block: &[String], heading: &str, code: &str, urls: &[&str]) {
        let qr = lines(code);
        assert_eq!(block[..2], [heading, code]);
        assert_eq!(block[2..2 + qr.len()], qr);
        assert_eq!(block[2 + qr.len()], "");
        assert_eq!(block[3 + qr.len()..], urls.iter().map(|&url| url.to_owned()).collect::<Vec<_>>());
    }

    #[test]
    fn gives_one_address_as_a_code_of_one_route_with_the_name() {
        let block = printed(Some("Mac Studio"), &[target(Kind::Lan, "en0", "192.168.1.20")], 8765);
        let code = format!("{LAN}&name=Mac%20Studio");
        assert_block(&block, "Mac Studio — LAN (en0)", &code, &[&format!("LAN (en0)  {LAN}")]);
    }

    #[test]
    fn gives_every_address_in_one_code_and_one_line_each_below_it() {
        let targets = [target(Kind::Lan, "en0", "192.168.1.20"), target(Kind::Tailscale, "utun9", "100.101.102.103")];
        let block = printed(Some("MacBook Pro"), &targets, 8765);
        let code = format!("{LAN}&also=100.101.102.103&name=MacBook%20Pro");
        let urls = [format!("LAN (en0)  {LAN}"), format!("Tailscale (utun9)  {TAILSCALE}")];
        assert_block(&block, "MacBook Pro — LAN (en0), Tailscale (utun9)", &code, &urls.each_ref().map(String::as_str));
    }

    #[test]
    fn names_an_interface_once_when_it_has_two_addresses() {
        let targets = [
            target(Kind::Tailscale, "utun9", "100.101.102.103"),
            target(Kind::Tailscale, "utun9", "fd7a:115c:a1e0::1"),
        ];
        let block = printed(None, &targets, 8765);
        let v6 = "ws://[fd7a:115c:a1e0::1]:8765/?token=AbCdEfGhIjKlMnOpQrStUvWxYz0123456789-_AbCdE";
        let urls = [format!("Tailscale (utun9)  {TAILSCALE}"), format!("Tailscale (utun9)  {v6}")];
        let code = format!("{TAILSCALE}&also=fd7a:115c:a1e0::1");
        assert_block(&block, "Tailscale (utun9)", &code, &urls.each_ref().map(String::as_str));
    }

    #[test]
    fn gives_every_line_the_listeners_port() {
        let places = [(Place::Loopback, ip("127.0.0.1")), (Place::Loopback, ip("::1"))];
        let block = block(None, &places, 40123, TOKEN).expect("a valid token");
        let wanted = [
            format!("loopback  ws://127.0.0.1:40123/?token={TOKEN}"),
            format!("loopback  ws://[::1]:40123/?token={TOKEN}"),
        ];
        assert_eq!(block[block.len() - 2..], wanted);
    }

    #[rstest]
    #[case::with_a_name(Some("studio"), "studio — loopback", "&name=studio")]
    #[case::without_a_name(None, "loopback", "")]
    #[case::with_a_blank_name(Some("  "), "loopback", "")]
    fn gives_only_the_loopback_address_on_loopback(
        #[case] name: Option<&str>,
        #[case] heading: &str,
        #[case] suffix: &str,
    ) {
        let url = format!("ws://127.0.0.1:40123/?token={TOKEN}");
        let block = block(name, &[(Place::Loopback, ip("127.0.0.1"))], 40123, TOKEN).expect("a valid token");
        assert_block(&block, heading, &format!("{url}{suffix}"), &[&format!("loopback  {url}")]);
    }

    #[test]
    fn heads_the_block_with_the_name_the_code_keeps() {
        let block =
            block(Some(&"m".repeat(70)), &[(Place::Loopback, ip("127.0.0.1"))], 1, TOKEN).expect("a valid token");
        assert_eq!(block[0], format!("{} — loopback", "m".repeat(64)));
    }

    #[test]
    fn is_empty_without_an_address() {
        assert_eq!(block(Some("studio"), &[], 8765, TOKEN), Ok(Vec::new()));
    }

    #[rstest]
    #[case::empty("", RouteError::EmptyToken)]
    #[case::not_base64url("a+b/c=", RouteError::TokenCharacters)]
    fn refuses_a_token_the_app_would_refuse(#[case] token: &str, #[case] error: RouteError) {
        assert_eq!(block(None, &[(Place::Loopback, ip("127.0.0.1"))], 1, token), Err(error));
    }

    #[test]
    fn fits_in_eighty_columns_with_three_addresses_and_the_longest_name() {
        let targets = [
            target(Kind::Lan, "en0", "192.168.100.200"),
            target(Kind::Tailscale, "utun9", "100.101.102.103"),
            target(Kind::Tailscale, "utun9", "fd7a:115c:a1e0:ab12:4843:cd96:6258:b240"),
        ];
        let name: String = "Mac Studio de la oficina 2 ".repeat(3).chars().take(64).collect();
        let block = printed(Some(&name), &targets, 65535);
        let code = &block[1];
        assert!(code.ends_with(&format!("&name={}", name.replace(' ', "%20"))), "{code}");
        let widest = lines(code).iter().map(|line| cells(line).len()).max().expect("the code is drawn");
        assert!(widest <= 80, "{widest} columns");
    }
}
