use super::env_diagnose::diagnose_at;
use vw_api_types::{agent_apps::AppId, app_providers::env_diagnose::*};
#[test]
fn cc04_env_diagnose_reports_shell_origin() {
    let home = tempfile::tempdir().unwrap();
    let system = tempfile::tempdir().unwrap();
    let shell = home.path().join(".zshrc");
    let contents = "# fixture\nexport ANTHROPIC_API_KEY='fixture-secret'\nANTHROPIC_BASE_URL=https://example.invalid\n# ANTHROPIC_IGNORED=no\nMY_ANTHROPIC_KEY=unrelated\n";
    std::fs::write(&shell, contents).unwrap();
    let report = diagnose_at(home.path(), system.path(), AppId::Claude, []).unwrap();
    assert_eq!(report.entries.len(), 2);
    assert_eq!(report.entries[0].variable, "ANTHROPIC_API_KEY");
    assert_eq!(
        report.entries[0].source,
        EnvSource::UserShell { file: UserShellFile::Zshrc, line: 2 }
    );
    assert!(report.entries[0].cleanup_available);
    assert!(!serde_json::to_string(&report).unwrap().contains("fixture-secret"));
    assert!(!format!("{report:?}").contains("example.invalid"));
    assert_eq!(std::fs::read_to_string(shell).unwrap(), contents);
}
#[test]
fn cc04_env_diagnose_inherited_env_is_not_deletable() {
    let home = tempfile::tempdir().unwrap();
    let system = tempfile::tempdir().unwrap();
    std::fs::write(home.path().join(".profile"), "OPENAI_API_KEY=shell-secret\n").unwrap();
    std::fs::write(system.path().join("profile"), "OPENAI_API_KEY=system-secret\n").unwrap();
    let report =
        diagnose_at(home.path(), system.path(), AppId::Codex, ["OPENAI_API_KEY".into()]).unwrap();
    assert_eq!(report.entries.len(), 3);
    assert!(
        report
            .entries
            .iter()
            .any(|e| matches!(e.source, EnvSource::UserShell { .. }) && e.cleanup_available)
    );
    for source in [
        EnvSource::ProcessInherited,
        EnvSource::SystemShell { file: SystemShellFile::Profile, line: 1 },
    ] {
        let entry = report.entries.iter().find(|e| e.source == source).unwrap();
        assert!(!entry.cleanup_available);
    }
}

#[test]
fn cc04_env_diagnose_complex_shell_is_read_only_and_never_executed() {
    let home = tempfile::tempdir().unwrap();
    let system = tempfile::tempdir().unwrap();
    let marker = home.path().join("must-not-exist");
    let contents = format!(
        "cat <<'EOF'\nANTHROPIC_EXAMPLE=not-an-effective-definition\nEOF\nexport ANTHROPIC_API_KEY=$(touch {})\n",
        marker.display()
    );
    std::fs::write(home.path().join(".bashrc"), &contents).unwrap();
    let report = diagnose_at(home.path(), system.path(), AppId::Claude, []).unwrap();
    assert_eq!(report.entries.len(), 2);
    assert!(report.entries.iter().all(|entry| !entry.cleanup_available));
    assert!(!marker.exists());
    assert_eq!(std::fs::read_to_string(home.path().join(".bashrc")).unwrap(), contents);
}

#[test]
fn cc04_env_diagnose_rejects_links_devices_and_oversized_sources() {
    use std::os::unix::fs::symlink;
    let home = tempfile::tempdir().unwrap();
    let system = tempfile::tempdir().unwrap();
    let outside = tempfile::NamedTempFile::new().unwrap();
    std::fs::write(outside.path(), "ANTHROPIC_API_KEY=outside-secret").unwrap();
    let path = home.path().join(".zshrc");
    symlink(outside.path(), &path).unwrap();
    assert_eq!(
        diagnose_at(home.path(), system.path(), AppId::Claude, []).unwrap_err(),
        "environment source unavailable"
    );
    std::fs::remove_file(&path).unwrap();
    std::fs::create_dir(&path).unwrap();
    assert!(diagnose_at(home.path(), system.path(), AppId::Claude, []).is_err());
    std::fs::remove_dir(&path).unwrap();
    std::fs::write(&path, vec![b'x'; 1024 * 1024 + 1]).unwrap();
    assert!(diagnose_at(home.path(), system.path(), AppId::Claude, []).is_err());
    assert_eq!(
        std::fs::read_to_string(outside.path()).unwrap(),
        "ANTHROPIC_API_KEY=outside-secret"
    );
}

#[test]
fn cc04_env_diagnose_matches_upstream_keywords_and_rejects_unsupported_apps() {
    let home = tempfile::tempdir().unwrap();
    let system = tempfile::tempdir().unwrap();
    std::fs::write(home.path().join(".zprofile"), "export XAI_API_KEY='fake'\nGROK_HOME=keep\nXAI_API_KEY_BACKUP=keep\nexport GROK_DEFAULT_MODEL=grok\n").unwrap();
    let report = diagnose_at(
        home.path(),
        system.path(),
        AppId::GrokBuild,
        ["GROK_BIN_DIR".into(), "xai_api_key".into(), "xai_api_key".into()],
    )
    .unwrap();
    assert_eq!(report.entries.len(), 3);
    assert_eq!(report.entries[0].variable, "XAI_API_KEY");
    assert_eq!(report.entries[1].variable, "GROK_DEFAULT_MODEL");
    assert!(report.entries[..2].iter().all(|entry| entry.cleanup_available));
    assert!(!report.entries[2].cleanup_available);
    let report = diagnose_at(
        home.path(),
        system.path(),
        AppId::Gemini,
        ["GOOGLE_GEMINI_API_KEY".into(), "GEMINI_API_KEY".into(), "MY_GEMINI_API_KEY".into()],
    )
    .unwrap();
    assert_eq!(report.entries.len(), 2);
    assert_eq!(
        diagnose_at(home.path(), system.path(), AppId::Omp, []).unwrap_err(),
        "unsupported environment application"
    );
}

#[test]
fn cc04_env_diagnose_execution_process_is_read_only() {
    const PROBE: &str = "ANTHROPIC_SHIJU_INHERITED_PROBE";
    if std::env::var_os(PROBE).is_some() {
        let home = tempfile::tempdir().unwrap();
        let report = super::env_diagnose::diagnose(home.path(), AppId::Claude).unwrap();
        let entry = report.entries.iter().find(|entry| entry.variable == PROBE).unwrap();
        assert_eq!(entry.source, EnvSource::ProcessInherited);
        assert!(!entry.cleanup_available);
        assert!(!serde_json::to_string(&report).unwrap().contains("fixture-inherited-secret"));
        assert_eq!(std::env::var(PROBE).unwrap(), "fixture-inherited-secret");
        assert_eq!(std::fs::read_dir(home.path()).unwrap().count(), 0);
        return;
    }
    let output = std::process::Command::new(std::env::current_exe().unwrap())
        .args([
            "--exact",
            "providers::env_diagnose_tests::cc04_env_diagnose_execution_process_is_read_only",
            "--nocapture",
        ])
        .env(PROBE, "fixture-inherited-secret")
        .output()
        .unwrap();
    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
    assert!(String::from_utf8_lossy(&output.stdout).contains("1 passed"));
    assert!(std::env::var_os(PROBE).is_none());
}
