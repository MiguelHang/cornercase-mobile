use rstest::rstest;

use super::*;

const WIFI: Network = Network { kind: Kind::Wifi, expensive: false };
const CELLULAR: Network = Network { kind: Kind::Cellular, expensive: true };
const NONE: Network = Network { kind: Kind::None, expensive: false };

async fn within(debounce: &mut Debounce, wait: Duration) -> Option<Decision> {
    time::timeout(wait, debounce.decided()).await.ok()
}

async fn settled(baseline: Network) -> Debounce {
    let mut debounce = Debounce::default();
    debounce.observe(baseline);
    let _ = within(&mut debounce, Duration::from_secs(60)).await;
    debounce
}

async fn burst(debounce: &mut Debounce, networks: &[Network]) {
    for network in networks {
        debounce.observe(*network);
        time::sleep(Duration::from_millis(300)).await;
    }
}

mod baseline {
    use super::*;

    #[tokio::test(start_paused = true)]
    async fn decides_nothing_when_the_phone_starts_on_a_network() {
        let mut debounce = Debounce::default();
        debounce.observe(WIFI);
        assert_eq!(within(&mut debounce, Duration::from_secs(60)).await, None);
        assert!(!debounce.is_down());
    }

    #[tokio::test(start_paused = true)]
    async fn goes_offline_after_a_second_when_the_phone_starts_without_network() {
        let mut debounce = Debounce::default();
        debounce.observe(NONE);
        let started = Instant::now();
        assert_eq!(debounce.decided().await, Decision::Offline);
        assert_eq!(started.elapsed(), Duration::from_secs(1));
        assert!(debounce.is_down());
    }

    #[tokio::test(start_paused = true)]
    async fn waits_for_a_first_value_without_being_down() {
        let mut debounce = Debounce::default();
        assert_eq!(within(&mut debounce, Duration::from_secs(60)).await, None);
        assert!(!debounce.is_down());
    }
}

mod changes {
    use super::*;

    #[tokio::test(start_paused = true)]
    async fn give_one_decision_for_a_burst_with_the_last_value() {
        let mut debounce = settled(WIFI).await;
        burst(&mut debounce, &[CELLULAR, Network { kind: Kind::Other, expensive: false }]).await;
        debounce.observe(Network { kind: Kind::Other, expensive: true });
        let started = Instant::now();
        let decision = debounce.decided().await;
        assert_eq!(decision, Decision::Changed(Network { kind: Kind::Other, expensive: true }));
        assert_eq!(started.elapsed(), Duration::from_secs(1));
        assert_eq!(within(&mut debounce, Duration::from_secs(60)).await, None);
    }

    #[tokio::test(start_paused = true)]
    async fn come_back_online_after_going_offline() {
        let mut debounce = settled(WIFI).await;
        debounce.observe(NONE);
        assert_eq!(debounce.decided().await, Decision::Offline);
        assert!(debounce.is_down());
        debounce.observe(WIFI);
        assert_eq!(debounce.decided().await, Decision::Changed(WIFI));
        assert!(!debounce.is_down());
    }

    #[tokio::test(start_paused = true)]
    async fn come_back_online_after_starting_without_network() {
        let mut debounce = settled(NONE).await;
        debounce.observe(CELLULAR);
        assert_eq!(debounce.decided().await, Decision::Changed(CELLULAR));
    }

    #[tokio::test(start_paused = true)]
    async fn skip_offline_when_the_network_is_back_within_the_second() {
        let mut debounce = settled(WIFI).await;
        burst(&mut debounce, &[NONE]).await;
        debounce.observe(CELLULAR);
        assert_eq!(debounce.decided().await, Decision::Changed(CELLULAR));
        assert_eq!(within(&mut debounce, Duration::from_secs(60)).await, None);
        assert!(!debounce.is_down());
    }

    #[tokio::test(start_paused = true)]
    async fn give_nothing_for_the_value_already_applied() {
        let mut debounce = settled(WIFI).await;
        burst(&mut debounce, &[NONE, CELLULAR]).await;
        debounce.observe(WIFI);
        assert_eq!(within(&mut debounce, Duration::from_secs(60)).await, None);
    }

    #[tokio::test(start_paused = true)]
    async fn count_a_change_of_cost_on_the_same_interface() {
        let mut debounce = settled(WIFI).await;
        debounce.observe(Network { kind: Kind::Wifi, expensive: true });
        assert_eq!(debounce.decided().await, Decision::Changed(Network { kind: Kind::Wifi, expensive: true }));
    }

    #[tokio::test(start_paused = true)]
    async fn give_offline_once_while_the_network_stays_down() {
        let mut debounce = settled(WIFI).await;
        debounce.observe(NONE);
        assert_eq!(debounce.decided().await, Decision::Offline);
        debounce.observe(NONE);
        assert_eq!(within(&mut debounce, Duration::from_secs(60)).await, None);
    }
}

mod waiting {
    use super::*;

    #[tokio::test(start_paused = true)]
    async fn takes_the_quiet_time_it_is_given() {
        let mut debounce = Debounce::new(Duration::from_millis(200));
        debounce.observe(NONE);
        let started = Instant::now();
        assert_eq!(debounce.decided().await, Decision::Offline);
        assert_eq!(started.elapsed(), Duration::from_millis(200));
    }

    #[tokio::test(start_paused = true)]
    async fn keeps_the_pending_value_when_the_wait_is_dropped() {
        let mut debounce = settled(WIFI).await;
        debounce.observe(NONE);
        assert_eq!(within(&mut debounce, Duration::from_millis(500)).await, None);
        let started = Instant::now();
        assert_eq!(debounce.decided().await, Decision::Offline);
        assert_eq!(started.elapsed(), Duration::from_millis(500));
    }
}

mod payload {
    use super::*;

    #[rstest]
    #[case::wifi(r#"{"kind":"wifi","expensive":false}"#, WIFI)]
    #[case::cellular(r#"{"kind":"cellular","expensive":true}"#, CELLULAR)]
    #[case::other(r#"{"kind":"other","expensive":false}"#, Network { kind: Kind::Other, expensive: false })]
    #[case::none(r#"{"kind":"none","expensive":false}"#, NONE)]
    fn reads_the_kind_in_lower_case(#[case] json: &str, #[case] network: Network) {
        assert_eq!(serde_json::from_str::<Network>(json).expect("a network"), network);
    }
}
