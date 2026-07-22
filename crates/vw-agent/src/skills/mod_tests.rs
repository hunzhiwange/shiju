use super::*;

#[test]
fn discover_default_sources_includes_legacy_workspace_directory() {
    let workspace = tempfile::tempdir().expect("temp dir");
    let legacy = workspace.path().join("skills");
    std::fs::create_dir_all(&legacy).unwrap();
    let legacy = legacy.canonicalize().expect("legacy skills dir should canonicalize");

    let sources = discover_local_skill_source_dirs(Some(workspace.path()));

    assert!(
        sources.iter().any(|source| {
            source.kind == LocalSkillSourceKind::Workspace && source.path == legacy
        })
    );
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn handle_command_forwards_remove_validation_errors() {
    let workspace = tempfile::tempdir().expect("temp dir");
    let mut config = crate::app::agent::config::Config::default();
    config.workspace_dir = workspace.path().to_path_buf();

    let error = handle_command(
        crate::app::agent::skill::SkillCommands::Remove { name: "../escape".into() },
        &config,
    )
    .unwrap_err();

    assert!(error.to_string().contains("Invalid skill name"));
}
