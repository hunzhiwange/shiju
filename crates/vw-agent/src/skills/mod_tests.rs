//! Host CLI forwarding boundary.
use super::*;

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
