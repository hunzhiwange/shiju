//! 验证日志级别、文件轮换、结构化字段、缓存记录器与计时器行为。
//! 全局日志状态在相关用例前重置，临时目录隔离文件副作用和轮换顺序。

use super::{
    DEFAULT, InitOptions, LAST_MS, LEVEL, LOG_PATH, Level, Timer, WRITER, Writer, cleanup, create,
    file, init_in_dir, is_timestamp_log_name, iso_file_stamp_utc, iso_seconds_utc, map_from_pairs,
    should_log, write_line,
};
use serde_json::{Map, Value, json};
use std::fs;
use std::io::BufWriter;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

static NEXT_TEMP_DIR: AtomicU64 = AtomicU64::new(0);
static LOG_STATE_LOCK: Mutex<()> = Mutex::new(());

struct TestDir {
    path: PathBuf,
}

impl TestDir {
    fn new() -> Self {
        let id = NEXT_TEMP_DIR.fetch_add(1, Ordering::Relaxed);
        let path =
            std::env::temp_dir().join(format!("vw-agent-log-tests-{}-{id}", std::process::id()));
        fs::create_dir(&path).expect("create isolated log test directory");
        Self { path }
    }

    fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for TestDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

fn timestamp_log_name(index: usize) -> String {
    format!("2024-01-01T0000{index:02}.log")
}

fn create_timestamp_logs(dir: &Path, count: usize) -> Vec<PathBuf> {
    (0..count)
        .map(|index| {
            let path = dir.join(timestamp_log_name(index));
            fs::write(&path, b"log").expect("create timestamp log fixture");
            path
        })
        .collect()
}

// 日志实现含进程级静态状态，测试必须恢复默认值才能保持可重复执行。
fn reset_log_state() {
    LEVEL.store(Level::Info.priority(), Ordering::Relaxed);
    LAST_MS.store(0, Ordering::Relaxed);
    *WRITER.lock().expect("writer lock") = Writer::Stderr;
    *LOG_PATH.lock().expect("log path lock") = None;
}

fn install_file_writer(path: &Path) {
    let file = fs::OpenOptions::new().create(true).append(true).open(path).expect("open test log");
    *WRITER.lock().expect("writer lock") = Writer::File(Arc::new(Mutex::new(BufWriter::new(file))));
    *LOG_PATH.lock().expect("log path lock") = Some(path.to_path_buf());
}

#[test]
fn level_metadata_maps_every_level_to_stable_priority_and_label() {
    let cases = [
        (Level::Debug, 0, "DEBUG"),
        (Level::Info, 1, "INFO "),
        (Level::Warn, 2, "WARN "),
        (Level::Error, 3, "ERROR"),
    ];

    for (level, priority, label) in cases {
        assert_eq!(level.priority(), priority);
        assert_eq!(level.label(), label);
    }
}

#[test]
fn timestamp_log_name_accepts_only_the_structural_filename_shape() {
    assert!(is_timestamp_log_name("2024-03-15T143022.log"));
    assert!(is_timestamp_log_name("9999-99-99T999999.log"));

    for invalid in [
        "2024-03-15T143022",
        "2024-03-15T14302x.log",
        "2024/03-15T143022.log",
        "2024-03-15_143022.log",
        "2024-03-15T143022.txt",
    ] {
        assert!(!is_timestamp_log_name(invalid), "accepted invalid name: {invalid}");
    }
}

#[test]
fn generated_utc_timestamps_have_the_documented_shapes() {
    let seconds = iso_seconds_utc();
    assert_eq!(seconds.len(), "0000-00-00T00:00:00".len());
    assert_eq!(seconds.get(4..5), Some("-"));
    assert_eq!(seconds.get(7..8), Some("-"));
    assert_eq!(seconds.get(10..11), Some("T"));
    assert_eq!(seconds.get(13..14), Some(":"));
    assert_eq!(seconds.get(16..17), Some(":"));
    assert!(
        seconds.bytes().enumerate().all(|(index, byte)| {
            matches!(index, 4 | 7 | 10 | 13 | 16) || byte.is_ascii_digit()
        })
    );

    let file_name = format!("{}.log", iso_file_stamp_utc());
    assert!(is_timestamp_log_name(&file_name));
}

#[test]
fn cleanup_tolerates_missing_directory_and_keeps_small_sets() {
    let dir = TestDir::new();
    let missing = dir.path().join("missing");
    cleanup(&missing);
    assert!(!missing.exists());

    let logs = create_timestamp_logs(dir.path(), 5);
    let unrelated = dir.path().join("dev.log");
    fs::write(&unrelated, b"log").expect("create unrelated log fixture");
    cleanup(dir.path());

    assert!(logs.iter().all(|path| path.exists()));
    assert!(unrelated.exists());
}

#[test]
fn cleanup_keeps_six_to_ten_timestamp_logs() {
    let dir = TestDir::new();
    let logs = create_timestamp_logs(dir.path(), 10);

    cleanup(dir.path());

    assert!(logs.iter().all(|path| path.exists()));
}

#[test]
fn cleanup_removes_only_oldest_timestamp_logs_beyond_limit() {
    let dir = TestDir::new();
    let logs = create_timestamp_logs(dir.path(), 12);
    let unrelated = dir.path().join("dev.log");
    fs::write(&unrelated, b"log").expect("create unrelated log fixture");

    cleanup(dir.path());

    assert!(!logs[0].exists());
    assert!(!logs[1].exists());
    assert!(logs[2..].iter().all(|path| path.exists()));
    assert!(unrelated.exists());
}

#[test]
fn file_writer_records_lines_and_reports_current_path() {
    let _guard = LOG_STATE_LOCK.lock().expect("log state lock");
    reset_log_state();
    let dir = TestDir::new();
    let path = dir.path().join("direct.log");
    install_file_writer(&path);

    assert_eq!(file(), Some(path.clone()));
    write_line("first\n");
    write_line("second\n");

    assert_eq!(fs::read_to_string(&path).expect("read log"), "first\nsecond\n");
    reset_log_state();
}

#[test]
fn init_print_mode_updates_level_and_clears_file_state() {
    let _guard = LOG_STATE_LOCK.lock().expect("log state lock");
    reset_log_state();
    let dir = TestDir::new();
    install_file_writer(&dir.path().join("previous.log"));

    init_in_dir(InitOptions { print: true, dev: true, level: Some(Level::Error) }, dir.path());

    assert_eq!(file(), None);
    assert!(!should_log(Level::Warn));
    assert!(should_log(Level::Error));
    write_line("");
    reset_log_state();
}

#[test]
fn init_file_mode_creates_dev_log_and_tracks_path() {
    let _guard = LOG_STATE_LOCK.lock().expect("log state lock");
    reset_log_state();
    let dir = TestDir::new();

    init_in_dir(InitOptions { print: false, dev: true, level: Some(Level::Debug) }, dir.path());

    let path = dir.path().join("dev.log");
    assert_eq!(file(), Some(path.clone()));
    assert!(should_log(Level::Debug));

    write_line("raw log\n");
    assert_eq!(fs::read_to_string(&path).expect("read log"), "raw log\n");
    reset_log_state();
}

#[test]
fn logger_methods_respect_level_filter_and_write_structured_lines() {
    let _guard = LOG_STATE_LOCK.lock().expect("log state lock");
    reset_log_state();
    let dir = TestDir::new();
    let path = dir.path().join("filtered.log");
    install_file_writer(&path);
    LEVEL.store(Level::Warn.priority(), Ordering::Relaxed);

    let logger =
        create(Some(map_from_pairs([("service", Value::String("task-670-filter".to_string()))])));
    logger.debug("debug hidden", None);
    logger.info("info hidden", None);
    logger.warn("warn shown", None);
    logger.error("error shown", None);

    let content = fs::read_to_string(&path).expect("read log");
    assert!(!content.contains("debug hidden"));
    assert!(!content.contains("info hidden"));
    assert!(content.contains("WARN"));
    assert!(content.contains("warn shown"));
    assert!(content.contains("ERROR"));
    assert!(content.contains("error shown"));
    reset_log_state();
}

#[test]
fn timer_logs_started_and_completed_once() {
    let _guard = LOG_STATE_LOCK.lock().expect("log state lock");
    reset_log_state();
    let dir = TestDir::new();
    let path = dir.path().join("timer.log");
    install_file_writer(&path);
    LEVEL.store(Level::Info.priority(), Ordering::Relaxed);

    let mut extra = Map::new();
    extra.insert("operation".to_string(), Value::String("index".to_string()));
    let logger = create(Some(map_from_pairs([("component", Value::String("timer".to_string()))])));
    let timer = logger.time("timed work", Some(extra));
    timer.stop();
    timer.stop();
    drop(timer);

    let content = fs::read_to_string(&path).expect("read log");
    assert_eq!(content.matches("timed work").count(), 2);
    assert!(content.contains("status=started"));
    assert!(content.contains("status=completed"));
    assert!(content.contains("operation=index"));
    assert!(content.contains("duration="));
    reset_log_state();
}

#[test]
fn default_logger_uses_default_service_tag() {
    assert_eq!(DEFAULT.tags_snapshot().get("service"), Some(&Value::String("default".to_string())));
}

#[test]
fn map_from_pairs_collects_values_and_uses_the_last_duplicate() {
    let map = map_from_pairs([
        ("mode", Value::String("first".to_string())),
        ("count", Value::Number(1.into())),
        ("mode", Value::String("last".to_string())),
    ]);

    assert_eq!(map.len(), 2);
    assert_eq!(map.get("mode"), Some(&Value::String("last".to_string())));
    assert_eq!(map.get("count"), Some(&Value::Number(1.into())));
}

#[test]
fn create_reuses_string_service_logger_and_its_shared_tags() {
    let first =
        create(Some(map_from_pairs([("service", Value::String("task-247-cache".to_string()))])));
    let second = create(Some(map_from_pairs([
        ("service", Value::String("task-247-cache".to_string())),
        ("ignored", Value::Bool(true)),
    ])));

    assert!(Arc::ptr_eq(&first.inner, &second.inner));
    assert!(!first.tags_snapshot().contains_key("ignored"));

    second.tag("shared", "yes");
    assert_eq!(first.tags_snapshot().get("shared"), Some(&Value::String("yes".to_string())));
}

#[test]
fn create_does_not_cache_absent_or_non_string_service_tags() {
    let without_service_a = create(None);
    let without_service_b = create(None);
    assert!(!Arc::ptr_eq(&without_service_a.inner, &without_service_b.inner));

    let numeric_service_a = create(Some(map_from_pairs([("service", Value::Number(247.into()))])));
    let numeric_service_b = create(Some(map_from_pairs([("service", Value::Number(247.into()))])));
    assert!(!Arc::ptr_eq(&numeric_service_a.inner, &numeric_service_b.inner));
}

#[test]
fn clone_logger_snapshots_non_service_tags_independently() {
    let original =
        create(Some(map_from_pairs([("component", Value::String("scheduler".to_string()))])));
    let cloned = original.clone_logger();

    assert!(!Arc::ptr_eq(&original.inner, &cloned.inner));
    assert_eq!(original.tags_snapshot(), cloned.tags_snapshot());

    cloned.tag("request", "247");
    assert!(!original.tags_snapshot().contains_key("request"));
    assert_eq!(cloned.tags_snapshot().get("request"), Some(&Value::String("247".to_string())));
}

#[test]
fn build_merges_tags_overrides_values_and_filters_nulls() {
    let logger = create(Some(map_from_pairs([
        ("inherited", Value::String("kept".to_string())),
        ("overridden", Value::String("base".to_string())),
        ("removed", Value::String("base".to_string())),
    ])));
    let mut extra = Map::new();
    extra.insert("overridden".to_string(), Value::String("extra".to_string()));
    extra.insert("removed".to_string(), Value::Null);
    extra.insert("attempt".to_string(), Value::Number(3.into()));
    extra.insert("active".to_string(), Value::Bool(true));
    extra.insert("detail".to_string(), json!({ "kind": "io" }));

    let line = logger.build(Level::Warn, "request failed", Some(&extra));

    assert!(line.starts_with("WARN  "));
    assert!(line.ends_with(" request failed\n"));
    assert!(line.contains("inherited=kept"));
    assert!(line.contains("overridden=extra"));
    assert!(line.contains("attempt=3"));
    assert!(line.contains("active=true"));
    assert!(line.contains("detail={\"kind\":\"io\"}"));
    assert!(!line.contains("overridden=base"));
    assert!(!line.contains("removed="));
}

#[test]
fn build_omits_empty_message_and_empty_tag_prefix() {
    let line = create(None).build(Level::Info, "", None);

    assert!(line.starts_with("INFO  "));
    assert!(line.ends_with('\n'));
    assert!(!line.ends_with(" \n"));
}

#[test]
fn stopped_timer_rejects_repeated_stop_without_changing_state() {
    let timer = Timer {
        stopped: AtomicBool::new(true),
        logger: create(None),
        message: "already completed".to_string(),
        start_ms: 0,
        extra: None,
    };

    timer.stop();

    assert!(timer.stopped.load(Ordering::Relaxed));
}
