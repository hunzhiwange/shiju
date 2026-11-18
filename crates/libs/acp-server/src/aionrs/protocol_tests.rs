use super::protocol::Frame;
#[test]
fn aionrs_ready_requires_approval_capabilities() {
    for (version, approval, mode, expected) in [
        ("0.2.11", true, "default", true),
        ("0.2.0", true, "default", false),
        ("0.2.11", false, "default", false),
        ("0.2.11", true, "yolo", false),
    ] {
        let frame = serde_json::from_value(serde_json::json!({
            "type": "ready", "version": version,
            "capabilities": {"tool_approval": approval, "current_mode": mode}
        }))
        .unwrap();
        assert_eq!(super::process::reviewed_ready(frame), expected);
    }
}
#[test]
fn aionrs_tool_request_retains_turn_and_call_identity() {
    let frame: Frame = serde_json::from_str(r#"{"type":"tool_request","msg_id":"m1","call_id":"t1","tool":{"name":"Write","args":{"secret":"redacted"}}}"#).unwrap();
    assert_eq!(frame.msg_id.as_deref(), Some("m1"));
    assert_eq!(frame.call_id.as_deref(), Some("t1"));
    assert_eq!(frame.tool.unwrap().name, "Write");
}

#[test]
fn aionrs_workspace_tool_rejects_unsafe_paths_and_process_tools() {
    use super::backend::workspace_tool;
    use serde_json::json;

    let root = tempfile::tempdir().unwrap();
    let workspace = root.path().canonicalize().unwrap().join("workspace");
    std::fs::create_dir(&workspace).unwrap();
    assert!(workspace_tool(&workspace, "Write", &json!({"file_path":"src/new.ts"})));
    assert!(workspace_tool(&workspace, "Glob", &json!({"pattern":"**/*.ts"})));
    assert!(!workspace_tool(&workspace, "Write", &json!({"file_path":"../outside"})));
    assert!(!workspace_tool(&workspace, "Glob", &json!({"pattern":"/etc/*"})));
    assert!(!workspace_tool(&workspace, "Glob", &json!({"pattern":"../*"})));
    assert!(!workspace_tool(&workspace, "Grep", &json!({"pattern":"example"})));
    #[cfg(unix)]
    {
        std::os::unix::fs::symlink(root.path(), workspace.join("outside")).unwrap();
        assert!(!workspace_tool(&workspace, "Write", &json!({"file_path":"outside/new.ts"})));
    }
}
