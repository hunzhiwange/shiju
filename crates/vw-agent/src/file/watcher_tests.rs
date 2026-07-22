use super::event::UPDATED;
use super::{UpdatedProperties, publish_updated};
use crate::app::agent::bus;
use serde_json::Value;
#[cfg(not(target_arch = "wasm32"))]
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
#[cfg(not(target_arch = "wasm32"))]
use std::time::{Duration, Instant, SystemTime};

#[test]
fn watcher_event_contract_is_stable() {
    let props = UpdatedProperties { file: "src/lib.rs".to_string(), event: "change".to_string() };
    let json = serde_json::to_value(props).expect("serialize");

    assert_eq!(UPDATED.r#type, "file.watcher.updated");
    assert_eq!(json["file"], "src/lib.rs");
    assert_eq!(json["event"], "change");
    assert_eq!(json.as_object().expect("object").len(), 2);
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn snapshot_updates_reports_add_change_and_unlink_without_unchanged_files() {
    let initial = SystemTime::UNIX_EPOCH;
    let changed = initial + Duration::from_secs(1);
    let previous = HashMap::from([
        ("changed.rs".to_string(), initial),
        ("removed.rs".to_string(), initial),
        ("unchanged.rs".to_string(), initial),
    ]);
    let next = HashMap::from([
        ("added.rs".to_string(), changed),
        ("changed.rs".to_string(), changed),
        ("unchanged.rs".to_string(), initial),
    ]);

    let updates = super::snapshot_updates(&previous, &next);
    let by_file =
        updates.into_iter().map(|update| (update.file, update.event)).collect::<HashMap<_, _>>();

    assert_eq!(by_file.len(), 3);
    assert_eq!(by_file.get("added.rs").map(String::as_str), Some("add"));
    assert_eq!(by_file.get("changed.rs").map(String::as_str), Some("change"));
    assert_eq!(by_file.get("removed.rs").map(String::as_str), Some("unlink"));
    assert!(!by_file.contains_key("unchanged.rs"));
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn snapshot_updates_returns_empty_for_identical_and_empty_snapshots() {
    let snapshot = HashMap::from([("src/lib.rs".to_string(), SystemTime::UNIX_EPOCH)]);

    assert!(super::snapshot_updates(&snapshot, &snapshot).is_empty());
    assert!(super::snapshot_updates(&HashMap::new(), &HashMap::new()).is_empty());
}

#[test]
fn publish_updated_emits_expected_payload() {
    let seen = Arc::new(Mutex::new(Vec::<Value>::new()));
    let slot = Arc::clone(&seen);
    let unsubscribe = bus::subscribe(UPDATED, move |payload| {
        slot.lock().unwrap().push(payload);
    });

    publish_updated("src/main.rs", "change");
    unsubscribe();

    let lock = seen.lock().unwrap();
    assert_eq!(lock.len(), 1);
    assert_eq!(lock[0]["type"], UPDATED.r#type);
    assert_eq!(lock[0]["properties"]["file"], "src/main.rs");
    assert_eq!(lock[0]["properties"]["event"], "change");
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn scan_snapshot_collects_files_and_skips_ignored_entries() {
    let temp = tempfile::tempdir().expect("tempdir");
    let root = temp.path();
    std::fs::create_dir_all(root.join("src")).expect("create src");
    std::fs::create_dir_all(root.join(".git/objects")).expect("create git");
    std::fs::create_dir_all(root.join("target/debug")).expect("create target");
    std::fs::write(root.join("src/lib.rs"), "pub fn ok() {}\n").expect("write lib");
    std::fs::write(root.join("app.log"), "noise\n").expect("write log");
    std::fs::write(root.join(".git/objects/ignored"), "git\n").expect("write git");
    std::fs::write(root.join("target/debug/app"), "bin\n").expect("write target");

    #[cfg(unix)]
    std::os::unix::fs::symlink(root.join("src/lib.rs"), root.join("src/link.rs"))
        .expect("create symlink");

    let snapshot = super::scan_snapshot(root);

    assert!(snapshot.contains_key("src/lib.rs"));
    assert!(!snapshot.contains_key("app.log"));
    assert!(!snapshot.contains_key(".git/objects/ignored"));
    assert!(!snapshot.contains_key("target/debug/app"));
    #[cfg(unix)]
    assert!(!snapshot.contains_key("src/link.rs"));
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn scan_snapshot_uses_nested_relative_paths_and_excludes_directories() {
    let temp = tempfile::tempdir().expect("tempdir");
    let nested = temp.path().join("src/nested");
    std::fs::create_dir_all(&nested).expect("create nested directory");
    std::fs::write(nested.join("module.rs"), "pub const VALUE: u8 = 1;\n").expect("write module");

    let snapshot = super::scan_snapshot(temp.path());

    assert_eq!(snapshot.len(), 1);
    assert!(snapshot.contains_key("src/nested/module.rs"));
    assert!(!snapshot.contains_key("src"));
    assert!(!snapshot.contains_key("src/nested"));
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn scan_snapshot_returns_empty_for_missing_root() {
    let temp = tempfile::tempdir().expect("tempdir");
    let missing = temp.path().join("missing");

    assert!(super::scan_snapshot(&missing).is_empty());
}

#[cfg(not(target_arch = "wasm32"))]
#[tokio::test]
async fn running_and_stop_reflect_task_registry() {
    let temp = tempfile::tempdir().expect("tempdir");
    let other = tempfile::tempdir().expect("other tempdir");
    let key = temp.path().to_string_lossy().to_string();
    super::stop(temp.path());

    assert!(!super::running(temp.path()));
    assert!(!super::running(other.path()));

    let handle = tokio::spawn(async {
        std::future::pending::<()>().await;
    });
    {
        let mut tasks = super::TASKS.lock().unwrap_or_else(|e| e.into_inner());
        tasks.insert(key.clone(), handle);
    }

    assert!(super::running(temp.path()));
    assert!(!super::running(other.path()));
    super::stop(temp.path());
    assert!(!super::running(temp.path()));
    assert!(!super::TASKS.lock().unwrap_or_else(|e| e.into_inner()).contains_key(&key));
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn stop_is_idempotent_for_unregistered_directory() {
    let temp = tempfile::tempdir().expect("tempdir");

    super::stop(temp.path());
    super::stop(temp.path());

    assert!(!super::running(temp.path()));
}

#[cfg(not(target_arch = "wasm32"))]
#[tokio::test]
async fn init_is_noop_when_filewatcher_flag_is_disabled() {
    let temp = tempfile::tempdir().expect("tempdir");
    super::stop(temp.path());

    if !*crate::app::agent::flag::VIBEWINDOW_EXPERIMENTAL_FILEWATCHER {
        super::init(temp.path());
        assert!(!super::running(temp.path()));
    }
}

#[cfg(not(target_arch = "wasm32"))]
#[tokio::test]
async fn init_respects_flags_and_deduplicates_enabled_directory() {
    let temp = tempfile::tempdir().expect("tempdir");
    let key = temp.path().to_string_lossy().to_string();
    super::stop(temp.path());

    super::init(temp.path());

    let should_run = *crate::app::agent::flag::VIBEWINDOW_EXPERIMENTAL_FILEWATCHER
        && !*crate::app::agent::flag::VIBEWINDOW_EXPERIMENTAL_DISABLE_FILEWATCHER;
    assert_eq!(super::running(temp.path()), should_run);

    if should_run {
        let first_id = super::TASKS
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .get(&key)
            .expect("registered watcher")
            .id();
        super::init(temp.path());
        let tasks = super::TASKS.lock().unwrap_or_else(|e| e.into_inner());
        assert_eq!(tasks.get(&key).expect("deduplicated watcher").id(), first_id);
        drop(tasks);
    }

    super::stop(temp.path());
}

#[cfg(not(target_arch = "wasm32"))]
#[tokio::test]
async fn init_enabled_watcher_publishes_real_file_system_changes() {
    if !filewatcher_should_run() {
        return;
    }

    let temp = tempfile::tempdir().expect("tempdir");
    let seen = Arc::new(Mutex::new(Vec::<Value>::new()));
    let slot = Arc::clone(&seen);
    let unsubscribe = bus::subscribe(UPDATED, move |payload| {
        slot.lock().unwrap().push(payload);
    });
    super::stop(temp.path());

    super::init(temp.path());
    tokio::time::sleep(Duration::from_millis(100)).await;

    std::fs::write(temp.path().join("watched.rs"), "pub const VALUE: u8 = 1;\n")
        .expect("write watched file");
    wait_for_watcher_event(&seen, "watched.rs", "add").await;

    std::fs::write(temp.path().join("watched.rs"), "pub const VALUE: u8 = 2;\n")
        .expect("change watched file");
    wait_for_watcher_event(&seen, "watched.rs", "change").await;

    std::fs::remove_file(temp.path().join("watched.rs")).expect("remove watched file");
    wait_for_watcher_event(&seen, "watched.rs", "unlink").await;

    super::stop(temp.path());
    unsubscribe();
}

#[cfg(target_arch = "wasm32")]
#[test]
fn wasm_watcher_functions_are_noops() {
    super::init("/tmp/project");
    assert!(!super::running("/tmp/project"));
    super::stop("/tmp/project");
}

#[cfg(not(target_arch = "wasm32"))]
fn filewatcher_should_run() -> bool {
    *crate::app::agent::flag::VIBEWINDOW_EXPERIMENTAL_FILEWATCHER
        && !*crate::app::agent::flag::VIBEWINDOW_EXPERIMENTAL_DISABLE_FILEWATCHER
}

#[cfg(not(target_arch = "wasm32"))]
async fn wait_for_watcher_event(seen: &Arc<Mutex<Vec<Value>>>, file: &str, event: &str) {
    let deadline = Instant::now() + Duration::from_secs(3);
    loop {
        {
            let lock = seen.lock().unwrap();
            if lock.iter().any(|payload| {
                payload["type"] == UPDATED.r#type
                    && payload["properties"]["file"] == file
                    && payload["properties"]["event"] == event
            }) {
                return;
            }
        }
        assert!(Instant::now() < deadline, "timed out waiting for watcher event {event} on {file}");
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
}
