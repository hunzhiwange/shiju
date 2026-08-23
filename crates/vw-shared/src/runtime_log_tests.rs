//! 运行时日志目录选择、级别分流与延迟写入测试。
//!
//! 夹具验证按小时归档、追加语义、父目录创建和默认产品目录。

use super::*;

#[test]
fn runtime_log_file_path_maps_every_level_to_hour_directory() {
    let root = Path::new("/tmp/vibewindow-log");
    let hour = "2026-07-04-00";

    for (level, file_name) in [
        (Level::ERROR, "error.log"),
        (Level::WARN, "warn.log"),
        (Level::INFO, "info.log"),
        (Level::DEBUG, "debug.log"),
        (Level::TRACE, "trace.log"),
    ] {
        assert_eq!(
            runtime_log_file_path_for_hour(root, hour, level),
            root.join(hour).join(file_name)
        );
    }
}

#[test]
fn runtime_log_writer_creates_parent_directories_lazily_and_appends() {
    let dir = tempfile::tempdir().expect("temp dir");
    let path = runtime_log_file_path_for_hour(dir.path(), "2026-07-04-00", Level::WARN);
    let parent = path.parent().expect("log parent");
    let mut writer = RuntimeLogWriter::new(path.clone());

    assert!(!parent.exists());
    assert!(writer.file.is_none());

    writer.write_all(b"first\n").expect("write first");
    writer.write_all(b"second\n").expect("write second");
    writer.flush().expect("flush");

    assert!(parent.is_dir());
    assert!(writer.file.is_some());
    assert_eq!(std::fs::read_to_string(path).expect("read log"), "first\nsecond\n");
}

#[test]
fn runtime_log_make_writer_defaults_to_info_and_preserves_requested_level() {
    let dir = tempfile::tempdir().expect("temp dir");
    let make_writer = RuntimeLogMakeWriter::new(dir.path());

    let default_writer = make_writer.make_writer();
    let error_writer = make_writer.writer_for_level(Level::ERROR);

    assert!(default_writer.path.starts_with(dir.path()));
    assert_eq!(default_writer.path.file_name().and_then(|name| name.to_str()), Some("info.log"));
    assert_eq!(error_writer.path.file_name().and_then(|name| name.to_str()), Some("error.log"));
}

#[test]
fn ensure_runtime_log_dir_creates_nested_directory() {
    let dir = tempfile::tempdir().expect("temp dir");
    let nested = dir.path().join("nested").join("runtime_log");

    ensure_runtime_log_dir(&nested).expect("create runtime log directory");

    assert!(nested.is_dir());
}

#[test]
fn default_home_runtime_log_dir_uses_product_home_name() {
    assert_eq!(
        default_home_runtime_log_dir(Path::new("/home/alice")),
        Path::new("/home/alice").join(".vibewindow").join("runtime_log")
    );
}
