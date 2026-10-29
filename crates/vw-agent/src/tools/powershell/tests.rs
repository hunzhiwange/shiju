use super::*;
use crate::security::AutonomyLevel;

#[test]
fn powershell_tool_preserves_identity_and_cwd_schema() {
    let tool = PowerShellTool::new(Arc::new(SecurityPolicy::default()));
    let spec = tool.spec();
    assert_eq!(spec.id, "PowerShell");
    assert!(spec.aliases.iter().any(|alias| alias == "powershell"));
    assert!(spec.input_schema["properties"].get("cwd").is_some());
    assert!(
        spec.input_schema["properties"]["command"]["description"]
            .as_str()
            .unwrap()
            .contains("PowerShell")
    );
}

#[tokio::test]
async fn powershell_tool_keeps_readonly_rejection_before_spawn() {
    let tool = PowerShellTool::new(Arc::new(SecurityPolicy {
        autonomy: AutonomyLevel::ReadOnly,
        ..SecurityPolicy::default()
    }));
    let result = tool
        .call(json!({"command": "Write-Output 'blocked'", "description": "verify readonly guard"}))
        .await
        .unwrap();
    assert!(!result.is_success());
    assert!(result.data["error"].as_str().unwrap().contains("read-only"));
    assert!(result.data.get("process_id").is_none());
}

#[tokio::test]
async fn powershell_tool_uses_managed_execution_and_decodes_chinese() {
    let root = tempfile::tempdir().unwrap();
    let tool = PowerShellTool::new(Arc::new(SecurityPolicy {
        autonomy: AutonomyLevel::Full,
        workspace_only: false,
        workspace_dir: root.path().to_path_buf(),
        allowed_commands: vec!["Write-Output".into()],
        ..SecurityPolicy::default()
    }));
    let context = Arc::new(crate::tools::context::ToolUseContext::new(
        uuid::Uuid::new_v4().to_string(),
        Some(root.path().to_string_lossy().to_string()),
    ));
    let result = crate::tools::scope_tool_use_context(
        context,
        tool.call(json!({
            "command": "Write-Output '中文输出🙂'",
            "description": "verify managed PowerShell and Unicode output",
            "approved": true,
        })),
    )
    .await
    .unwrap();
    assert!(result.is_success(), "{:?}", result.data);
    assert_eq!(result.data["state"], "exited");
    assert_eq!(result.data["exit_code"], 0);
    assert!(result.data["process_id"].is_string());
    assert!(result.data["output"].as_str().unwrap().contains("中文输出🙂"));
    assert_eq!(result.render_hint.unwrap().title.as_deref(), Some("PowerShell"));
}
