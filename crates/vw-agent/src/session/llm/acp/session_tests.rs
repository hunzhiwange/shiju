use super::*;
use acp_types::PermissionMode;
fn acp_config(command: &str) -> config::schema::AcpAgentConfig { config::schema::AcpAgentConfig { command: command.into(), ..Default::default() } }
#[test]
fn acp_desktop_approval_policy_binds_current_session_and_isolates_cached_grants() {
    let root = tempfile::tempdir().unwrap();
    let foreign = tempfile::tempdir().unwrap();
    let context = crate::tools::ToolUseContext::new(
        "current",
        Some(root.path().to_string_lossy().into_owned()),
    )
    .with_channel("desktop");
    let requested = ParsedAcpOptions {
        permission_mode: Some(PermissionMode::ApproveAll),
        full_access: true,
        ..Default::default()
    };
    assert!(desktop_workspace(&context, "foreign", Some(root.path())).is_err());
    assert!(desktop_workspace(&context, "current", Some(foreign.path())).is_err());
    let canonical = root.path().canonicalize().unwrap();
    let (cwd, workspace) = desktop_workspace(&context, "current", Some(&canonical)).unwrap();
    assert_eq!(cwd, root.path());
    assert_eq!(workspace, canonical);

    let mut restricted = requested.clone();
    bind_desktop_access(&mut restricted, workspace.clone(), context.full_access_enabled());
    assert!(!restricted.full_access);
    assert_eq!(restricted.permission_mode, Some(PermissionMode::ApproveReads));
    assert_eq!(restricted.desktop_workspace, Some(workspace.clone()));

    let mut granted = requested.clone();
    let full = context.clone().with_full_access_enabled(true);
    bind_desktop_access(&mut granted, workspace.clone(), full.full_access_enabled());
    assert!(granted.full_access);
    let config = acp_config("claude");
    let restricted_key = acp_client_cache_key("claude", &config, root.path(), &restricted);
    assert_ne!(restricted_key, acp_client_cache_key("claude", &config, root.path(), &granted));
    bind_desktop_access(&mut granted, workspace.clone(), context.full_access_enabled());
    assert_eq!(restricted_key, acp_client_cache_key("claude", &config, root.path(), &granted));

    let mut denied = requested;
    denied.permission_mode = Some(PermissionMode::DenyAll);
    bind_desktop_access(&mut denied, workspace, full.full_access_enabled());
    assert_eq!(denied.permission_mode, Some(PermissionMode::DenyAll));
    assert!(!denied.full_access);
}
