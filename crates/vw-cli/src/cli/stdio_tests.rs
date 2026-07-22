use super::stdio::StdIoRedirectGuard;

#[cfg(unix)]
mod unix {
    use std::ffi::OsStr;
    use std::fs;
    use std::os::unix::ffi::OsStrExt;
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicU64, Ordering};

    use super::StdIoRedirectGuard;

    struct TestDir {
        path: PathBuf,
    }

    impl TestDir {
        fn new() -> Self {
            static NEXT_ID: AtomicU64 = AtomicU64::new(0);
            let id = NEXT_ID.fetch_add(1, Ordering::Relaxed);
            let path =
                std::env::temp_dir().join(format!("vw-cli-stdio-test-{}-{id}", std::process::id()));
            fs::create_dir_all(&path).expect("create stdio test directory");
            Self { path }
        }
    }

    impl Drop for TestDir {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.path);
        }
    }

    #[test]
    fn redirect_to_file_rejects_parent_that_is_a_regular_file() {
        let test_dir = TestDir::new();
        let blocking_parent = test_dir.path.join("not-a-directory");
        fs::write(&blocking_parent, b"fixture").expect("create blocking parent file");

        let result = StdIoRedirectGuard::redirect_to_file(&blocking_parent.join("output.log"));

        assert!(result.is_err());
    }

    #[test]
    fn redirect_to_file_rejects_directory_as_log_file() {
        let test_dir = TestDir::new();

        let result = StdIoRedirectGuard::redirect_to_file(&test_dir.path);

        assert!(result.is_err());
    }

    #[test]
    fn redirect_to_file_rejects_path_containing_nul_byte() {
        let invalid_path = PathBuf::from(OsStr::from_bytes(b"invalid\0stdio.log"));

        let result = StdIoRedirectGuard::redirect_to_file(&invalid_path);

        assert!(result.is_err());
    }

    #[test]
    fn redirect_to_file_creates_missing_parent_before_open_error() {
        let test_dir = TestDir::new();
        let nested_parent = test_dir.path.join("nested").join("logs");
        let invalid_name = OsStr::from_bytes(b"invalid\0stdio.log");
        let invalid_path = nested_parent.join(invalid_name);

        let result = StdIoRedirectGuard::redirect_to_file(&invalid_path);

        assert!(result.is_err());
        assert!(nested_parent.is_dir());
    }

    #[test]
    fn redirect_to_file_appends_stdout_and_stderr_until_guard_drops() {
        let test_dir = TestDir::new();
        let log_path = test_dir.path.join("agent.log");

        {
            let _guard = StdIoRedirectGuard::redirect_to_file(&log_path).expect("redirect stdio");
            write_fd(libc::STDOUT_FILENO, b"stdout-line\n");
            write_fd(libc::STDERR_FILENO, b"stderr-line\n");
        }

        let contents = fs::read_to_string(&log_path).expect("read redirected log");
        assert!(contents.contains("stdout-line"));
        assert!(contents.contains("stderr-line"));

        write_fd(libc::STDOUT_FILENO, b"");
    }

    fn write_fd(fd: i32, bytes: &[u8]) {
        let written = unsafe { libc::write(fd, bytes.as_ptr().cast(), bytes.len()) };
        assert_eq!(written, bytes.len() as isize);
    }
}

#[cfg(not(unix))]
#[test]
fn redirect_to_file_is_noop_on_non_unix() {
    let result = StdIoRedirectGuard::redirect_to_file(std::path::Path::new("ignored"));

    assert!(result.is_ok());
}
