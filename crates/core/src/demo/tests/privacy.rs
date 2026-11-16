use std::fs;
use std::net::Ipv4Addr;
use std::path::{Path, PathBuf};

use rstest::rstest;

const SOURCES: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/src");
const ITSELF: &str = "privacy.rs";
const NAMES: [&str; 3] = ["miguel", "reveni", "@gmail"];
const HOMES: [&str; 2] = ["/Users/", "/home/"];
const FICTIONAL_HOME: &str = "demo";
const FICTIONAL_MAIL: [&str; 3] = ["example.com", "example.org", "example.net"];
const TOKEN_PREFIXES: [&str; 10] =
    ["ghp_", "gho_", "ghu_", "ghs_", "ghr_", "github_pat_", "xoxb-", "xoxp-", "sk-", "AKIA"];
const TOKEN_SHORTEST: usize = 20;
const SECRET_SHORTEST: usize = 32;
const DOCUMENTATION: [[u8; 3]; 3] = [[192, 0, 2], [198, 51, 100], [203, 0, 113]];

fn files(folder: &Path) -> Vec<PathBuf> {
    let entries = fs::read_dir(folder).expect("the demo's sources are readable");
    let paths = entries.map(|entry| entry.expect("a readable entry").path());
    paths.flat_map(|path| if path.is_dir() { files(&path) } else { vec![path] }).collect()
}

fn scanned() -> Vec<PathBuf> {
    let root = Path::new(SOURCES);
    let mut paths = files(&root.join("demo"));
    paths.push(root.join("demo.rs"));
    paths.retain(|path| !path.ends_with(ITSELF));
    paths
}

fn named(text: &str) -> Vec<String> {
    let lowered = text.to_lowercase();
    NAMES.iter().filter(|name| lowered.contains(*name)).map(|name| (*name).to_owned()).collect()
}

fn homes(text: &str) -> Vec<String> {
    let after = HOMES.iter().flat_map(|home| text.split(home).skip(1).map(move |rest| (home, rest)));
    let foreign = after.filter(|(_, rest)| rest.split('/').next() != Some(FICTIONAL_HOME));
    foreign.map(|(home, rest)| format!("{home}{}", rest.split(['/', '"', ' ']).next().unwrap_or_default())).collect()
}

fn domain(rest: &str) -> &str {
    let end = rest.find(|letter: char| !(letter.is_ascii_alphanumeric() || letter == '.' || letter == '-'));
    rest[..end.unwrap_or(rest.len())].trim_end_matches('.')
}

fn mails(text: &str) -> Vec<String> {
    let domains = text.split('@').skip(1).map(domain);
    let real = domains.filter(|domain| domain.contains('.') && !FICTIONAL_MAIL.contains(domain));
    real.map(|domain| format!("@{domain}")).collect()
}

fn reserved(address: Ipv4Addr) -> bool {
    let [first, second, third, _] = address.octets();
    let documentation = DOCUMENTATION.contains(&[first, second, third]);
    let local = address.is_private() || address.is_loopback() || address.is_link_local();
    documentation || local || address.is_unspecified() || address.is_broadcast()
}

fn addresses(text: &str) -> Vec<String> {
    let numbers = text.split(|letter: char| !(letter.is_ascii_digit() || letter == '.'));
    let parsed = numbers.filter_map(|number| number.trim_matches('.').parse::<Ipv4Addr>().ok());
    parsed.filter(|address| !reserved(*address)).map(|address| address.to_string()).collect()
}

fn prefixed(word: &str) -> bool {
    word.len() >= TOKEN_SHORTEST && TOKEN_PREFIXES.iter().any(|prefix| word.starts_with(prefix))
}

fn secret(word: &str) -> bool {
    let runs = word.split(|letter: char| !letter.is_ascii_alphanumeric());
    let long = |run: &&str| run.len() >= SECRET_SHORTEST;
    runs.filter(long)
        .any(|run| run.contains(|letter: char| letter.is_ascii_digit()) && run.contains(char::is_alphabetic))
}

fn tokens(text: &str) -> Vec<String> {
    let words = text.split(|letter: char| !(letter.is_ascii_alphanumeric() || letter == '_' || letter == '-'));
    let keys = text.contains("PRIVATE KEY").then(|| "PRIVATE KEY".to_owned());
    words.filter(|word| prefixed(word) || secret(word)).map(str::to_owned).chain(keys).collect()
}

fn markers(text: &str) -> Vec<String> {
    [named(text), homes(text), mails(text), addresses(text), tokens(text)].concat()
}

#[test]
fn the_demo_names_no_one_real() {
    let found: Vec<(PathBuf, Vec<String>)> = scanned()
        .into_iter()
        .map(|path| {
            let text = fs::read_to_string(&path).expect("the demo's sources are text");
            (path.strip_prefix(SOURCES).unwrap_or(&path).to_owned(), markers(&text))
        })
        .filter(|(_, markers)| !markers.is_empty())
        .collect();
    assert_eq!(found, []);
}

#[rstest]
#[case::the_data("data/accounts.json")]
#[case::the_simulation("chat.rs")]
#[case::the_tests("start/tests.rs")]
#[case::the_module("demo.rs")]
fn the_scan_reads(#[case] file: &str) {
    assert!(scanned().iter().any(|path| path.ends_with(file)), "{file} is not scanned");
}

#[rstest]
#[case::a_name("written by Miguel")]
#[case::a_company("see reveni.io")]
#[case::a_mail_at_gmail("someone@gmail")]
#[case::a_real_mail("someone@company.dev")]
#[case::a_home("cd /Users/someone/src")]
#[case::a_linux_home("/home/someone")]
#[case::a_public_address("ssh 8.8.8.8")]
#[case::a_private_key("-----BEGIN OPENSSH PRIVATE KEY-----")]
fn the_scan_finds(#[case] text: &str) {
    assert_ne!(markers(text), Vec::<String>::new(), "{text}");
}

#[rstest]
#[case::a_github_token("ghp_", 18)]
#[case::an_openai_key("sk-", 16)]
#[case::an_aws_key("AKIA", 8)]
#[case::a_bare_secret("", 16)]
fn the_scan_finds_a_token(#[case] prefix: &str, #[case] pairs: usize) {
    let token = format!("{prefix}{}", "a1".repeat(pairs));
    assert_eq!(markers(&format!("token = \"{token}\"")), [token]);
}

#[rstest]
#[case::the_fictional_home("/Users/demo/src/acme-store")]
#[case::an_example_mail("demo@example.com")]
#[case::a_local_address("127.0.0.1 and 10.0.2.2 and 192.168.1.20")]
#[case::a_documentation_address("203.0.113.7")]
#[case::a_version("v1.2.3")]
#[case::a_long_test_name("puts_the_agent_to_work_and_then_back_to_idle")]
#[case::a_word_with_a_name_inside("changes")]
fn the_scan_lets_through(#[case] text: &str) {
    assert_eq!(markers(text), Vec::<String>::new(), "{text}");
}
