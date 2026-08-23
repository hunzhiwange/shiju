// 覆盖文件时间戳校验及同步、异步锁的并发语义。

use super::{
    Error, LOCKS, LOCKS_SYNC, READ, assert as assert_async, assert_sync, get, normalize, read,
    with_lock, with_lock_sync,
};
use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::sync::mpsc;
use std::sync::{Arc, Mutex};
use std::time::Duration;

fn filetime_check_disabled() -> bool {
    *crate::app::agent::flag::VIBEWINDOW_DISABLE_FILETIME_CHECK
}

#[test]
fn read_records_file_time_by_session() {
    read("session-a", "/tmp/example.txt");

    assert!(get("session-a", "/tmp/example.txt").is_some());
    assert!(get("session-b", "/tmp/example.txt").is_none());
}

#[test]
fn read_updates_existing_file_time_record() {
    read("session-update", "/tmp/update.txt");
    let first = get("session-update", "/tmp/update.txt").expect("first read time");

    std::thread::sleep(Duration::from_millis(2));
    read("session-update", "/tmp/update.txt");
    let second = get("session-update", "/tmp/update.txt").expect("second read time");

    assert!(second >= first);
}

#[test]
fn display_errors_include_target_path() {
    let err = Error::MustReadFirst { filepath: "/tmp/example.txt".to_string() };

    assert!(err.to_string().contains("/tmp/example.txt"));
}

#[test]
fn display_covers_all_error_variants_and_from_io() {
    let modified = Error::ModifiedSinceRead { filepath: "/tmp/changed.txt".to_string() };
    let io = Error::Io(std::io::Error::new(std::io::ErrorKind::NotFound, "missing"));
    let from_io: Error = std::io::Error::new(std::io::ErrorKind::PermissionDenied, "denied").into();

    assert!(modified.to_string().contains("/tmp/changed.txt"));
    assert!(io.to_string().contains("missing"));
    assert!(matches!(from_io, Error::Io(_)));
}

#[test]
fn assert_sync_accepts_unmodified_file() {
    if filetime_check_disabled() {
        return;
    }

    let temp = tempfile::tempdir().expect("tempdir");
    let path = temp.path().join("note.txt");
    fs::write(&path, "hello").expect("write note");
    let path_str = path.to_string_lossy().to_string();

    read("time-sync-ok", &path_str);

    assert_sync("time-sync-ok", &path).expect("file was read and not modified");
}

#[test]
fn assert_sync_requires_read_first_and_propagates_metadata_errors() {
    if filetime_check_disabled() {
        return;
    }

    let temp = tempfile::tempdir().expect("tempdir");
    let path = temp.path().join("missing.txt");
    let path_str = path.to_string_lossy().to_string();

    let must_read = assert_sync("time-sync-must-read", &path).expect_err("read first");
    assert!(matches!(must_read, Error::MustReadFirst { .. }));

    read("time-sync-io", &path_str);
    let io = assert_sync("time-sync-io", &path).expect_err("missing file metadata");
    assert!(matches!(io, Error::Io(_)));
}

#[test]
fn assert_sync_detects_modified_file_after_read() {
    if filetime_check_disabled() {
        return;
    }

    let temp = tempfile::tempdir().expect("tempdir");
    let path = temp.path().join("changed.txt");
    fs::write(&path, "before").expect("write before");
    let path_str = path.to_string_lossy().to_string();

    read("time-sync-modified", &path_str);
    std::thread::sleep(Duration::from_millis(20));
    fs::write(&path, "after").expect("write after");

    let err = assert_sync("time-sync-modified", &path).expect_err("modified after read");
    assert!(matches!(err, Error::ModifiedSinceRead { .. }));
}

#[test]
fn disabled_filetime_check_bypasses_sync_and_async_assertions() {
    if std::env::var_os("VW_AGENT_FILETIME_DISABLED_CHILD").is_some() {
        let temp = tempfile::tempdir().expect("tempdir");
        let missing = temp.path().join("missing.txt");

        assert_sync("time-disabled-sync", &missing).expect("sync check is disabled");
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("runtime")
            .block_on(async {
                assert_async("time-disabled-async", &missing)
                    .await
                    .expect("async check is disabled");
            });
        return;
    }

    let current_exe = std::env::current_exe().expect("current test executable");
    let status = Command::new(current_exe)
        .arg("disabled_filetime_check_bypasses_sync_and_async_assertions")
        .env("VW_AGENT_FILETIME_DISABLED_CHILD", "1")
        .env("VIBEWINDOW_DISABLE_FILETIME_CHECK", "1")
        .status()
        .expect("child test process should run");

    assert!(status.success());
}

#[test]
fn poisoned_global_registries_recover_in_child_process() {
    if std::env::var_os("VW_AGENT_POISONED_FILETIME_CHILD").is_some() {
        let read_poisoned = std::panic::catch_unwind(|| {
            let _guard = READ.lock().expect("read registry lock");
            panic!("poison read registry");
        });
        assert!(read_poisoned.is_err());
        assert_eq!(get("time-poisoned-read", "/tmp/time-poisoned-read.txt"), None);

        read("time-poisoned-read", "/tmp/time-poisoned-read.txt");
        let read_registry = READ.lock().unwrap_or_else(|error| error.into_inner());
        assert!(read_registry["time-poisoned-read"].contains_key("/tmp/time-poisoned-read.txt"));
        drop(read_registry);

        let async_locks_poisoned = std::panic::catch_unwind(|| {
            let _guard = LOCKS.lock().expect("async lock registry");
            panic!("poison async lock registry");
        });
        assert!(async_locks_poisoned.is_err());

        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("runtime")
            .block_on(async {
                assert_eq!(with_lock("/tmp/time-poisoned-async-lock.txt", async { 29 }).await, 29);
            });

        let sync_locks_poisoned = std::panic::catch_unwind(|| {
            let _guard = LOCKS_SYNC.lock().expect("sync lock registry");
            panic!("poison sync lock registry");
        });
        assert!(sync_locks_poisoned.is_err());
        assert_eq!(with_lock_sync("/tmp/time-poisoned-sync-lock.txt", || 31), 31);
        return;
    }

    let current_exe = std::env::current_exe().expect("current test executable");
    let status = Command::new(current_exe)
        .arg("poisoned_global_registries_recover_in_child_process")
        .env("VW_AGENT_POISONED_FILETIME_CHILD", "1")
        .status()
        .expect("child test process should run");

    assert!(status.success());
}

#[tokio::test]
async fn assert_async_accepts_unmodified_file() {
    if filetime_check_disabled() {
        return;
    }

    let temp = tempfile::tempdir().expect("tempdir");
    let path = temp.path().join("async-note.txt");
    fs::write(&path, "hello").expect("write note");
    let path_str = path.to_string_lossy().to_string();

    read("time-async-ok", &path_str);

    assert_async("time-async-ok", &path).await.expect("file was read and not modified");
}

#[tokio::test]
async fn assert_async_reports_read_metadata_and_modified_errors() {
    if filetime_check_disabled() {
        return;
    }

    let temp = tempfile::tempdir().expect("tempdir");
    let missing = temp.path().join("async-missing.txt");
    let missing_str = missing.to_string_lossy().to_string();

    let must_read = assert_async("time-async-must-read", &missing).await.expect_err("read first");
    assert!(matches!(must_read, Error::MustReadFirst { .. }));

    read("time-async-io", &missing_str);
    let io = assert_async("time-async-io", &missing).await.expect_err("missing file metadata");
    assert!(matches!(io, Error::Io(_)));

    let changed = temp.path().join("async-changed.txt");
    fs::write(&changed, "before").expect("write before");
    let changed_str = changed.to_string_lossy().to_string();
    read("time-async-modified", &changed_str);
    std::thread::sleep(Duration::from_millis(20));
    fs::write(&changed, "after").expect("write after");

    let modified =
        assert_async("time-async-modified", &changed).await.expect_err("modified after read");
    assert!(matches!(modified, Error::ModifiedSinceRead { .. }));
}

#[tokio::test]
async fn with_lock_runs_async_future_and_returns_value() {
    let value = with_lock("/tmp/example-async-lock.txt", async { 11 }).await;

    assert_eq!(value, 11);
}

#[tokio::test]
async fn with_lock_serializes_same_file_tasks() {
    let events = Arc::new(tokio::sync::Mutex::new(Vec::new()));
    let first_events = Arc::clone(&events);
    let second_events = Arc::clone(&events);
    let (locked_tx, locked_rx) = tokio::sync::oneshot::channel();

    let first = tokio::spawn(async move {
        with_lock("/tmp/time-async-serial.txt", async move {
            first_events.lock().await.push("first-start");
            locked_tx.send(()).expect("lock notification");
            tokio::time::sleep(Duration::from_millis(20)).await;
            first_events.lock().await.push("first-end");
        })
        .await;
    });
    locked_rx.await.expect("first task should hold lock");
    let second = tokio::spawn(async move {
        with_lock("/tmp/time-async-serial.txt", async move {
            second_events.lock().await.push("second-start");
        })
        .await;
    });

    first.await.expect("first task");
    second.await.expect("second task");

    let events = events.lock().await.clone();
    assert_eq!(events, vec!["first-start", "first-end", "second-start"]);
}

#[tokio::test]
async fn with_lock_allows_different_file_tasks_to_overlap() {
    let (first_locked_tx, first_locked_rx) = tokio::sync::oneshot::channel();
    let (release_first_tx, release_first_rx) = tokio::sync::oneshot::channel();

    let first = tokio::spawn(async move {
        with_lock("/tmp/time-async-distinct-first.txt", async move {
            first_locked_tx.send(()).expect("lock notification");
            release_first_rx.await.expect("release notification");
        })
        .await;
    });
    first_locked_rx.await.expect("first task should hold its lock");

    let second = tokio::time::timeout(
        Duration::from_secs(1),
        with_lock("/tmp/time-async-distinct-second.txt", async { 17 }),
    )
    .await
    .expect("a different file lock should not block");

    assert_eq!(second, 17);
    release_first_tx.send(()).expect("release first task");
    first.await.expect("first task");
}

#[test]
fn with_lock_sync_runs_closure_and_returns_value() {
    assert_eq!(with_lock_sync("/tmp/time-sync-return.txt", || 7), 7);
}

#[test]
fn with_lock_sync_serializes_same_file_tasks() {
    let events = Arc::new(Mutex::new(Vec::new()));
    let first_events = Arc::clone(&events);
    let second_events = Arc::clone(&events);
    let (locked_tx, locked_rx) = mpsc::channel();

    let first = std::thread::spawn(move || {
        with_lock_sync("/tmp/time-sync-serial.txt", || {
            first_events.lock().expect("events").push("first-start");
            locked_tx.send(()).expect("lock notification");
            std::thread::sleep(Duration::from_millis(20));
            first_events.lock().expect("events").push("first-end");
        });
    });
    locked_rx.recv().expect("first thread should hold lock");
    let second = std::thread::spawn(move || {
        with_lock_sync("/tmp/time-sync-serial.txt", || {
            second_events.lock().expect("events").push("second-start");
        });
    });

    first.join().expect("first thread");
    second.join().expect("second thread");

    let events = events.lock().expect("events").clone();
    assert_eq!(events, vec!["first-start", "first-end", "second-start"]);
}

#[test]
fn with_lock_sync_allows_different_file_tasks_to_overlap() {
    let (first_locked_tx, first_locked_rx) = mpsc::channel();
    let (release_first_tx, release_first_rx) = mpsc::channel();

    let first = std::thread::spawn(move || {
        with_lock_sync("/tmp/time-sync-distinct-first.txt", || {
            first_locked_tx.send(()).expect("lock notification");
            release_first_rx.recv().expect("release notification");
        });
    });
    first_locked_rx.recv().expect("first thread should hold its lock");

    let (second_locked_tx, second_locked_rx) = mpsc::channel();
    let second = std::thread::spawn(move || {
        with_lock_sync("/tmp/time-sync-distinct-second.txt", || {
            second_locked_tx.send(()).expect("second lock notification");
        });
    });

    second_locked_rx
        .recv_timeout(Duration::from_secs(1))
        .expect("a different file lock should not block");
    release_first_tx.send(()).expect("release first thread");
    first.join().expect("first thread");
    second.join().expect("second thread");
}

#[test]
fn with_lock_sync_recovers_poisoned_file_lock() {
    let poisoned = std::panic::catch_unwind(|| {
        with_lock_sync("/tmp/time-sync-poisoned.txt", || panic!("poison file lock"));
    });

    assert!(poisoned.is_err());
    assert_eq!(with_lock_sync("/tmp/time-sync-poisoned.txt", || 23), 23);
}

#[test]
fn normalize_returns_lossy_path_string() {
    let relative = PathBuf::from("relative.txt");
    let absolute = std::env::temp_dir().join("absolute.txt");

    assert_eq!(normalize(&relative), "relative.txt");
    assert_eq!(normalize(&absolute), absolute.to_string_lossy().to_string());
}

#[cfg(unix)]
#[test]
fn normalize_replaces_invalid_utf8_path_bytes() {
    use std::ffi::OsString;
    use std::os::unix::ffi::OsStringExt;

    let path = PathBuf::from(OsString::from_vec(vec![b'f', 0x80]));

    assert_eq!(normalize(path), "f\u{fffd}");
}
