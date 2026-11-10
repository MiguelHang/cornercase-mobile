use std::future;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Mutex, PoisonError};

use futures_util::SinkExt;
use rstest::rstest;
use serde_json::json;

use super::*;
use crate::machines::MachineId;
use crate::machines::tests::{TOKEN, bridge};
use crate::transport::fake::FakeTransport;
use crate::transport::{Closed, Exit, Out};

const CONTRACT: &[&str] = &["contract", "--json"];
const LAN: &str = "192.168.1.20";
const TAILNET: &str = "fd7a:115c:a1e0::1";
const TAILNET_V4: &str = "100.101.102.103";
const PUBLIC: &str = "studio.example.com";
const GONE: &str = "10.0.0.9";

#[derive(Default)]
struct Network {
    transports: Vec<(&'static str, Arc<dyn Transport>)>,
    calls: Mutex<Vec<String>>,
}

impl Network {
    fn with(mut self, host: &'static str, transport: FakeTransport) -> Self {
        self.transports.push((host, Arc::new(transport)));
        self
    }

    fn connect(&self, route: &Route) -> Result<Arc<dyn Transport>, Error> {
        let host = &route.address().host;
        self.calls.lock().unwrap_or_else(PoisonError::into_inner).push(host.clone());
        let found = self.transports.iter().find(|(known, _)| known == host);
        found.map(|(_, transport)| transport.clone()).ok_or_else(|| Error::Connect("no route to host".to_owned()))
    }

    async fn pick(&self, routes: Vec<Route>) -> Result<Picked, PickError> {
        self.pick_from(machine(routes), PickConfig::default()).await
    }

    async fn pick_from(&self, machine: Machine, config: PickConfig) -> Result<Picked, PickError> {
        pick_route(&machine, |route: &Route| self.connect(route), config).await
    }

    fn calls(&self) -> Vec<String> {
        self.calls.lock().unwrap_or_else(PoisonError::into_inner).clone()
    }
}

fn machine(routes: Vec<Route>) -> Machine {
    Machine { id: MachineId(0), name: "studio".to_owned(), routes, last_route: None }
}

fn saved(routes: Vec<Route>, host: &str) -> Machine {
    Machine { last_route: Some(address(host)), ..machine(routes) }
}

fn route(host: &str) -> Route {
    bridge(host, 8765, Some(TOKEN))
}

fn address(host: &str) -> Address {
    route(host).address().clone()
}

fn after(delay: Duration, stdout: &'static str, code: i32) -> FakeTransport {
    FakeTransport::default().script(CONTRACT, move |_, mut out| async move {
        time::sleep(delay).await;
        out.send(Out::Stdout(stdout.to_owned())).await.map_err(|Closed| Error::OutputClosed)?;
        Ok(Exit { code, stderr: String::new() })
    })
}

fn answers_after(millis: u64, contract: &'static str) -> FakeTransport {
    after(Duration::from_millis(millis), contract, 0)
}

fn fails_after(millis: u64) -> FakeTransport {
    after(Duration::from_millis(millis), "", 1)
}

fn hangs() -> FakeTransport {
    FakeTransport::default().script(CONTRACT, |_, _| future::pending())
}

fn network_of(answers: Vec<(&'static str, FakeTransport)>) -> (Network, Vec<Route>) {
    let routes = answers.iter().map(|(host, _)| route(host)).collect();
    let network =
        answers.into_iter().fold(Network::default(), |network, (host, transport)| network.with(host, transport));
    (network, routes)
}

async fn races(network: &Network, machine: Machine, config: PickConfig) -> (Route, Duration) {
    let started = Instant::now();
    let picked = network.pick_from(machine, config).await.expect("a route answers");
    (picked.route, started.elapsed())
}

fn all_failed_at_the_deadline(result: Result<Picked, PickError>, started: Instant, errors: Vec<(Address, Error)>) {
    assert_eq!(result.map(|picked| picked.route), Err(PickError::AllFailed(errors)));
    assert_eq!(started.elapsed(), PickConfig::default().deadline);
}

struct Flag(Arc<AtomicBool>);

impl Drop for Flag {
    fn drop(&mut self) {
        self.0.store(true, Ordering::SeqCst);
    }
}

fn hangs_until_dropped(dropped: &Arc<AtomicBool>) -> FakeTransport {
    let dropped = dropped.clone();
    FakeTransport::default().script(CONTRACT, move |_, _| {
        let flag = Flag(dropped.clone());
        async move {
            let _flag = flag;
            future::pending().await
        }
    })
}

mod pick_route {
    use super::*;

    #[tokio::test(start_paused = true)]
    async fn gives_the_fastest_route_with_its_transport_and_its_contract() {
        let network = Network::default()
            .with(LAN, answers_after(300, r#"{"from":"lan"}"#))
            .with(TAILNET, answers_after(50, r#"{"from":"tailnet"}"#));
        let started = Instant::now();
        let picked = network.pick(vec![route(LAN), route(TAILNET)]).await.expect("a route answers");
        assert_eq!(started.elapsed(), Duration::from_millis(200));
        assert_eq!((picked.route.clone(), picked.contract.clone()), (route(TAILNET), json!({"from":"tailnet"})));
        let again =
            transport::json::<Value>(picked.transport.as_ref(), vec!["contract".to_owned(), "--json".to_owned()]);
        assert_eq!(again.await, Ok(json!({"from":"tailnet"})));
    }

    #[tokio::test(start_paused = true)]
    async fn is_not_held_up_by_a_route_that_fails_or_one_that_hangs() {
        let network = Network::default()
            .with(LAN, FakeTransport::default().fail(CONTRACT, Error::Connect("refused".to_owned())))
            .with(TAILNET, hangs())
            .with(PUBLIC, answers_after(1000, "{}"));
        let started = Instant::now();
        let picked = network.pick(vec![route(LAN), route(TAILNET), route(PUBLIC)]).await.expect("a route answers");
        assert_eq!((picked.route, started.elapsed()), (route(PUBLIC), Duration::from_millis(1150)));
    }

    #[tokio::test(start_paused = true)]
    async fn drops_the_routes_that_lose() {
        let dropped = Arc::new(AtomicBool::new(false));
        let network =
            Network::default().with(LAN, hangs_until_dropped(&dropped)).with(TAILNET, answers_after(10, "{}"));
        let started = Instant::now();
        let picked = network.pick(vec![route(LAN), route(TAILNET)]).await.expect("a route answers");
        assert_eq!((picked.route, started.elapsed()), (route(TAILNET), Duration::from_millis(160)));
        assert!(dropped.load(Ordering::SeqCst));
    }

    #[tokio::test(start_paused = true)]
    async fn gives_an_error_for_each_route_in_their_order_when_none_answers() {
        let network = Network::default()
            .with(LAN, FakeTransport::default().answer(CONTRACT, &[], &["unknown command"], 2))
            .with(TAILNET, answers_after(0, "not json"))
            .with(PUBLIC, hangs());
        let started = Instant::now();
        let result = network.pick(vec![route(LAN), route(TAILNET), route(PUBLIC), route(GONE)]).await;
        let errors = vec![
            (address(LAN), Error::Failed { code: 2, stderr: "unknown command".to_owned() }),
            (address(TAILNET), Error::Json("not JSON at line 1 column 2".to_owned())),
            (address(PUBLIC), Error::Timeout),
            (address(GONE), Error::Connect("no route to host".to_owned())),
        ];
        all_failed_at_the_deadline(result, started, errors);
    }

    #[tokio::test(start_paused = true)]
    async fn counts_the_deadline_from_the_start() {
        let network =
            Network::default().with(LAN, after(Duration::from_secs(3), "", 1)).with(TAILNET, answers_after(6000, "{}"));
        let result = network.pick(vec![route(LAN), route(TAILNET)]).await;
        let errors =
            vec![(address(LAN), Error::Failed { code: 1, stderr: String::new() }), (address(TAILNET), Error::Timeout)];
        assert_eq!(result.map(|picked| picked.route), Err(PickError::AllFailed(errors)));
    }

    #[tokio::test(start_paused = true)]
    async fn refuses_a_machine_without_routes_before_reaching_any() {
        let network = Network::default();
        assert_eq!(network.pick(Vec::new()).await.map(|picked| picked.route), Err(PickError::NoRoutes));
        assert_eq!(network.calls(), Vec::<String>::new());
    }

    #[tokio::test(start_paused = true)]
    async fn fails_a_route_without_a_token_before_reaching_it() {
        let network = Network::default().with(LAN, answers_after(0, "{}")).with(TAILNET, answers_after(0, "{}"));
        let routes = vec![bridge(LAN, 8765, None), route(TAILNET)];
        assert_eq!(network.pick(routes).await.map(|picked| picked.route), Ok(route(TAILNET)));
        assert_eq!(network.calls(), [TAILNET]);
        let result = network.pick(vec![bridge(LAN, 8765, None)]).await.map(|picked| picked.route);
        assert_eq!(result, Err(PickError::AllFailed(vec![(address(LAN), Error::NoToken)])));
        assert_eq!(network.calls(), [TAILNET]);
    }

    #[test]
    fn can_run_on_another_task() {
        fn sendable<T: Send>(_: T) {}
        let machine =
            Machine { id: MachineId(0), name: "studio".to_owned(), routes: vec![route(LAN)], last_route: None };
        sendable(pick_route(&machine, |_: &Route| Err(Error::NoToken), PickConfig::default()));
    }

    #[test]
    fn waits_five_seconds_and_150_ms_for_a_better_route_by_default() {
        let expected = PickConfig { deadline: Duration::from_secs(5), window: Duration::from_millis(150) };
        assert_eq!(PickConfig::default(), expected);
    }
}

mod preference {
    use super::*;

    #[rstest]
    #[case::the_lan_alone(vec![(LAN, answers_after(50, "{}"))], LAN, 50)]
    #[case::tailscale_alone_without_the_window(vec![(TAILNET, answers_after(50, "{}"))], TAILNET, 50)]
    #[case::the_lan_answering_within_the_window(
        vec![(TAILNET, answers_after(50, "{}")), (LAN, answers_after(120, "{}"))], LAN, 120
    )]
    #[case::tailscale_at_the_end_of_the_window_when_the_lan_is_slower(
        vec![(TAILNET, answers_after(50, "{}")), (LAN, answers_after(400, "{}"))], TAILNET, 200
    )]
    #[case::tailscale_at_once_when_the_lan_failed_before(
        vec![(TAILNET, answers_after(50, "{}")), (LAN, fails_after(10))], TAILNET, 50
    )]
    #[case::tailscale_as_soon_as_the_lan_fails_within_the_window(
        vec![(TAILNET, answers_after(50, "{}")), (LAN, fails_after(100))], TAILNET, 100
    )]
    #[case::tailscale_over_the_rest_within_the_window(
        vec![(PUBLIC, answers_after(50, "{}")), (TAILNET, answers_after(120, "{}"))], TAILNET, 120
    )]
    #[case::tailscale_over_the_rest_at_the_end_of_the_window_while_the_lan_hangs(
        vec![(PUBLIC, answers_after(50, "{}")), (TAILNET, answers_after(120, "{}")), (LAN, hangs())], TAILNET, 200
    )]
    #[case::the_first_to_answer_of_the_same_kind(
        vec![(TAILNET_V4, answers_after(50, "{}")), (TAILNET, answers_after(80, "{}"))], TAILNET_V4, 50
    )]
    #[case::the_first_of_the_same_kind_while_the_lan_is_pending(
        vec![(TAILNET_V4, answers_after(50, "{}")), (TAILNET, answers_after(80, "{}")), (LAN, hangs())], TAILNET_V4, 200
    )]
    #[case::not_a_worse_kind_answering_later_while_the_lan_is_pending(
        vec![(TAILNET, answers_after(50, "{}")), (PUBLIC, answers_after(80, "{}")), (LAN, hangs())], TAILNET, 200
    )]
    #[case::the_better_kind_over_the_best_so_far_within_the_window(
        vec![(PUBLIC, answers_after(20, "{}")), (TAILNET, answers_after(60, "{}")), (LAN, answers_after(100, "{}"))],
        LAN, 100
    )]
    #[tokio::test(start_paused = true)]
    async fn picks(#[case] answers: Vec<(&'static str, FakeTransport)>, #[case] winner: &str, #[case] millis: u64) {
        let (network, routes) = network_of(answers);
        let picked = races(&network, machine(routes), PickConfig::default()).await;
        assert_eq!(picked, (route(winner), Duration::from_millis(millis)));
    }

    #[tokio::test(start_paused = true)]
    async fn picks_the_lan_over_a_saved_tailscale_route_that_answers_first() {
        let (network, routes) = network_of(vec![(TAILNET, answers_after(20, "{}")), (LAN, answers_after(100, "{}"))]);
        let picked = races(&network, saved(routes, TAILNET), PickConfig::default()).await;
        assert_eq!(picked, (route(LAN), Duration::from_millis(100)));
    }

    #[rstest]
    #[case::by_default(PickConfig::default(), 4900, 5000)]
    #[case::when_the_window_is_longer(
        PickConfig { deadline: Duration::from_secs(1), window: Duration::from_secs(2) }, 500, 1000
    )]
    #[tokio::test(start_paused = true)]
    async fn ends_the_window_at_the_deadline(#[case] config: PickConfig, #[case] tailnet: u64, #[case] millis: u64) {
        let (network, routes) = network_of(vec![(TAILNET, answers_after(tailnet, "{}")), (LAN, hangs())]);
        let picked = races(&network, machine(routes), config).await;
        assert_eq!(picked, (route(TAILNET), Duration::from_millis(millis)));
    }

    #[tokio::test(start_paused = true)]
    async fn lets_go_of_a_route_that_answered_within_the_window_and_lost() {
        let (network, routes) = network_of(vec![
            (TAILNET, answers_after(50, r#"{"from":"tailnet"}"#)),
            (LAN, answers_after(120, r#"{"from":"lan"}"#)),
        ]);
        let picked = network.pick(routes).await.expect("a route answers");
        assert_eq!((picked.route.clone(), picked.contract.clone()), (route(LAN), json!({"from":"lan"})));
        let (_, tailnet) = network.transports.iter().find(|(host, _)| *host == TAILNET).expect("the tailnet transport");
        assert_eq!(Arc::strong_count(tailnet), 1);
    }
}

mod picked {
    use super::*;

    #[tokio::test(start_paused = true)]
    async fn leaves_the_token_out_of_its_debug() {
        let network = Network::default().with(LAN, answers_after(0, r#"{"version":1}"#));
        let picked = network.pick(vec![route(LAN)]).await.expect("the route answers");
        let text = format!("{picked:?}");
        assert!(text.contains(LAN) && text.contains("version"), "{text}");
        assert!(!text.contains(TOKEN), "{text}");
    }
}
