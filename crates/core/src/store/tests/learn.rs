use super::boot::answering_only;
use super::connect::contract;
use super::resume::contracts;
use super::streams::{connections, fine, serving};
use super::transport::until_connected_on;
use super::*;
use crate::cli::contract::fixtures::REAL;
use crate::machines::route::Token;

fn with_token(host: &str, token: &str) -> Route {
    let mut route = route(host);
    route.set_token(Some(Token::new(token.to_owned())));
    route
}

fn lan_then_tailnet(asked: &Asked) -> (Store, Changes) {
    let hosts = vec![(LAN, answering_only(serving(fine()), true)), (TAILNET, serving(fine()))];
    let (store, driver) = store_on(vec![route(LAN)], hosts, asked);
    let changes = store.changes();
    tokio::spawn(driver);
    (store, changes)
}

mod a_learned_route {
    use super::*;

    #[tokio::test(start_paused = true)]
    async fn leaves_the_connection_as_it_is() {
        let asked = Asked::default();
        let (store, mut changes) = lan_then_tailnet(&asked);
        until_connected_on(&mut changes, LAN).await;
        store.learn(&[route(TAILNET)]);
        time::sleep(Duration::from_secs(60)).await;
        assert_eq!(connections(&unread(&mut changes)), []);
        assert_eq!(store.connection(), Connection::Connected { route: route(LAN).address().clone() });
        assert_eq!(contracts(&asked), 1);
    }

    #[tokio::test(start_paused = true)]
    async fn enters_the_race_once_the_network_changes() {
        let (store, mut changes) = lan_then_tailnet(&Asked::default());
        until_connected_on(&mut changes, LAN).await;
        store.learn(&[route(TAILNET)]);
        store.network_changed();
        until_connected_on(&mut changes, TAILNET).await;
    }

    #[tokio::test(start_paused = true)]
    async fn is_added_once_and_never_changes_a_saved_one() {
        let shared = Arc::new(Shared::default());
        let factory = network(vec![(LAN, contract(REAL))], &Asked::default());
        let (mut driver, _worker) =
            Driver::new(machine(vec![route(LAN)]), factory, PickConfig::default(), shared.clone());
        let store = Store { shared, resumes: mpsc::unbounded_channel().0 };
        store.learn(&[with_token(LAN, "b3RoZXI"), route(TAILNET)]);
        store.learn(&[route(TAILNET)]);
        driver.connect().await.expect("the lan answers");
        assert_eq!(driver.machine.routes, [route(LAN), route(TAILNET)]);
        assert_eq!(store.shared.learned(), []);
    }
}
