use super::boot::answering_only;
use super::streams::{fine, serving};
use super::*;
use crate::transport;

async fn reached(store: &Store, asked: &Asked) -> String {
    let transport = store.transport().expect("a transport once a route answered");
    let version = transport::output(transport.as_ref(), vec!["version".to_owned()]).await;
    version.expect_err("nothing answers `version`");
    asked.all().pop().expect("the command was asked")
}

async fn until_connected_on(changes: &mut Changes, host: &str) {
    let expected = Connection::Connected { route: route(host).address().clone() };
    bounded(async { while next_connection(changes).await != expected {} }).await;
}

mod the_transport {
    use super::*;

    #[tokio::test(start_paused = true)]
    async fn is_none_before_a_route_answers() {
        let (store, driver) = unreachable();
        assert!(store.transport().is_none());
        settled(&store, driver).await;
        assert!(store.transport().is_none());
    }

    #[tokio::test(start_paused = true)]
    async fn is_the_winning_route_s_once_connected() {
        let asked = Asked::default();
        let (store, mut changes) = spawned(serving(fine()), &asked);
        until_connected(&mut changes).await;
        assert_eq!(reached(&store, &asked).await, format!("{LAN}: version"));
    }

    #[tokio::test(start_paused = true)]
    async fn is_the_new_route_s_after_connecting_again() {
        let asked = Asked::default();
        let lan = answering_only(serving(fine()), true);
        let tailnet = answering_only(serving(fine()), false);
        let (store, driver) = store_on(vec![route(LAN), route(TAILNET)], vec![(LAN, lan), (TAILNET, tailnet)], &asked);
        let mut changes = store.changes();
        tokio::spawn(driver);
        until_connected_on(&mut changes, LAN).await;
        store.resume_now();
        until_connected_on(&mut changes, TAILNET).await;
        assert_eq!(reached(&store, &asked).await, format!("{TAILNET}: version"));
    }
}
