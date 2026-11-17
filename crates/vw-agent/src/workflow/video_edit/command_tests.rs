use super::prepare::NativeVideoContext;
use super::*;
use crate::tools::ToolRuntimeContext;

#[cfg(unix)]
#[test]
fn video_edit_command_symlink_obeys_native_security_and_full_access() {
    let root = tempfile::tempdir().unwrap();
    let external = tempfile::tempdir().unwrap();
    let file = external.path().join("source.mp4");
    std::fs::write(&file, b"probe later").unwrap();
    std::os::unix::fs::symlink(&file, root.path().join("link.mp4")).unwrap();
    let ctx = ToolRuntimeContext::new("video", Some(root.path().to_string_lossy().into()));
    let mut config = crate::app::agent::config::Config::default();
    config.autonomy.workspace_only = true;
    let ctx = ctx.with_test_config(config);
    assert!(
        prepare_video_edit_request("/shiju 剪辑 link.mp4", &NativeVideoContext(ctx.clone()))
            .is_err()
    );
    let full = ctx.clone().with_tool_use_context(
        ctx.tool_use_context().as_ref().clone().with_full_access_enabled(true),
    );
    assert_eq!(
        prepare_video_edit_request("/shiju 剪辑 link.mp4", &NativeVideoContext(full.clone()))
            .unwrap()
            .unwrap()
            .video_path,
        file.canonicalize().unwrap()
    );
}

#[test]
fn video_edit_command_external_allowed_root_and_no_project() {
    let root = tempfile::tempdir().unwrap();
    let external = tempfile::tempdir().unwrap();
    let file = external.path().join("source.mp4");
    std::fs::write(&file, b"cloud probe pending").unwrap();
    let mut config = crate::app::agent::config::Config::default();
    config.workspace_dir = root.path().to_path_buf();
    config.autonomy.workspace_only = true;
    config.autonomy.allowed_roots = vec![external.path().to_string_lossy().into_owned()];
    let ctx = ToolRuntimeContext::new("video", Some(root.path().to_string_lossy().into()))
        .with_test_config(config);
    let query = format!("/shiju 剪辑 '{}'", file.display());
    assert!(
        prepare_video_edit_request(&query, &NativeVideoContext(ctx.clone())).unwrap().is_some()
    );
    let full = ToolRuntimeContext::new("video", None).with_tool_use_context(
        crate::tools::ToolUseContext::new("video", None).with_full_access_enabled(true),
    );
    let req =
        prepare_video_edit_request(&query, &NativeVideoContext(full.clone())).unwrap().unwrap();
    assert_eq!(req.project_root, None);
    assert!(
        prepare_video_edit_request("/shiju 剪辑 relative.mp4", &NativeVideoContext(full.clone()))
            .is_err()
    );
}

#[test]
fn video_edit_command_delegated_parent_remains_restricted_under_full_access() {
    use std::sync::Arc;
    let root = tempfile::tempdir().unwrap();
    let external = tempfile::tempdir().unwrap();
    let file = external.path().join("source.mp4");
    std::fs::write(&file, b"probe later").unwrap();
    let mut config = crate::app::agent::config::Config::default();
    config.autonomy.workspace_only = true;
    let policy = crate::app::agent::security::SecurityPolicy::from_configs(
        &config.autonomy,
        &config.security,
        root.path(),
    );
    let base =
        crate::tools::ToolUseContext::new("video", Some(root.path().to_string_lossy().into()))
            .with_security(Arc::new(policy))
            .for_child("child".into(), vec![])
            .with_full_access_enabled(true);
    let ctx = ToolRuntimeContext::new("child", Some(root.path().to_string_lossy().into()))
        .with_tool_use_context(base)
        .with_test_config(config);
    assert!(
        prepare_video_edit_request(
            &format!("/shiju 剪辑 '{}'", file.display()),
            &NativeVideoContext(ctx.clone())
        )
        .is_err()
    );
}
