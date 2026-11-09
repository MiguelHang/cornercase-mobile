use std::fs;
use std::path::PathBuf;

use cornercase_bridge::test_util::TempDir;

use super::*;
use crate::machines::store::JsonFileStore;
use crate::machines::tests::{TOKEN, bridge};
use crate::secrets::memory::MemorySecrets;

const NEW_TOKEN: &str = "bmV3LXRva2Vu";
const LAN: &str = "192.168.1.20";
const TAILNET: &str = "fd7a:115c:a1e0::1";

struct Phone {
    dir: TempDir,
    secrets: Arc<MemorySecrets>,
}

impl Phone {
    fn new() -> Self {
        Self { dir: TempDir::new(), secrets: Arc::default() }
    }

    fn file(&self) -> PathBuf {
        self.dir.path().join("machines.json")
    }

    fn open(&self) -> Machines {
        Machines::load(Box::new(JsonFileStore::new(self.file())), self.secrets.clone()).expect("load the machines")
    }

    fn text(&self) -> String {
        fs::read_to_string(self.file()).unwrap_or_default()
    }

    fn secret(&self, id: MachineId, host: &str) -> Option<String> {
        let value = self.secrets.get(&key(id, &address(host))).expect("read the secret");
        value.map(|bytes| String::from_utf8(bytes).expect("a text secret"))
    }

    fn studio(&self) -> (Machines, MachineId) {
        let mut machines = self.open();
        let id = machines.add("studio".to_owned(), route(LAN, TOKEN)).expect("add the studio");
        machines.add_route(id, route(TAILNET, TOKEN)).expect("add the tailnet route");
        (machines, id)
    }
}

fn route(host: &str, token: &str) -> Route {
    bridge(host, 8765, Some(token))
}

fn address(host: &str) -> Address {
    Address { host: host.to_owned(), port: 8765 }
}

fn unknown_machine() -> Result<(), Error> {
    Err(Error::Store("no saved machine has that id".to_owned()))
}

mod load {
    use super::*;

    #[test]
    fn brings_the_machines_back_with_their_tokens() {
        let phone = Phone::new();
        let (machines, id) = phone.studio();
        let expected = Machine {
            id,
            name: "studio".to_owned(),
            routes: vec![route(LAN, TOKEN), route(TAILNET, TOKEN)],
            last_route: None,
        };
        assert_eq!(machines.machines(), std::slice::from_ref(&expected));
        assert_eq!(phone.open().machines(), [expected]);
        assert!(!phone.text().contains(TOKEN), "{}", phone.text());
    }

    #[test]
    fn keeps_a_route_whose_token_is_missing_without_a_token() {
        let phone = Phone::new();
        let (_, id) = phone.studio();
        phone.secrets.delete(&key(id, &address(LAN))).expect("forget the token");
        let routes = phone.open().machine(id).expect("the studio").routes.clone();
        assert_eq!(routes, [bridge(LAN, 8765, None), route(TAILNET, TOKEN)]);
    }
}

mod add {
    use super::*;

    #[test]
    fn keeps_the_token_under_the_machine_id_and_the_address() {
        let phone = Phone::new();
        let id = phone.open().add("studio".to_owned(), route(LAN, TOKEN)).expect("add");
        let key = format!("route/{}/ws://{LAN}:8765/", id.0);
        assert_eq!(phone.secrets.get(&key).expect("read the secret"), Some(TOKEN.as_bytes().to_vec()));
    }

    #[test]
    fn never_gives_the_id_of_a_removed_machine_again() {
        let phone = Phone::new();
        let (mut machines, first) = phone.studio();
        machines.remove(first).expect("remove the studio");
        let second = phone.open().add("laptop".to_owned(), route(LAN, NEW_TOKEN)).expect("add the laptop");
        assert_eq!((first, second), (MachineId(0), MachineId(1)));
    }

    #[test]
    fn leaves_the_file_as_it_was_when_the_token_cannot_be_saved() {
        let phone = Phone::new();
        let (mut machines, _) = phone.studio();
        let before = phone.text();
        phone.secrets.fail_writes(true);
        let result = machines.add("laptop".to_owned(), route(LAN, NEW_TOKEN));
        assert!(matches!(result, Err(Error::Secrets(_))), "{result:?}");
        assert_eq!(phone.text(), before);
        assert_eq!(machines.machines().len(), 1);
    }

    #[test]
    fn refuses_a_route_without_a_token() {
        let phone = Phone::new();
        let mut machines = phone.open();
        assert_eq!(machines.add("studio".to_owned(), bridge(LAN, 8765, None)), Err(Error::NoToken));
        assert!(!phone.file().exists());
    }
}

mod add_route {
    use super::*;

    #[test]
    fn replaces_the_token_of_the_route_at_the_same_address() {
        let phone = Phone::new();
        let (mut machines, id) = phone.studio();
        machines.add_route(id, route(LAN, NEW_TOKEN)).expect("scan the code again");
        let expected = [route(LAN, NEW_TOKEN), route(TAILNET, TOKEN)];
        assert_eq!(machines.machine(id).expect("the studio").routes, expected);
        assert_eq!(phone.open().machine(id).expect("the studio").routes, expected);
        assert_eq!(phone.secret(id, LAN).as_deref(), Some(NEW_TOKEN));
    }

    #[test]
    fn refuses_an_unknown_machine_without_saving_its_token() {
        let phone = Phone::new();
        let (mut machines, id) = phone.studio();
        let other = MachineId(id.0 + 1);
        assert_eq!(machines.add_route(other, route(LAN, NEW_TOKEN)), unknown_machine());
        assert_eq!(phone.secret(other, LAN), None);
    }
}

mod remove {
    use super::*;

    #[test]
    fn forgets_the_machine_and_deletes_its_tokens() {
        let phone = Phone::new();
        let (mut machines, id) = phone.studio();
        machines.remove(id).expect("remove the studio");
        assert_eq!(machines.machines(), []);
        assert_eq!(phone.open().machines(), []);
        assert_eq!((phone.secret(id, LAN), phone.secret(id, TAILNET)), (None, None));
    }

    #[test]
    fn writes_the_file_before_deleting_the_tokens() {
        let phone = Phone::new();
        let (mut machines, id) = phone.studio();
        phone.secrets.fail_writes(true);
        assert!(matches!(machines.remove(id), Err(Error::Secrets(_))));
        assert_eq!(phone.open().machines(), []);
        assert_eq!(phone.secret(id, LAN).as_deref(), Some(TOKEN));
    }

    #[test]
    fn does_nothing_for_an_unknown_machine() {
        let phone = Phone::new();
        let (mut machines, id) = phone.studio();
        let before = phone.text();
        machines.remove(MachineId(id.0 + 1)).expect("remove nothing");
        assert_eq!(phone.text(), before);
    }
}

mod remember {
    use super::*;

    #[test]
    fn keeps_the_winning_route_after_loading_again() {
        let phone = Phone::new();
        let (mut machines, id) = phone.studio();
        machines.remember(id, &address(TAILNET)).expect("remember the tailnet route");
        assert_eq!(phone.open().machine(id).expect("the studio").last_route, Some(address(TAILNET)));
    }

    #[test]
    fn refuses_an_address_the_machine_has_no_route_to() {
        let phone = Phone::new();
        let (mut machines, id) = phone.studio();
        let message = "the machine has no route at that address".to_owned();
        assert_eq!(machines.remember(id, &address("10.0.0.9")), Err(Error::Store(message)));
        assert_eq!(phone.open().machine(id).expect("the studio").last_route, None);
    }

    #[test]
    fn refuses_an_unknown_machine() {
        let phone = Phone::new();
        let (mut machines, id) = phone.studio();
        assert_eq!(machines.remember(MachineId(id.0 + 1), &address(LAN)), unknown_machine());
    }
}

mod debug {
    use super::*;

    #[test]
    fn leaves_the_tokens_out() {
        let phone = Phone::new();
        let (machines, _) = phone.studio();
        let text = format!("{machines:?}");
        assert!(text.contains("studio"), "{text}");
        assert!(!text.contains(TOKEN), "{text}");
    }
}
