use super::export_ui::save_download;

#[test]
fn cc_data_41_export_ui_saves_downloaded_bytes() {
    let root = tempfile::tempdir().unwrap();
    let destination = root.path().join("匿名配置.json");
    let bytes = br#"{"fixture":"portable"}"#;
    assert_eq!(save_download(Some(destination.clone()), bytes), Ok(true));
    assert_eq!(std::fs::read(&destination).unwrap(), bytes);
    assert!(save_download(Some(destination.clone()), b"overwrite").is_err());
    assert_eq!(std::fs::read(&destination).unwrap(), bytes);
}

#[test]
fn cc_data_41_export_ui_cancel_creates_no_file() {
    assert_eq!(save_download(None, b"portable fixture"), Ok(false));
}

#[test]
fn cc_data_41_export_ui_reports_missing_directory() {
    let root = tempfile::tempdir().unwrap();
    assert!(save_download(Some(root.path().join("missing/export.json")), b"fixture").is_err());
    assert_eq!(std::fs::read_dir(root.path()).unwrap().count(), 0);
}
