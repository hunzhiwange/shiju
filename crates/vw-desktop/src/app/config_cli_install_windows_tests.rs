use super::*;

#[test]
fn file_operation_retries_transient_windows_locks_until_success() {
    for code in [5, 32, 33] {
        let mut attempts = 0;
        retry_windows_file_operation(|| {
            attempts += 1;
            if attempts < 3 { Err(std::io::Error::from_raw_os_error(code)) } else { Ok(()) }
        })
        .unwrap();
        assert_eq!(attempts, 3, "Windows error {code}");
    }
}

#[test]
fn file_operation_stops_after_six_attempts_and_preserves_last_error() {
    let mut attempts = 0;
    let error = retry_windows_file_operation(|| {
        attempts += 1;
        Err(std::io::Error::from_raw_os_error(32))
    })
    .unwrap_err();

    assert_eq!(attempts, 6);
    assert_eq!(error.raw_os_error(), Some(32));
}

#[test]
fn file_operation_returns_permanent_error_without_retrying() {
    let mut attempts = 0;
    let error = retry_windows_file_operation(|| {
        attempts += 1;
        Err(std::io::Error::from_raw_os_error(87))
    })
    .unwrap_err();

    assert_eq!(attempts, 1);
    assert_eq!(error.raw_os_error(), Some(87));
}

#[test]
fn replace_staged_cli_installs_and_cleans_backup() {
    let dir = tempfile::tempdir().unwrap();
    let target = dir.path().join("shiju.exe");
    let staged = dir.path().join("staged.exe");
    fs::write(&target, b"old cli").unwrap();
    fs::write(&staged, b"new cli").unwrap();

    replace_staged_cli(&staged, &target).unwrap();

    assert_eq!(fs::read(&target).unwrap(), b"new cli");
    assert_eq!(fs::read_dir(dir.path()).unwrap().count(), 1);
}

#[test]
fn replace_staged_cli_restores_old_cli_when_placement_fails() {
    let dir = tempfile::tempdir().unwrap();
    let target = dir.path().join("shiju.exe");
    fs::write(&target, b"old cli").unwrap();

    let error = replace_staged_cli(&dir.path().join("missing.exe"), &target).unwrap_err();

    assert!(error.contains("替换 CLI 可执行文件失败"), "{error}");
    assert_eq!(fs::read(&target).unwrap(), b"old cli");
    assert_eq!(fs::read_dir(dir.path()).unwrap().count(), 1);
}

#[test]
fn replace_staged_cli_supports_first_install() {
    let dir = tempfile::tempdir().unwrap();
    let target = dir.path().join("shiju.exe");
    let staged = dir.path().join("staged.exe");
    fs::write(&staged, b"new cli").unwrap();

    replace_staged_cli(&staged, &target).unwrap();

    assert_eq!(fs::read(&target).unwrap(), b"new cli");
    assert_eq!(fs::read_dir(dir.path()).unwrap().count(), 1);
}

#[test]
fn replace_staged_cli_preserves_directory_target_and_staged_file() {
    let dir = tempfile::tempdir().unwrap();
    let target = dir.path().join("shiju.exe");
    let staged = dir.path().join("staged.exe");
    fs::create_dir(&target).unwrap();
    fs::write(target.join("data"), b"existing data").unwrap();
    fs::write(&staged, b"new cli").unwrap();

    assert!(replace_staged_cli(&staged, &target).unwrap_err().contains("不是普通文件"));
    assert_eq!(fs::read(target.join("data")).unwrap(), b"existing data");
    assert_eq!(fs::read(&staged).unwrap(), b"new cli");
}

#[cfg(windows)]
struct RunningCli(std::process::Child);

#[cfg(windows)]
impl Drop for RunningCli {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

#[cfg(windows)]
fn running_cli(target: &Path) -> RunningCli {
    use std::process::Stdio;
    let cmd =
        std::path::PathBuf::from(std::env::var_os("SystemRoot").unwrap()).join("System32/cmd.exe");
    fs::copy(cmd, target).unwrap();
    let mut process = super::super::background_command(target)
        .args(["/D", "/Q"])
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    assert!(process.try_wait().unwrap().is_none());
    RunningCli(process)
}

#[cfg(windows)]
#[test]
fn replace_staged_cli_waits_for_exclusive_file_lock_to_release() {
    use std::os::windows::fs::OpenOptionsExt;

    let dir = tempfile::tempdir().unwrap();
    let target = dir.path().join("shiju.exe");
    let staged = dir.path().join("staged.exe");
    fs::write(&target, b"old cli").unwrap();
    fs::write(&staged, b"new cli").unwrap();
    let lock = fs::OpenOptions::new().read(true).share_mode(0).open(&target).unwrap();
    let release = std::thread::spawn(move || {
        std::thread::sleep(Duration::from_millis(150));
        drop(lock);
    });

    let result = replace_staged_cli(&staged, &target);
    release.join().unwrap();
    result.unwrap();

    assert_eq!(fs::read(&target).unwrap(), b"new cli");
    assert_eq!(fs::read_dir(dir.path()).unwrap().count(), 1);
}

#[cfg(windows)]
#[test]
fn replace_staged_cli_preserves_running_image_without_deleting_it() {
    let dir = tempfile::tempdir().unwrap();
    let target = dir.path().join("shiju.exe");
    let staged = dir.path().join("staged.exe");
    let mut running = running_cli(&target);
    assert!(fs::remove_file(&target).is_err(), "a loaded executable cannot be deleted");
    fs::write(&staged, b"new cli").unwrap();

    replace_staged_cli(&staged, &target).unwrap();

    assert_eq!(fs::read(&target).unwrap(), b"new cli");
    assert!(running.0.try_wait().unwrap().is_none());
    assert_eq!(fs::read_dir(dir.path()).unwrap().count(), 2, "loaded backup is retained");
    drop(running);
}

#[cfg(windows)]
#[test]
fn stop_cli_process_checks_full_path_and_waits_for_exit() {
    let dir = tempfile::tempdir().unwrap();
    let cli_dir = dir.path().join("安装 目录");
    fs::create_dir(&cli_dir).unwrap();
    let target = cli_dir.join("shiju.exe");
    let other = dir.path().join("shiju.exe");
    let mut running = running_cli(&target);
    fs::copy(&target, &other).unwrap();

    let error = stop_cli_process(running.0.id(), &other).unwrap_err();
    assert!(error.contains("不属于"), "{error}");
    assert!(running.0.try_wait().unwrap().is_none());

    stop_cli_process(running.0.id(), &target).unwrap();
    assert!(running.0.try_wait().unwrap().is_some());
    fs::remove_file(&target).expect("the loaded executable is unlocked before returning");
}
