// 覆盖工作区可写性、磁盘空间及必需文件检查。

use super::workspace_checks::{
    append_disk_space_check, check_file_exists, check_workspace_with_disk_probe,
    parse_df_available_mb, workspace_probe_path,
};
use super::{DiagItem, Severity};
use crate::app::agent::config::Config;

fn config_for_workspace(path: std::path::PathBuf) -> Config {
    Config { workspace_dir: path, ..Config::default() }
}

// 诊断断言同时检查严重级别与可操作提示，避免只有返回值正确而用户指引失真。
#[test]
fn parse_df_available_mb_reads_last_data_line() {
    let stdout = "Filesystem 1M-blocks Used Available Use% Mounted on\n/dev/disk 100 20 80 20% /";

    assert_eq!(parse_df_available_mb(stdout), Some(80));
    assert_eq!(parse_df_available_mb(""), None);
    assert_eq!(
        parse_df_available_mb(
            "Filesystem 1M-blocks Used Available Use% Mounted on\n/dev/disk 100 20 nope 20% /"
        ),
        None
    );
    assert_eq!(parse_df_available_mb("Filesystem 1M-blocks Used Available"), None);
}

#[test]
fn parse_df_available_mb_ignores_blank_lines_and_uses_last_mount() {
    let stdout = "\
Filesystem 1M-blocks Used Available Use% Mounted on
/dev/disk1 100 20 80 20% /
/dev/disk2 200 50 150 25% /Volumes/Data

";

    assert_eq!(parse_df_available_mb(stdout), Some(150));
}

#[test]
fn workspace_probe_path_stays_inside_workspace() {
    let root = tempfile::tempdir().expect("tempdir");
    let probe = workspace_probe_path(root.path());

    assert!(probe.starts_with(root.path()));
    assert!(probe.file_name().unwrap().to_string_lossy().starts_with(".vibewindow_doctor_probe_"));
}

// 损坏或缺失的外部状态应转换成诊断项，而不是中断后续检查。
#[test]
fn check_workspace_reports_missing_directory_and_stops() {
    let temp = tempfile::tempdir().expect("tempdir");
    let missing = temp.path().join("missing");
    let config = config_for_workspace(missing.clone());
    let mut items = Vec::<DiagItem>::new();

    check_workspace_with_disk_probe(&config, &mut items, |_| {
        panic!("missing workspace must stop before disk probe")
    });

    assert_eq!(items.len(), 1);
    assert_eq!(items[0].severity, Severity::Error);
    assert!(items[0].message.contains(&missing.display().to_string()));
}

#[test]
fn check_workspace_reports_file_path_as_not_writable_workspace() {
    let temp = tempfile::tempdir().expect("tempdir");
    let file_workspace = temp.path().join("workspace-file");
    std::fs::write(&file_workspace, "not a directory").expect("write file workspace");
    let config = config_for_workspace(file_workspace);
    let mut items = Vec::<DiagItem>::new();

    check_workspace_with_disk_probe(&config, &mut items, |_| None);

    assert!(items.iter().any(|item| {
        item.severity == Severity::Ok && item.message.starts_with("directory exists:")
    }));
    assert!(items.iter().any(|item| {
        item.severity == Severity::Error && item.message.starts_with("directory is not writable:")
    }));
}

#[test]
fn check_workspace_reports_writable_directory_disk_and_optional_docs() {
    let temp = tempfile::tempdir().expect("tempdir");
    std::fs::write(temp.path().join("SOUL.md"), "soul").expect("write soul");
    std::fs::write(temp.path().join("AGENTS.md"), "agents").expect("write agents");
    let config = config_for_workspace(temp.path().to_path_buf());
    let mut items = Vec::<DiagItem>::new();

    check_workspace_with_disk_probe(&config, &mut items, |_| Some(100));

    assert!(items.iter().any(|item| {
        item.severity == Severity::Ok && item.message.starts_with("directory exists:")
    }));
    assert!(
        items.iter().any(|item| {
            item.severity == Severity::Ok && item.message == "directory is writable"
        })
    );
    assert!(items.iter().any(|item| {
        item.severity == Severity::Ok && item.message == "disk space: 100 MB available"
    }));
    assert!(
        items.iter().any(|item| item.severity == Severity::Ok && item.message == "SOUL.md present")
    );
    assert!(
        items
            .iter()
            .any(|item| { item.severity == Severity::Ok && item.message == "AGENTS.md present" })
    );
    assert!(std::fs::read_dir(temp.path()).expect("read tempdir").flatten().all(|entry| {
        !entry.file_name().to_string_lossy().starts_with(".vibewindow_doctor_probe_")
    }));
}

#[test]
fn check_workspace_warns_for_optional_docs_missing_from_writable_directory() {
    let temp = tempfile::tempdir().expect("tempdir");
    let config = config_for_workspace(temp.path().to_path_buf());
    let mut items = Vec::<DiagItem>::new();

    check_workspace_with_disk_probe(&config, &mut items, |_| None);

    assert!(items.iter().any(|item| {
        item.severity == Severity::Warn && item.message == "SOUL.md not found (optional)"
    }));
    assert!(items.iter().any(|item| {
        item.severity == Severity::Warn && item.message == "AGENTS.md not found (optional)"
    }));
    assert!(items.iter().all(|item| !item.message.starts_with("disk space:")));
}

#[test]
fn check_file_exists_reports_optional_and_required_missing_files() {
    let temp = tempfile::tempdir().expect("tempdir");
    let mut items = Vec::<DiagItem>::new();

    check_file_exists(temp.path(), "optional.md", false, "workspace", &mut items);
    check_file_exists(temp.path(), "required.md", true, "workspace", &mut items);

    assert_eq!(items.len(), 2);
    assert_eq!(items[0].severity, Severity::Warn);
    assert_eq!(items[0].message, "optional.md not found (optional)");
    assert_eq!(items[1].severity, Severity::Error);
    assert_eq!(items[1].message, "required.md missing");
}

#[test]
fn check_file_exists_reports_present_file_with_requested_category() {
    let temp = tempfile::tempdir().expect("tempdir");
    std::fs::write(temp.path().join("README.md"), "docs").expect("write readme");
    let mut items = Vec::<DiagItem>::new();

    check_file_exists(temp.path(), "README.md", true, "project", &mut items);

    assert_eq!(items.len(), 1);
    assert_eq!(items[0].severity, Severity::Ok);
    assert_eq!(items[0].category, "project");
    assert_eq!(items[0].message, "README.md present");
}

#[test]
fn append_disk_space_check_ignores_unavailable_measurement() {
    let mut items = Vec::<DiagItem>::new();

    append_disk_space_check(None, "workspace", &mut items);

    assert!(items.is_empty());
}

#[test]
fn append_disk_space_check_warns_below_minimum() {
    let mut items = Vec::<DiagItem>::new();

    append_disk_space_check(Some(99), "workspace", &mut items);

    assert_eq!(items.len(), 1);
    assert_eq!(items[0].severity, Severity::Warn);
    assert_eq!(items[0].category, "workspace");
    assert_eq!(items[0].message, "low disk space: only 99 MB available");
}

#[test]
fn append_disk_space_check_accepts_minimum() {
    let mut items = Vec::<DiagItem>::new();

    append_disk_space_check(Some(100), "workspace", &mut items);

    assert_eq!(items.len(), 1);
    assert_eq!(items[0].severity, Severity::Ok);
    assert_eq!(items[0].category, "workspace");
    assert_eq!(items[0].message, "disk space: 100 MB available");
}
