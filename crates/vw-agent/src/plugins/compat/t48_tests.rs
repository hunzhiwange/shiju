use crate::plugins::install_plugin_from_source;
#[test]
fn codex_plugin_compat_t48_rejects_invalid_input() {
    let out = tempfile::tempdir().unwrap();
    assert!(install_plugin_from_source("git+/missing", out.path()).is_err());
    assert_eq!(std::fs::read_dir(out.path()).unwrap().count(), 0);
}
#[test]
fn codex_plugin_compat_t48_installs_pinned_git_revision() {
    let source = tempfile::tempdir().unwrap();
    let repo = git2::Repository::init(source.path()).unwrap();
    std::fs::create_dir(source.path().join(".codex-plugin")).unwrap();
    std::fs::write(
        source.path().join(".codex-plugin/plugin.json"),
        r#"{"name":"fixture","version":"1"}"#,
    )
    .unwrap();
    std::fs::create_dir_all(source.path().join("nested/.codex-plugin")).unwrap();
    std::fs::copy(
        source.path().join(".codex-plugin/plugin.json"),
        source.path().join("nested/.codex-plugin/plugin.json"),
    )
    .unwrap();
    let mut index = repo.index().unwrap();
    index.add_path(std::path::Path::new("nested/.codex-plugin/plugin.json")).unwrap();
    index.add_path(std::path::Path::new(".codex-plugin/plugin.json")).unwrap();
    let tree = index.write_tree().unwrap();
    let signature = git2::Signature::now("fixture", "fixture@example.invalid").unwrap();
    let oid = repo
        .commit(
            Some("HEAD"),
            &signature,
            &signature,
            "fixture",
            &repo.find_tree(tree).unwrap(),
            &[],
        )
        .unwrap();
    let out = tempfile::tempdir().unwrap();
    let installed =
        install_plugin_from_source(&format!("git+{}#{oid}", source.path().display()), out.path())
            .unwrap();
    assert_eq!(installed.manifest.name, "fixture");
    assert!(!installed.installed_dir.join(".git").exists());
    let manifest =
        std::fs::read(installed.installed_dir.join(".codex-plugin/plugin.json")).unwrap();
    assert_eq!(manifest, std::fs::read(source.path().join(".codex-plugin/plugin.json")).unwrap());
    let metadata =
        super::identity::read_install_metadata(&installed.installed_dir, &installed.manifest)
            .unwrap()
            .unwrap();
    let selected_out = tempfile::tempdir().unwrap();
    let selected = super::git_source::install(
        source.path().to_str().unwrap(),
        Some("nested"),
        None,
        Some(&oid.to_string()),
        selected_out.path(),
        Some("catalog"),
    )
    .unwrap();
    assert_eq!(selected.identity.as_key(), "fixture@catalog");
    assert!(!selected.installed_dir.join("nested").exists());
    #[cfg(unix)]
    {
        std::os::unix::fs::symlink("/tmp", source.path().join("nested/escape")).unwrap();
        index.add_path(std::path::Path::new("nested/escape")).unwrap();
        let bad_tree = index.write_tree().unwrap();
        let bad = repo
            .commit(
                Some("HEAD"),
                &signature,
                &signature,
                "symlink",
                &repo.find_tree(bad_tree).unwrap(),
                &[&repo.find_commit(oid).unwrap()],
            )
            .unwrap();
        let rejected = tempfile::tempdir().unwrap();
        assert!(
            super::git_source::install(
                source.path().to_str().unwrap(),
                Some("nested"),
                None,
                Some(&bad.to_string()),
                rejected.path(),
                None
            )
            .unwrap_err()
            .to_string()
            .contains("symlink")
        );
        assert_eq!(std::fs::read_dir(rejected.path()).unwrap().count(), 0);
    }
    assert!(metadata.source.contains(&oid.to_string()));
    assert!(!metadata.source.contains("payload"));
    assert!(
        install_plugin_from_source(&format!("git+{}#missing", source.path().display()), out.path())
            .is_err()
    );
}
#[test]
fn codex_plugin_compat_t48_preserves_isolation() {
    let out = tempfile::tempdir().unwrap();
    assert!(
        super::git_source::install(
            "/missing",
            Some("../escape"),
            Some("HEAD"),
            None,
            out.path(),
            None
        )
        .is_err()
    );
    assert_eq!(std::fs::read_dir(out.path()).unwrap().count(), 0);
}
