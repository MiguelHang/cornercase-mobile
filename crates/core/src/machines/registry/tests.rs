use std::fs;
use std::path::PathBuf;

use cornercase_bridge::test_util::TempDir;
use rstest::rstest;

use super::*;
use crate::machines::store::JsonFileStore;
use crate::machines::tests::{TOKEN, bridge};
use crate::secrets::memory::MemorySecrets;

const NEW_TOKEN: &str = "bmV3LXRva2Vu";
const LAN: &str = "192.168.1.20";
const TAILNET: &str = "fd7a:115c:a1e0::1";
const GONE: &str = "10.0.0.9";

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
        let id = machines.add("studio", route(LAN, TOKEN)).expect("add the studio");
        machines.add_route(id, route(TAILNET, TOKEN)).expect("add the tailnet route");
        (machines, id)
    }

    fn studio_and_laptop(&self) -> (Machines, MachineId, MachineId) {
        let (mut machines, studio) = self.studio();
        let laptop = machines.add("laptop", route(GONE, NEW_TOKEN)).expect("add the laptop");
        (machines, studio, laptop)
    }

    fn block_the_file(&self) {
        fs::remove_file(self.file()).ok();
        fs::create_dir(self.file()).expect("put a folder in the file's place");
        fs::write(self.file().join("inside"), "kept").expect("fill the folder");
    }

    fn unblock_the_file(&self) {
        fs::remove_dir_all(self.file()).expect("take the folder away");
    }

    fn unchanged<T>(&self, machines: &mut Machines, change: impl FnOnce(&mut Machines) -> T) -> T {
        let (text, before) = (self.text(), machines.machines().to_vec());
        let result = change(machines);
        assert_eq!((self.text(), machines.machines()), (text, before.as_slice()));
        result
    }
}

fn route(host: &str, token: &str) -> Route {
    bridge(host, 8765, Some(token))
}

fn address(host: &str) -> Address {
    Address { host: host.to_owned(), port: 8765 }
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

    #[test]
    fn never_gives_a_saved_id_again_when_the_counter_is_behind() {
        let phone = Phone::new();
        let machine = r#"{"id": 3, "name": "studio", "routes": [], "last_route": null}"#;
        fs::write(phone.file(), format!(r#"{{"version": 1, "next_id": 0, "machines": [{machine}]}}"#))
            .expect("write the file by hand");
        let id = phone.open().add("laptop", route(LAN, TOKEN)).expect("add the laptop");
        assert_eq!(id, MachineId(4));
    }
}

mod add {
    use super::*;

    #[test]
    fn keeps_the_token_under_the_machine_id_and_the_address() {
        let phone = Phone::new();
        let id = phone.open().add("studio", route(LAN, TOKEN)).expect("add");
        let key = format!("route/{}/ws://{LAN}:8765/", id.0);
        assert_eq!(phone.secrets.get(&key).expect("read the secret"), Some(TOKEN.as_bytes().to_vec()));
    }

    #[test]
    fn never_gives_the_id_of_a_removed_machine_again() {
        let phone = Phone::new();
        let (mut machines, first) = phone.studio();
        machines.remove(first).expect("remove the studio");
        let second = phone.open().add("laptop", route(LAN, NEW_TOKEN)).expect("add the laptop");
        assert_eq!((first, second), (MachineId(0), MachineId(1)));
    }

    #[test]
    fn leaves_the_file_as_it_was_when_the_token_cannot_be_saved() {
        let phone = Phone::new();
        let (mut machines, _) = phone.studio();
        phone.secrets.fail_writes(true);
        let result = phone.unchanged(&mut machines, |machines| machines.add("laptop", route(LAN, NEW_TOKEN)));
        assert!(matches!(result, Err(Error::Secrets(_))), "{result:?}");
    }

    #[test]
    fn keeps_the_list_and_the_id_when_the_file_cannot_be_written() {
        let phone = Phone::new();
        let (mut machines, studio) = phone.studio();
        phone.block_the_file();
        let result = phone.unchanged(&mut machines, |machines| machines.add("laptop", route(LAN, NEW_TOKEN)));
        assert!(matches!(result, Err(Error::Store(_))), "{result:?}");
        phone.unblock_the_file();
        let laptop = machines.add("laptop", route(LAN, NEW_TOKEN)).expect("add the laptop");
        assert_eq!(laptop, MachineId(studio.0 + 1));
    }

    #[test]
    fn refuses_a_route_without_a_token() {
        let phone = Phone::new();
        let mut machines = phone.open();
        assert_eq!(machines.add("studio", bridge(LAN, 8765, None)), Err(Error::NoToken));
        assert!(!phone.file().exists());
    }

    #[test]
    fn keeps_the_name_trimmed() {
        let phone = Phone::new();
        let id = phone.open().add("  studio ", route(LAN, TOKEN)).expect("add");
        assert_eq!(phone.open().machine(id).expect("the studio").name, "studio");
    }

    #[test]
    fn refuses_a_name_another_machine_has_without_saving_its_token() {
        let phone = Phone::new();
        let (mut machines, studio) = phone.studio();
        let result = phone.unchanged(&mut machines, |machines| machines.add(" Studio", route(LAN, NEW_TOKEN)));
        assert_eq!(result, Err(Error::NameTaken));
        assert_eq!(phone.secret(MachineId(studio.0 + 1), LAN), None);
    }
}

mod rename {
    use super::*;

    #[test]
    fn keeps_the_new_name_trimmed_after_loading_again() {
        let phone = Phone::new();
        let (mut machines, id) = phone.studio();
        machines.rename(id, "  Mac mini ").expect("rename the studio");
        assert_eq!(machines.machine(id).expect("the studio").name, "Mac mini");
        assert_eq!(phone.open().machine(id).expect("the studio").name, "Mac mini");
    }

    #[test]
    fn lets_the_machine_change_the_case_of_its_own_name() {
        let phone = Phone::new();
        let (mut machines, id) = phone.studio();
        machines.rename(id, "Studio").expect("rename the studio");
        assert_eq!(phone.open().machine(id).expect("the studio").name, "Studio");
    }

    #[test]
    fn writes_nothing_when_the_name_does_not_change() {
        let phone = Phone::new();
        let (mut machines, id) = phone.studio();
        phone.block_the_file();
        assert_eq!(machines.rename(id, " studio "), Ok(()));
    }

    #[rstest]
    #[case::empty("  ", Error::NameEmpty)]
    #[case::too_long(&"m".repeat(65), Error::NameTooLong)]
    #[case::taken_in_other_case_and_with_spaces(" LAPTOP  ", Error::NameTaken)]
    fn refuses_a_name_and_keeps_the_old_one(#[case] name: &str, #[case] error: Error) {
        let phone = Phone::new();
        let (mut machines, id) = phone.studio();
        machines.add("laptop", route(GONE, NEW_TOKEN)).expect("add the laptop");
        let result = phone.unchanged(&mut machines, |machines| machines.rename(id, name));
        assert_eq!(result, Err(error));
    }

    #[test]
    fn refuses_an_unknown_machine() {
        let phone = Phone::new();
        let (mut machines, id) = phone.studio();
        let result = phone.unchanged(&mut machines, |machines| machines.rename(MachineId(id.0 + 1), "laptop"));
        assert_eq!(result, Err(Error::UnknownMachine));
    }

    #[test]
    fn keeps_the_list_when_the_file_cannot_be_written() {
        let phone = Phone::new();
        let (mut machines, id) = phone.studio();
        phone.block_the_file();
        assert!(matches!(machines.rename(id, "Mac mini"), Err(Error::Store(_))));
        assert_eq!(machines.machine(id).expect("the studio").name, "studio");
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
        assert_eq!(machines.add_route(other, route(LAN, NEW_TOKEN)), Err(Error::UnknownMachine));
        assert_eq!(phone.secret(other, LAN), None);
    }

    #[test]
    fn leaves_the_file_as_it_was_when_the_token_cannot_be_saved() {
        let phone = Phone::new();
        let (mut machines, id) = phone.studio();
        phone.secrets.fail_writes(true);
        let result = phone.unchanged(&mut machines, |machines| machines.add_route(id, route(LAN, NEW_TOKEN)));
        assert!(matches!(result, Err(Error::Secrets(_))), "{result:?}");
        assert_eq!(phone.secret(id, LAN).as_deref(), Some(TOKEN));
    }

    #[test]
    fn refuses_a_route_without_a_token() {
        let phone = Phone::new();
        let (mut machines, id) = phone.studio();
        let result = phone.unchanged(&mut machines, |machines| machines.add_route(id, bridge(LAN, 8765, None)));
        assert_eq!(result, Err(Error::NoToken));
        assert_eq!(phone.secret(id, LAN).as_deref(), Some(TOKEN));
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

mod remove_route {
    use super::*;

    #[test]
    fn deletes_its_token_and_keeps_the_other_routes_after_loading_again() {
        let phone = Phone::new();
        let (mut machines, id) = phone.studio();
        machines.remove_route(id, &address(LAN)).expect("remove the lan route");
        assert_eq!(machines.machine(id).expect("the studio").routes, [route(TAILNET, TOKEN)]);
        assert_eq!(phone.open().machine(id).expect("the studio").routes, [route(TAILNET, TOKEN)]);
        assert_eq!((phone.secret(id, LAN), phone.secret(id, TAILNET).as_deref()), (None, Some(TOKEN)));
    }

    #[rstest]
    #[case::the_removed_one(TAILNET, None)]
    #[case::another_one(LAN, Some(address(TAILNET)))]
    fn forgets_the_last_winner_only_when_it_is_removed(#[case] removed: &str, #[case] last: Option<Address>) {
        let phone = Phone::new();
        let (mut machines, id) = phone.studio();
        machines.remember(id, &address(TAILNET)).expect("remember the tailnet route");
        machines.remove_route(id, &address(removed)).expect("remove a route");
        assert_eq!(phone.open().machine(id).expect("the studio").last_route, last);
    }

    #[test]
    fn refuses_the_last_route_and_keeps_its_token() {
        let phone = Phone::new();
        let (mut machines, id) = phone.studio();
        machines.remove_route(id, &address(LAN)).expect("remove the lan route");
        let result = phone.unchanged(&mut machines, |machines| machines.remove_route(id, &address(TAILNET)));
        assert_eq!(result, Err(Error::LastRoute));
        assert_eq!(phone.secret(id, TAILNET).as_deref(), Some(TOKEN));
    }

    #[test]
    fn refuses_an_address_the_machine_has_no_route_to() {
        let phone = Phone::new();
        let (mut machines, id) = phone.studio();
        let result = phone.unchanged(&mut machines, |machines| machines.remove_route(id, &address(GONE)));
        assert_eq!(result, Err(Error::UnknownRoute));
    }

    #[test]
    fn refuses_an_unknown_machine() {
        let phone = Phone::new();
        let (mut machines, id) = phone.studio();
        let other = MachineId(id.0 + 1);
        let result = phone.unchanged(&mut machines, |machines| machines.remove_route(other, &address(LAN)));
        assert_eq!(result, Err(Error::UnknownMachine));
    }

    #[test]
    fn writes_the_file_before_deleting_the_token() {
        let phone = Phone::new();
        let (mut machines, id) = phone.studio();
        phone.secrets.fail_writes(true);
        assert!(matches!(machines.remove_route(id, &address(LAN)), Err(Error::Secrets(_))));
        assert_eq!(phone.open().machine(id).expect("the studio").routes, [route(TAILNET, TOKEN)]);
        assert_eq!(phone.secret(id, LAN).as_deref(), Some(TOKEN));
    }

    #[test]
    fn keeps_the_route_when_the_file_cannot_be_written() {
        let phone = Phone::new();
        let (mut machines, id) = phone.studio();
        phone.block_the_file();
        assert!(matches!(machines.remove_route(id, &address(LAN)), Err(Error::Store(_))));
        assert_eq!(machines.machine(id).expect("the studio").routes.len(), 2);
        assert_eq!(phone.secret(id, LAN).as_deref(), Some(TOKEN));
    }
}

mod move_route {
    use super::*;

    fn three_routes(phone: &Phone) -> (Machines, MachineId) {
        let (mut machines, id) = phone.studio();
        machines.add_route(id, route(GONE, TOKEN)).expect("add a third route");
        (machines, id)
    }

    fn hosts(machines: &Machines, id: MachineId) -> Vec<String> {
        let routes = &machines.machine(id).expect("the studio").routes;
        routes.iter().map(|route| route.address().host.clone()).collect()
    }

    #[rstest]
    #[case::up(TAILNET, Direction::Up, [TAILNET, LAN, GONE])]
    #[case::down(TAILNET, Direction::Down, [LAN, GONE, TAILNET])]
    fn swaps_the_route_with_its_neighbour_and_keeps_the_order(
        #[case] moved: &str,
        #[case] direction: Direction,
        #[case] order: [&str; 3],
    ) {
        let phone = Phone::new();
        let (mut machines, id) = three_routes(&phone);
        machines.move_route(id, &address(moved), direction).expect("move the route");
        assert_eq!(hosts(&machines, id), order);
        assert_eq!(hosts(&phone.open(), id), order);
    }

    #[rstest]
    #[case::the_first_up(LAN, Direction::Up)]
    #[case::the_last_down(GONE, Direction::Down)]
    fn writes_nothing_at_the_ends(#[case] moved: &str, #[case] direction: Direction) {
        let phone = Phone::new();
        let (mut machines, id) = three_routes(&phone);
        phone.block_the_file();
        assert_eq!(machines.move_route(id, &address(moved), direction), Ok(()));
        assert_eq!(hosts(&machines, id), [LAN, TAILNET, GONE]);
    }

    #[test]
    fn refuses_an_address_the_machine_has_no_route_to() {
        let phone = Phone::new();
        let (mut machines, id) = phone.studio();
        let result = phone.unchanged(&mut machines, |machines| machines.move_route(id, &address(GONE), Direction::Up));
        assert_eq!(result, Err(Error::UnknownRoute));
    }

    #[test]
    fn refuses_an_unknown_machine() {
        let phone = Phone::new();
        let (mut machines, id) = phone.studio();
        let other = MachineId(id.0 + 1);
        let result =
            phone.unchanged(&mut machines, |machines| machines.move_route(other, &address(LAN), Direction::Down));
        assert_eq!(result, Err(Error::UnknownMachine));
    }

    #[test]
    fn keeps_the_order_when_the_file_cannot_be_written() {
        let phone = Phone::new();
        let (mut machines, id) = three_routes(&phone);
        phone.block_the_file();
        assert!(matches!(machines.move_route(id, &address(LAN), Direction::Down), Err(Error::Store(_))));
        assert_eq!(hosts(&machines, id), [LAN, TAILNET, GONE]);
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
        assert_eq!(machines.remember(id, &address(GONE)), Err(Error::UnknownRoute));
        assert_eq!(phone.open().machine(id).expect("the studio").last_route, None);
    }

    #[test]
    fn refuses_an_unknown_machine() {
        let phone = Phone::new();
        let (mut machines, id) = phone.studio();
        assert_eq!(machines.remember(MachineId(id.0 + 1), &address(LAN)), Err(Error::UnknownMachine));
    }

    #[test]
    fn writes_nothing_when_the_route_is_already_the_last_winner() {
        let phone = Phone::new();
        let (mut machines, id) = phone.studio();
        machines.remember(id, &address(TAILNET)).expect("remember the tailnet route");
        phone.block_the_file();
        assert_eq!(machines.remember(id, &address(TAILNET)), Ok(()));
    }
}

mod activate {
    use super::*;

    #[test]
    fn keeps_the_active_machine_after_loading_again() {
        let phone = Phone::new();
        let (mut machines, _, laptop) = phone.studio_and_laptop();
        machines.activate(laptop).expect("activate the laptop");
        assert_eq!((machines.active(), phone.open().active()), (Some(laptop), Some(laptop)));
    }

    #[test]
    fn refuses_an_unknown_machine() {
        let phone = Phone::new();
        let (mut machines, _, laptop) = phone.studio_and_laptop();
        let result = phone.unchanged(&mut machines, |machines| machines.activate(MachineId(laptop.0 + 1)));
        assert_eq!(result, Err(Error::UnknownMachine));
    }

    #[test]
    fn writes_nothing_when_the_machine_is_already_active() {
        let phone = Phone::new();
        let (mut machines, _, laptop) = phone.studio_and_laptop();
        machines.activate(laptop).expect("activate the laptop");
        phone.block_the_file();
        assert_eq!(machines.activate(laptop), Ok(()));
    }

    #[test]
    fn keeps_the_previous_one_when_the_file_cannot_be_written() {
        let phone = Phone::new();
        let (mut machines, studio, laptop) = phone.studio_and_laptop();
        phone.block_the_file();
        assert!(matches!(machines.activate(laptop), Err(Error::Store(_))));
        assert_eq!(machines.active(), Some(studio));
    }
}

mod active {
    use super::*;

    #[test]
    fn is_the_first_machine_until_one_is_activated() {
        let phone = Phone::new();
        let (machines, studio, _) = phone.studio_and_laptop();
        assert_eq!(machines.active(), Some(studio));
    }

    #[test]
    fn is_none_without_machines() {
        assert_eq!(Phone::new().open().active(), None);
    }

    #[test]
    fn falls_back_to_the_first_and_is_forgotten_when_the_active_one_is_removed() {
        let phone = Phone::new();
        let (mut machines, studio, laptop) = phone.studio_and_laptop();
        machines.activate(laptop).expect("activate the laptop");
        machines.remove(laptop).expect("remove the laptop");
        assert_eq!((machines.active(), phone.open().active()), (Some(studio), Some(studio)));
        let written: serde_json::Value = serde_json::from_str(&phone.text()).expect("the file is JSON");
        assert_eq!(written["active"], serde_json::Value::Null);
    }

    #[test]
    fn stays_when_another_machine_is_removed() {
        let phone = Phone::new();
        let (mut machines, studio, laptop) = phone.studio_and_laptop();
        machines.activate(laptop).expect("activate the laptop");
        machines.remove(studio).expect("remove the studio");
        assert_eq!(phone.open().active(), Some(laptop));
    }

    #[test]
    fn falls_back_to_the_first_when_the_saved_one_is_not_in_the_list() {
        let phone = Phone::new();
        let machine = r#"{"id": 3, "name": "studio", "routes": [], "last_route": null}"#;
        fs::write(phone.file(), format!(r#"{{"version": 1, "next_id": 4, "machines": [{machine}], "active": 7}}"#))
            .expect("write the file by hand");
        assert_eq!(phone.open().active(), Some(MachineId(3)));
    }
}

mod same {
    use super::*;
    use crate::machines::same::Why;

    #[test]
    fn compares_the_tokens_read_back_from_the_secrets() {
        let phone = Phone::new();
        let (_, id) = phone.studio();
        let same = phone.open().same(&route("studio.local", TOKEN), "new", None, &HashMap::new());
        assert_eq!(same, Some(Same { machine: id, why: Why::Token }));
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
