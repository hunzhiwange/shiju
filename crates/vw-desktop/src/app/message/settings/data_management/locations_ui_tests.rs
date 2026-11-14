use super::*;
#[test]
fn cc_data_44_locations_ui_shows_partial_cleanup() {
    let report = DataBackupLocationDeletion { freed_bytes: 42, undeleted_count: 3 };
    let summary = cleanup_summary(&report);
    assert!(summary.contains("部分清理"));
    assert!(summary.contains("42"));
    assert!(summary.contains("3 项未删除"));
    let database = DataBackupLocation { id: "database".into(), size_bytes: 0, item_count: 0, last_modified: None, deletable: true };
    assert!(!can_cleanup(&database));
}
#[test]
fn cc_data_44_locations_ui_does_not_open_remote_path_locally() {
    assert!(!can_reveal(true, "remote-executor"));
    assert!(!can_reveal(false, vw_shared::local_apps::HOST_ID));
    assert!(can_reveal(true, vw_shared::local_apps::HOST_ID));
}
