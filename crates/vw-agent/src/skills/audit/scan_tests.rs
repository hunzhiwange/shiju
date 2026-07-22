use super::*;

#[test]
fn depth_first_collection_is_deterministic() {
    let dir = tempfile::tempdir().expect("temp dir");
    std::fs::create_dir(dir.path().join("b")).unwrap();
    std::fs::create_dir(dir.path().join("a")).unwrap();
    std::fs::write(dir.path().join("a").join("SKILL.md"), "# A").unwrap();

    let names = collect_paths_depth_first(dir.path())
        .unwrap()
        .into_iter()
        .filter_map(|path| path.file_name().and_then(|name| name.to_str()).map(str::to_string))
        .collect::<Vec<_>>();

    assert_eq!(names[1], "a");
    assert_eq!(names[2], "SKILL.md");
    assert_eq!(names[3], "b");
}

#[test]
fn audit_path_flags_script_symlink_and_oversized_text_without_reading() {
    let dir = tempfile::tempdir().expect("temp dir");
    let script = dir.path().join("run.sh");
    let large = dir.path().join("SKILL.md");
    std::fs::write(&script, "echo hi\n").unwrap();
    std::fs::write(&large, "x".repeat(MAX_TEXT_FILE_BYTES as usize + 1)).unwrap();

    #[cfg(unix)]
    {
        std::os::unix::fs::symlink(&script, dir.path().join("link")).unwrap();
    }

    let mut report = crate::app::agent::skills::audit::report::SkillAuditReport::default();
    audit_path(dir.path(), &script, &mut report).unwrap();
    audit_path(dir.path(), &large, &mut report).unwrap();
    #[cfg(unix)]
    audit_path(dir.path(), &dir.path().join("link"), &mut report).unwrap();

    let summary = report.summary();
    assert!(summary.contains("script-like files are blocked"));
    assert!(summary.contains("file is too large for static audit"));
    #[cfg(unix)]
    assert!(summary.contains("symlinks are not allowed"));
}

#[test]
fn audit_path_applies_stricter_link_checks_only_to_skill_entry_markdown() {
    let dir = tempfile::tempdir().expect("temp dir");
    let skill = dir.path().join("SKILL.md");
    let resource = dir.path().join("guide.md");
    std::fs::write(&skill, "[missing](docs/missing.md)\n").unwrap();
    std::fs::write(&resource, "[also missing](docs/also-missing.md)\n").unwrap();

    let mut report = crate::app::agent::skills::audit::report::SkillAuditReport::default();
    audit_path(dir.path(), &skill, &mut report).unwrap();
    audit_path(dir.path(), &resource, &mut report).unwrap();

    assert_eq!(report.summary().matches("missing file").count(), 1);
    assert!(is_skill_entry_markdown(&std::path::PathBuf::from("skill.MD")));
}

#[test]
fn audit_path_reports_missing_path_metadata_error() {
    let dir = tempfile::tempdir().expect("temp dir");
    let missing = dir.path().join("missing.md");
    let mut report = crate::app::agent::skills::audit::report::SkillAuditReport::default();

    let error = audit_path(dir.path(), &missing, &mut report).unwrap_err();

    assert!(error.to_string().contains("failed to read metadata"));
    assert!(error.to_string().contains("missing.md"));
    assert!(report.findings.is_empty());
}
