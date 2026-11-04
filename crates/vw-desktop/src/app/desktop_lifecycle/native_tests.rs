//! 在隔离目录验证偏好持久化，不修改用户启动项或系统托盘。
use super::*;

#[test]
fn close_preference_survives_reload_and_can_be_disabled() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("nested/preferences.json");
    assert!(!read_close_to_tray(&path).unwrap());
    write_close_to_tray(&path, true).unwrap();
    assert!(read_close_to_tray(&path).unwrap());
    write_close_to_tray(&path, false).unwrap();
    assert!(!read_close_to_tray(&path).unwrap());
}

#[test]
fn close_preference_rejects_corrupt_data_and_reports_write_failure() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("preferences.json");
    std::fs::write(&path, b"invalid").unwrap();
    assert!(read_close_to_tray(&path).is_err());
    assert!(write_close_to_tray(&path.join("child.json"), true).is_err());
}

#[cfg(target_os = "macos")]
#[test]
fn login_item_requires_app_bundle_metadata_and_executable_layout() {
    let dir = tempfile::tempdir().unwrap();
    let bundle = dir.path().join("识矩.app");
    let contents = bundle.join("Contents");
    std::fs::create_dir_all(contents.join("MacOS")).unwrap();
    let executable = contents.join("MacOS/shi-ju");
    assert_eq!(macos_app_bundle(&executable), None);
    std::fs::write(contents.join("Info.plist"), b"fixture").unwrap();
    assert_eq!(macos_app_bundle(&executable), Some(bundle.as_path()));
    assert_eq!(macos_app_bundle(&dir.path().join("target/debug/shi-ju")), None);
    assert_eq!(macos_app_bundle(&contents.join("Helpers/shi-ju")), None);
}
