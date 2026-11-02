use crate::app::agent::security::{AutonomyLevel, SecurityPolicy};
use crate::tools::{apply_patch::ApplyPatchTool, traits::Tool};
use serde_json::json;
use std::sync::Arc;
fn tool(path: &std::path::Path) -> ApplyPatchTool {
    ApplyPatchTool::new(Arc::new(SecurityPolicy {
        workspace_dir: path.into(),
        autonomy: AutonomyLevel::Full,
        ..SecurityPolicy::default()
    }))
}
#[tokio::test]
async fn codex_plugin_compat_t60_applies_patch_with_host_permissions() {
    let dir = tempfile::tempdir().unwrap();
    let result = tool(dir.path())
        .execute(json!("*** Begin Patch\n*** Add File: exact.txt\n+hello\n*** End Patch"))
        .await
        .unwrap();
    assert!(result.success, "{}", result.output);
    assert_eq!(std::fs::read_to_string(dir.path().join("exact.txt")).unwrap(), "hello\n");
}
#[tokio::test]
async fn codex_plugin_compat_t60_rejects_invalid_input() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("a.txt"), "original\n").unwrap();
    let result=tool(dir.path()).execute(json!({"patchText":"*** Begin Patch\n*** Add File: b.txt\n+new\n*** Update File: a.txt\n@@\n-missing\n+changed\n*** End Patch"})).await.unwrap();
    assert!(!result.success);
    assert!(!dir.path().join("b.txt").exists());
    assert_eq!(std::fs::read_to_string(dir.path().join("a.txt")).unwrap(), "original\n");
}
#[tokio::test]
async fn codex_plugin_compat_t60_preserves_isolation() {
    let dir = tempfile::tempdir().unwrap();
    let other = tempfile::tempdir().unwrap();
    let patch = format!(
        "*** Begin Patch\n*** Add File: {}\n+denied\n*** End Patch",
        other.path().join("escape.txt").display()
    );
    let result = tool(dir.path()).execute(json!({"patchText":patch})).await.unwrap();
    assert!(!result.success);
    assert!(!other.path().join("escape.txt").exists());
}
