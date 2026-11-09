use super::*;

fn secrets_with(key: &str, value: &[u8]) -> MemorySecrets {
    let secrets = MemorySecrets::default();
    secrets.set(key, value).expect("set the secret");
    secrets
}

mod get {
    use super::*;

    #[test]
    fn finds_nothing_under_an_unknown_key() {
        let secrets = secrets_with("route/1/a", b"one");
        assert_eq!(secrets.get("route/1/b"), Ok(None));
    }

    #[test]
    fn reads_what_was_set() {
        let secrets = secrets_with("route/1/a", b"one");
        assert_eq!(secrets.get("route/1/a"), Ok(Some(b"one".to_vec())));
    }
}

mod set {
    use super::*;

    #[test]
    fn replaces_the_value_under_the_same_key() {
        let secrets = secrets_with("route/1/a", b"one");
        secrets.set("route/1/a", b"two").expect("replace the secret");
        assert_eq!(secrets.get("route/1/a"), Ok(Some(b"two".to_vec())));
    }

    #[test]
    fn fails_and_keeps_the_old_value_while_writes_fail() {
        let secrets = secrets_with("route/1/a", b"one");
        secrets.fail_writes(true);
        assert!(matches!(secrets.set("route/1/a", b"two"), Err(Error::Secrets(_))));
        assert_eq!(secrets.get("route/1/a"), Ok(Some(b"one".to_vec())));
    }

    #[test]
    fn works_again_once_writes_stop_failing() {
        let secrets = secrets_with("route/1/a", b"one");
        secrets.fail_writes(true);
        secrets.fail_writes(false);
        secrets.set("route/1/a", b"two").expect("replace the secret");
        assert_eq!(secrets.get("route/1/a"), Ok(Some(b"two".to_vec())));
    }
}

mod delete {
    use super::*;

    #[test]
    fn removes_the_value() {
        let secrets = secrets_with("route/1/a", b"one");
        secrets.delete("route/1/a").expect("delete the secret");
        assert_eq!(secrets.get("route/1/a"), Ok(None));
    }

    #[test]
    fn accepts_an_unknown_key() {
        let secrets = MemorySecrets::default();
        assert_eq!(secrets.delete("route/1/a"), Ok(()));
    }

    #[test]
    fn fails_and_keeps_the_value_while_writes_fail() {
        let secrets = secrets_with("route/1/a", b"one");
        secrets.fail_writes(true);
        assert!(matches!(secrets.delete("route/1/a"), Err(Error::Secrets(_))));
        assert_eq!(secrets.get("route/1/a"), Ok(Some(b"one".to_vec())));
    }
}
