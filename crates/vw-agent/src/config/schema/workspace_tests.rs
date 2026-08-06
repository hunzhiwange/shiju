use super::workspace::{
    ConfigResolutionSource, clear_active_workspace_marker, default_config_and_workspace_dirs,
    default_config_dir, is_temp_directory, persist_active_workspace_config_dir_with_default,
    persist_active_workspace_marker, resolve_config_dir_for_workspace,
    resolve_runtime_config_dirs_from_env, sync_directory,
};
use std::path::{Path, PathBuf};

fn marker_path(root: &Path) -> PathBuf {
    root.join("active_workspace.toml")
}

#[test]
fn workspace_config_resolution_returns_config_and_workspace_dirs() {
    let workspace = std::path::Path::new("/tmp/vw-workspace");
    let (config_dir, workspace_dir) = resolve_config_dir_for_workspace(workspace);

    assert_eq!(config_dir, workspace);
    assert_eq!(workspace_dir, workspace.join("workspace"));
}

#[test]
fn workspace_resolution_detects_config_json_file() {
    let tmp = tempfile::tempdir().unwrap();
    let config_dir = tmp.path().join("project");
    std::fs::create_dir_all(&config_dir).unwrap();
    std::fs::write(config_dir.join("config.json"), "{}").unwrap();

    let (resolved_config, resolved_workspace) = resolve_config_dir_for_workspace(&config_dir);
    assert_eq!(resolved_config, config_dir);
    assert_eq!(resolved_workspace, resolved_config.join("workspace"));
}

#[test]
fn workspace_resolution_ignores_removed_project_config_override_file() {
    let tmp = tempfile::tempdir().unwrap();
    let workspace = tmp.path().join("workspace");
    let legacy = tmp.path().join(".vibewindow");
    std::fs::create_dir_all(&workspace).unwrap();
    std::fs::create_dir_all(&legacy).unwrap();
    std::fs::write(workspace.join(".vibewindow.json"), "{}").unwrap();
    std::fs::write(legacy.join("config.json"), "{}").unwrap();

    let (resolved_config, resolved_workspace) = resolve_config_dir_for_workspace(&workspace);
    assert_eq!(resolved_config, legacy);
    assert_eq!(resolved_workspace, workspace);
}

#[test]
fn workspace_resolution_detects_legacy_config_json_layout() {
    let tmp = tempfile::tempdir().unwrap();
    let workspace = tmp.path().join("workspace");
    let legacy = tmp.path().join(".vibewindow");
    std::fs::create_dir_all(&workspace).unwrap();
    std::fs::create_dir_all(&legacy).unwrap();
    std::fs::write(legacy.join("config.json"), "{}").unwrap();

    let (resolved_config, resolved_workspace) = resolve_config_dir_for_workspace(&workspace);
    assert_eq!(resolved_config, legacy);
    assert_eq!(resolved_workspace, workspace);
}

#[test]
fn workspace_named_workspace_uses_legacy_layout_without_config_file() {
    let tmp = tempfile::tempdir().unwrap();
    let workspace = tmp.path().join("workspace");
    std::fs::create_dir_all(&workspace).unwrap();

    let (resolved_config, resolved_workspace) = resolve_config_dir_for_workspace(&workspace);
    assert_eq!(resolved_config, tmp.path().join(".vibewindow"));
    assert_eq!(resolved_workspace, workspace);
}

#[test]
fn workspace_resolution_handles_empty_path_without_parent() {
    let workspace = Path::new("");

    let (resolved_config, resolved_workspace) = resolve_config_dir_for_workspace(workspace);

    assert_eq!(resolved_config, PathBuf::new());
    assert_eq!(resolved_workspace, PathBuf::from("workspace"));
}

#[test]
fn default_config_dir_can_be_resolved() {
    assert!(default_config_dir().is_ok());
}

#[test]
fn default_config_and_workspace_dirs_returns_workspace_inside_config_dir() {
    let (config_dir, workspace_dir) = default_config_and_workspace_dirs().unwrap();

    assert_eq!(workspace_dir, config_dir.join("workspace"));
}

#[test]
fn config_resolution_source_strings_are_stable() {
    assert_eq!(ConfigResolutionSource::EnvConfigDir.as_str(), "VIBEWINDOW_CONFIG_DIR");
    assert_eq!(ConfigResolutionSource::EnvWorkspace.as_str(), "VIBEWINDOW_WORKSPACE");
    assert_eq!(ConfigResolutionSource::ActiveWorkspaceMarker.as_str(), "active_workspace.toml");
    assert_eq!(ConfigResolutionSource::DefaultConfigDir.as_str(), "default");
}

#[test]
fn temp_directory_detection_handles_existing_and_missing_paths() {
    let tmp = tempfile::tempdir().unwrap();
    assert!(is_temp_directory(tmp.path()));

    let temp = std::env::temp_dir();
    let canonical_temp = temp.canonicalize().unwrap_or(temp);
    let missing = canonical_temp.join("vw-agent-missing-temp-path");
    assert!(is_temp_directory(&missing));
}

#[cfg(unix)]
#[test]
fn temp_directory_detection_rejects_filesystem_root() {
    assert!(!is_temp_directory(Path::new("/")));
}

#[tokio::test]
async fn config_dir_env_takes_priority_over_workspace_env_and_marker() {
    let tmp = tempfile::tempdir().unwrap();
    let default_config = tmp.path().join("default-config");
    let default_workspace = default_config.join("workspace");
    let marker_config = tmp.path().join("marker-config");
    tokio::fs::create_dir_all(&default_config).await.unwrap();
    persist_active_workspace_marker(&default_config, &marker_config).await.unwrap();

    let (config_dir, workspace_dir, source) = resolve_runtime_config_dirs_from_env(
        &default_config,
        &default_workspace,
        Some("  /tmp/vw-env-config  "),
        Some("/tmp/vw-env-workspace"),
    )
    .await
    .unwrap();

    assert_eq!(config_dir, PathBuf::from("/tmp/vw-env-config"));
    assert_eq!(workspace_dir, config_dir.join("workspace"));
    assert_eq!(source, ConfigResolutionSource::EnvConfigDir);
}

#[tokio::test]
async fn workspace_env_is_used_when_config_env_is_blank() {
    let tmp = tempfile::tempdir().unwrap();
    let workspace = tmp.path().join("project");
    tokio::fs::create_dir_all(&workspace).await.unwrap();
    let default_config = tmp.path().join("default-config");
    let default_workspace = default_config.join("workspace");
    let workspace_env = workspace.to_string_lossy().into_owned();

    let (config_dir, workspace_dir, source) = resolve_runtime_config_dirs_from_env(
        &default_config,
        &default_workspace,
        Some("   "),
        Some(workspace_env.as_str()),
    )
    .await
    .unwrap();

    assert_eq!(config_dir, workspace);
    assert_eq!(workspace_dir, config_dir.join("workspace"));
    assert_eq!(source, ConfigResolutionSource::EnvWorkspace);
}

#[tokio::test]
async fn workspace_env_takes_priority_over_active_workspace_marker() {
    let tmp = tempfile::tempdir().unwrap();
    let default_config = tmp.path().join("default-config");
    let default_workspace = default_config.join("workspace");
    let marker_config = tmp.path().join("marker-config");
    let workspace = tmp.path().join("project");
    tokio::fs::create_dir_all(&default_config).await.unwrap();
    tokio::fs::create_dir_all(&workspace).await.unwrap();
    persist_active_workspace_marker(&default_config, &marker_config).await.unwrap();
    let workspace_env = workspace.to_string_lossy().into_owned();

    let (config_dir, workspace_dir, source) = resolve_runtime_config_dirs_from_env(
        &default_config,
        &default_workspace,
        None,
        Some(workspace_env.as_str()),
    )
    .await
    .unwrap();

    assert_eq!(config_dir, workspace);
    assert_eq!(workspace_dir, config_dir.join("workspace"));
    assert_eq!(source, ConfigResolutionSource::EnvWorkspace);
}

#[tokio::test]
async fn empty_environment_values_allow_active_workspace_marker_resolution() {
    let tmp = tempfile::tempdir().unwrap();
    let default_config = tmp.path().join("default-config");
    let default_workspace = default_config.join("workspace");
    let marker_config = tmp.path().join("marker-config");
    tokio::fs::create_dir_all(&default_config).await.unwrap();
    persist_active_workspace_marker(&default_config, &marker_config).await.unwrap();

    let (config_dir, workspace_dir, source) = resolve_runtime_config_dirs_from_env(
        &default_config,
        &default_workspace,
        Some("   "),
        Some(""),
    )
    .await
    .unwrap();

    assert_eq!(config_dir, marker_config);
    assert_eq!(workspace_dir, config_dir.join("workspace"));
    assert_eq!(source, ConfigResolutionSource::ActiveWorkspaceMarker);
}

#[tokio::test]
async fn active_workspace_marker_round_trip_and_runtime_resolution_priority() {
    let tmp = tempfile::tempdir().unwrap();
    let default_config = tmp.path().join("default-config");
    let default_workspace = default_config.join("workspace");
    let selected_config = tmp.path().join("selected-config");
    tokio::fs::create_dir_all(&default_config).await.unwrap();
    tokio::fs::create_dir_all(&selected_config).await.unwrap();

    persist_active_workspace_marker(&default_config, &selected_config).await.unwrap();
    let (config_dir, workspace_dir, source) =
        resolve_runtime_config_dirs_from_env(&default_config, &default_workspace, None, None)
            .await
            .unwrap();

    assert_eq!(config_dir, selected_config);
    assert_eq!(workspace_dir, config_dir.join("workspace"));
    assert_eq!(source, ConfigResolutionSource::ActiveWorkspaceMarker);

    clear_active_workspace_marker(&default_config).await.unwrap();
    let (_, _, source) =
        resolve_runtime_config_dirs_from_env(&default_config, &default_workspace, None, None)
            .await
            .unwrap();
    assert_eq!(source, ConfigResolutionSource::DefaultConfigDir);
}

#[tokio::test]
async fn active_workspace_marker_resolves_relative_config_dir() {
    let tmp = tempfile::tempdir().unwrap();
    let default_config = tmp.path().join("default-config");
    let default_workspace = default_config.join("workspace");
    tokio::fs::create_dir_all(&default_config).await.unwrap();
    tokio::fs::write(marker_path(&default_config), "config_dir = \"relative-config\"\n")
        .await
        .unwrap();

    let (config_dir, workspace_dir, source) =
        resolve_runtime_config_dirs_from_env(&default_config, &default_workspace, None, None)
            .await
            .unwrap();

    assert_eq!(config_dir, default_config.join("relative-config"));
    assert_eq!(workspace_dir, config_dir.join("workspace"));
    assert_eq!(source, ConfigResolutionSource::ActiveWorkspaceMarker);
}

#[tokio::test]
async fn active_workspace_marker_trims_absolute_config_dir() {
    let tmp = tempfile::tempdir().unwrap();
    let default_config = tmp.path().join("default-config");
    let default_workspace = default_config.join("workspace");
    let selected_config = tmp.path().join("selected-config");
    tokio::fs::create_dir_all(&default_config).await.unwrap();
    tokio::fs::write(
        marker_path(&default_config),
        format!("config_dir = \"  {}  \"\n", selected_config.display()),
    )
    .await
    .unwrap();

    let (config_dir, workspace_dir, source) =
        resolve_runtime_config_dirs_from_env(&default_config, &default_workspace, None, None)
            .await
            .unwrap();

    assert_eq!(config_dir, selected_config);
    assert_eq!(workspace_dir, selected_config.join("workspace"));
    assert_eq!(source, ConfigResolutionSource::ActiveWorkspaceMarker);
}

#[tokio::test]
async fn invalid_active_workspace_markers_fall_back_to_default_dirs() {
    let tmp = tempfile::tempdir().unwrap();
    let default_config = tmp.path().join("default-config");
    let default_workspace = default_config.join("workspace");
    tokio::fs::create_dir_all(&default_config).await.unwrap();

    for contents in ["{", "config_dir = \"   \"\n"] {
        tokio::fs::write(marker_path(&default_config), contents).await.unwrap();
        let (config_dir, workspace_dir, source) =
            resolve_runtime_config_dirs_from_env(&default_config, &default_workspace, None, None)
                .await
                .unwrap();

        assert_eq!(config_dir, default_config);
        assert_eq!(workspace_dir, default_workspace);
        assert_eq!(source, ConfigResolutionSource::DefaultConfigDir);
    }
}

#[tokio::test]
async fn unreadable_active_workspace_marker_falls_back_to_default_dirs() {
    let tmp = tempfile::tempdir().unwrap();
    let default_config = tmp.path().join("default-config");
    let default_workspace = default_config.join("workspace");
    tokio::fs::create_dir_all(marker_path(&default_config)).await.unwrap();

    let (config_dir, workspace_dir, source) =
        resolve_runtime_config_dirs_from_env(&default_config, &default_workspace, None, None)
            .await
            .unwrap();

    assert_eq!(config_dir, default_config);
    assert_eq!(workspace_dir, default_workspace);
    assert_eq!(source, ConfigResolutionSource::DefaultConfigDir);
}

#[tokio::test]
async fn persist_active_workspace_marker_fails_when_marker_root_is_file() {
    let tmp = tempfile::tempdir().unwrap();
    let marker_root = tmp.path().join("marker-root");
    tokio::fs::write(&marker_root, "").await.unwrap();
    let config_dir = tmp.path().join("config");

    let err = persist_active_workspace_marker(&marker_root, &config_dir).await.unwrap_err();

    assert!(err.to_string().contains("Failed to create active workspace marker root"));
}

#[tokio::test]
async fn persist_active_workspace_marker_cleans_temp_file_when_rename_fails() {
    let tmp = tempfile::tempdir().unwrap();
    let marker_root = tmp.path().join("marker-root");
    tokio::fs::create_dir_all(marker_path(&marker_root)).await.unwrap();
    let config_dir = tmp.path().join("config");

    let err = persist_active_workspace_marker(&marker_root, &config_dir).await.unwrap_err();
    let mut entries = tokio::fs::read_dir(&marker_root).await.unwrap();
    let mut temp_count = 0;
    while let Some(entry) = entries.next_entry().await.unwrap() {
        if entry.file_name().to_string_lossy().starts_with(".active_workspace.toml.tmp-") {
            temp_count += 1;
        }
    }

    assert!(err.to_string().contains("Failed to atomically persist active workspace marker"));
    assert_eq!(temp_count, 0);
}

#[tokio::test]
async fn clear_active_workspace_marker_errors_when_marker_is_directory() {
    let tmp = tempfile::tempdir().unwrap();
    let marker_root = tmp.path().join("marker-root");
    tokio::fs::create_dir_all(marker_path(&marker_root)).await.unwrap();

    let err = clear_active_workspace_marker(&marker_root).await.unwrap_err();

    assert!(err.to_string().contains("Failed to clear active workspace marker"));
}

#[tokio::test]
async fn clear_active_workspace_marker_is_idempotent_when_marker_is_missing() {
    let tmp = tempfile::tempdir().unwrap();
    let marker_root = tmp.path().join("marker-root");
    tokio::fs::create_dir_all(&marker_root).await.unwrap();

    clear_active_workspace_marker(&marker_root).await.unwrap();

    assert!(!marker_path(&marker_root).exists());
}

#[tokio::test]
async fn persist_active_workspace_config_dir_removes_default_marker() {
    let tmp = tempfile::tempdir().unwrap();
    let default_config = tmp.path().join("default-config");
    tokio::fs::create_dir_all(&default_config).await.unwrap();
    tokio::fs::write(marker_path(&default_config), "config_dir = \"/tmp/old\"\n").await.unwrap();

    persist_active_workspace_config_dir_with_default(&default_config, &default_config)
        .await
        .unwrap();

    assert!(!marker_path(&default_config).exists());
}

#[tokio::test]
async fn persist_active_workspace_config_dir_writes_selected_marker_even_when_mirror_fails() {
    let tmp = tempfile::tempdir().unwrap();
    let default_config = tmp.path().join("default-config");
    let selected_config = tmp.path().join("selected-config");
    tokio::fs::write(&default_config, "").await.unwrap();

    persist_active_workspace_config_dir_with_default(&selected_config, &default_config)
        .await
        .unwrap();

    assert!(marker_path(&selected_config).exists());
}

#[tokio::test]
async fn persist_active_workspace_config_dir_writes_selected_and_mirrored_markers() {
    let tmp = tempfile::tempdir().unwrap();
    let default_config = tmp.path().join("default-config");
    let selected_config = tmp.path().join("selected-config");

    persist_active_workspace_config_dir_with_default(&selected_config, &default_config)
        .await
        .unwrap();

    assert!(marker_path(&selected_config).exists());
    assert!(marker_path(&default_config).exists());
    for marker_root in [&selected_config, &default_config] {
        let default_workspace = marker_root.join("workspace");
        let (config_dir, workspace_dir, source) =
            resolve_runtime_config_dirs_from_env(marker_root, &default_workspace, None, None)
                .await
                .unwrap();

        assert_eq!(config_dir, selected_config);
        assert_eq!(workspace_dir, selected_config.join("workspace"));
        assert_eq!(source, ConfigResolutionSource::ActiveWorkspaceMarker);
    }
}

#[cfg(unix)]
#[tokio::test]
async fn sync_directory_returns_error_for_missing_directory() {
    let tmp = tempfile::tempdir().unwrap();
    let missing = tmp.path().join("missing");

    assert!(sync_directory(&missing).await.is_err());
}
