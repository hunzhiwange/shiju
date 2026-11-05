use crate::app::agent::{
    runtime::NativeRuntime,
    security::{AutonomyLevel, SecurityPolicy},
};
use crate::tools::ToolUseContext;
use crate::tools::traits::Tool;
use serde_json::json;
use std::sync::Arc;

#[tokio::test]
async fn codex_plugin_compat_t59_preserves_exec_arguments_and_cancellation() {
    let dir = tempfile::tempdir().unwrap();
    let policy = Arc::new(SecurityPolicy {
        autonomy: AutonomyLevel::Full,
        workspace_only: false,
        workspace_dir: dir.path().into(),
        allowed_commands: vec!["printf".into(), "sleep".into()],
        forbidden_paths: vec![],
        ..SecurityPolicy::default()
    });
    let tools = crate::tools::default_tools_with_runtime(policy, Arc::new(NativeRuntime::new()));
    assert!(!tools.iter().any(|tool| tool.spec().id == "exec_command"));
    assert_eq!(tools.iter().filter(|tool| tool.spec().id == "bash").count(), 1);
    let ctx =
        Arc::new(ToolUseContext::new("t59-owner", Some(dir.path().to_string_lossy().into_owned())));
    let executed = crate::tools::execute_tool_from_registry(
        &tools,
        "bash",
        json!({"command":"printf ready; sleep 60","workdir":dir.path(),"yield_time_ms":250}),
        ctx.clone(),
    )
    .await
    .unwrap();
    assert_eq!(executed.tool_name, "bash");
    let mut started = executed.result;
    let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(10);
    while started.data["output"] != "ready"
        && started.data["state"] == "running"
        && tokio::time::Instant::now() < deadline
    {
        started = crate::tools::execute_tool_from_registry(
            &tools,
            "bash_wait",
            json!({"process_id":started.data["process_id"],"cursor":0,"yield_time_ms":250}),
            ctx.clone(),
        )
        .await
        .unwrap()
        .result;
    }
    assert_eq!(started.data["state"], "running");
    assert_eq!(started.data["output"], "ready");
    let other =
        Arc::new(ToolUseContext::new("t59-other", Some(dir.path().to_string_lossy().into_owned())));
    assert!(
        crate::tools::execute_tool_from_registry(
            &tools,
            "bash_cancel",
            json!({"process_id":started.data["process_id"]}),
            other
        )
        .await
        .is_err()
    );
    let done = crate::tools::execute_tool_from_registry(
        &tools,
        "bash_cancel",
        json!({"process_id":started.data["process_id"]}),
        ctx,
    )
    .await
    .unwrap()
    .result;
    assert_eq!(done.data["state"], "cancelled");
}

#[tokio::test]
async fn codex_plugin_compat_t59_rejects_removed_exec_command() {
    let tools = crate::tools::default_tools(Arc::new(SecurityPolicy::default()));
    let result = crate::tools::execute_tool_from_registry(
        &tools,
        "exec_command",
        json!({"cmd": "echo removed"}),
        Arc::new(ToolUseContext::new("removed-exec", None)),
    )
    .await;
    assert!(result.unwrap_err().to_string().contains("Unknown tool: exec_command"));
}

#[tokio::test]
async fn codex_plugin_compat_t59_preserves_isolation() {
    let tool = crate::tools::ShellTool::new(
        Arc::new(SecurityPolicy::default()),
        Arc::new(NativeRuntime::new()),
    );
    assert!(tool.call(json!({"command":"printf isolated"})).await.is_err());
}

#[tokio::test]
async fn codex_plugin_compat_t59_preserves_workdir_and_package_bytes() {
    let dir = tempfile::tempdir().unwrap();
    let package = dir.path().join("package");
    std::fs::create_dir(&package).unwrap();
    let script = "printf '%s' \"$PWD\"";
    std::fs::write(package.join("harmless.sh"), script).unwrap();
    let policy = Arc::new(SecurityPolicy {
        autonomy: AutonomyLevel::Full,
        workspace_only: false,
        workspace_dir: dir.path().into(),
        allowed_commands: vec!["sh".into()],
        forbidden_paths: vec![],
        ..SecurityPolicy::default()
    });
    let tools = crate::tools::default_tools_with_runtime(policy, Arc::new(NativeRuntime::new()));
    let ctx = Arc::new(ToolUseContext::new(
        "t59-script",
        Some(dir.path().to_string_lossy().into_owned()),
    ));
    let result = crate::tools::execute_tool_from_registry(
        &tools,
        "bash",
        json!({"command":"sh harmless.sh","workdir":package,"yield_time_ms":1000}),
        ctx,
    )
    .await
    .unwrap()
    .result;
    assert_eq!(result.data["state"], "exited");
    assert_eq!(result.data["exit_code"], 0);
    assert_eq!(
        std::fs::canonicalize(result.data["output"].as_str().unwrap()).unwrap(),
        std::fs::canonicalize(&package).unwrap()
    );
    assert_eq!(std::fs::read_to_string(package.join("harmless.sh")).unwrap(), script);
}
