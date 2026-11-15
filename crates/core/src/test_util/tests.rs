use super::*;

mod temp_dir {
    use super::*;

    #[test]
    fn removes_its_folder_and_what_is_in_it_when_dropped() {
        let dir = TempDir::new();
        let path = dir.path().to_path_buf();
        std::fs::write(path.join("file"), "x").expect("write a file");
        assert!(path.exists());
        drop(dir);
        assert!(!path.exists());
    }

    #[test]
    fn gives_each_dir_its_own_path() {
        let (a, b) = (TempDir::new(), TempDir::new());
        assert_ne!(a.path(), b.path());
    }
}
