use super::*;
use crate::app::agent::runtime::NativeRuntime;
use crate::tools::context::ToolUseContext;
use crate::tools::traits::ToolCallResult;

fn registry(security: Arc<SecurityPolicy>) -> Vec<Box<dyn Tool>> {
    crate::tools::default_tools_with_runtime(security, Arc::new(NativeRuntime::new()))
}

fn security(path: &Path) -> Arc<SecurityPolicy> {
    Arc::new(SecurityPolicy {
        autonomy: AutonomyLevel::Full,
        workspace_only: false,
        workspace_dir: path.to_path_buf(),
        allowed_commands: vec![
            "printf".into(),
            "sleep".into(),
            "grep".into(),
            "echo".into(),
            "env".into(),
            "pwd".into(),
        ],
        forbidden_paths: vec![],
        ..SecurityPolicy::default()
    })
}

fn context(path: &Path, session: &str) -> Arc<ToolUseContext> {
    Arc::new(ToolUseContext::new(session, Some(path.to_string_lossy().to_string())))
}

async fn call(
    tools: &[Box<dyn Tool>],
    ctx: Arc<ToolUseContext>,
    name: &str,
    input: serde_json::Value,
) -> ToolCallResult {
    crate::tools::execute_tool_from_registry(tools, name, input, ctx).await.unwrap().result
}

async fn start(tools: &[Box<dyn Tool>], ctx: Arc<ToolUseContext>) -> ToolCallResult {
    let mut result = call(tools, ctx.clone(), "bash", json!({"command":"printf ready; sleep 60", "description":"wait for explicit cancellation", "yield_time_ms":250})).await;
    let deadline = tokio::time::Instant::now() + Duration::from_secs(10);
    while result.data["output"] != "ready"
        && result.data["state"] == "running"
        && tokio::time::Instant::now() < deadline
    {
        result = call(
            tools,
            ctx.clone(),
            "bash_wait",
            json!({"process_id":result.data["process_id"],"cursor":0,"yield_time_ms":250}),
        )
        .await;
    }
    result
}

#[tokio::test]
async fn shell_session_tools_start_wait_cancel() {
    let dir = tempfile::tempdir().unwrap();
    let tools = registry(security(dir.path()));
    let ctx = context(dir.path(), "start-wait-cancel");
    let started = start(&tools, ctx.clone()).await;
    let id = started.data["process_id"].as_str().unwrap();
    assert_eq!(started.data["state"], "running");
    assert_eq!(started.data["output"], "ready");
    let read = call(
        &tools,
        ctx.clone(),
        "bash_wait",
        json!({"process_id":id,"cursor":started.data["next_cursor"],"yield_time_ms":250}),
    )
    .await;
    assert_eq!(read.data["state"], "running");
    assert_eq!(read.data["output"], "");
    let cancelled = call(&tools, ctx.clone(), "bash_cancel", json!({"process_id":id})).await;
    assert_eq!(cancelled.data["state"], "cancelled");
    let again = call(&tools, ctx, "bash_cancel", json!({"process_id":id})).await;
    assert_eq!(again.data["state"], "cancelled");
    eprintln!(
        "process={id} running -> cancelled; repeated cancellation acknowledged after reaping"
    );
}

#[test]
fn shell_session_tools_survive_registry_rebuild() {
    let dir = tempfile::tempdir().unwrap();
    let policy = security(dir.path());
    let ctx = context(dir.path(), "registry-rebuild");
    let first = registry(policy.clone());
    let started = tokio::runtime::Runtime::new().unwrap().block_on(start(&first, ctx.clone()));
    drop(first);
    let second = registry(policy);
    // Both the first registry and its caller runtime have been destroyed.
    let done = tokio::runtime::Runtime::new().unwrap().block_on(call(
        &second,
        ctx,
        "bash_cancel",
        json!({"process_id":started.data["process_id"]}),
    ));
    assert_eq!(done.data["state"], "cancelled");
    eprintln!(
        "process={} survived registry and caller runtime rebuild",
        started.data["process_id"]
    );
}

#[tokio::test]
async fn shell_session_tools_preserve_security_checks() {
    let dir = tempfile::tempdir().unwrap();
    let tools = registry(security(dir.path()));
    let ctx = context(dir.path(), "security-owner");
    for input in [
        json!({"command":"echo denied","yield_time_ms":249}),
        json!({"command":"echo denied","yield_time_ms":30001}),
        json!({"command":"echo denied","timeout":0}),
    ] {
        assert!(
            crate::tools::execute_tool_from_registry(&tools, "bash", input, ctx.clone())
                .await
                .is_err()
        );
    }
    let denied_tools = registry(Arc::new(SecurityPolicy {
        autonomy: AutonomyLevel::ReadOnly,
        workspace_dir: dir.path().to_path_buf(),
        ..SecurityPolicy::default()
    }));
    assert!(
        crate::tools::execute_tool_from_registry(
            &denied_tools,
            "bash",
            json!({"command":"touch forbidden"}),
            ctx.clone()
        )
        .await
        .is_err()
    );
    assert!(!dir.path().join("forbidden").exists());
    let started = start(&tools, ctx.clone()).await;
    for name in ["bash_wait", "bash_cancel"] {
        assert!(
            crate::tools::execute_tool_from_registry(
                &tools,
                name,
                json!({"process_id":started.data["process_id"]}),
                context(dir.path(), "other-session")
            )
            .await
            .is_err()
        );
    }
    let other_workspace = tempfile::tempdir().unwrap();
    let other_tools = registry(security(other_workspace.path()));
    let tenant_tools = crate::session::ui_store::with_tenant_scope(
        vw_api_types::id::TenantScope::new("fixture-platform", "other-company").unwrap(),
        async { registry(security(dir.path())) },
    )
    .await;
    for foreign_tools in [&other_tools, &tenant_tools] {
        for name in ["bash_wait", "bash_cancel"] {
            assert!(
                crate::tools::execute_tool_from_registry(
                    foreign_tools,
                    name,
                    json!({"process_id":started.data["process_id"]}),
                    ctx.clone()
                )
                .await
                .is_err()
            );
        }
    }
    for cursor in [json!(-1), json!("0"), json!(1.5)] {
        assert!(
            crate::tools::execute_tool_from_registry(
                &tools,
                "bash_wait",
                json!({"process_id":started.data["process_id"], "cursor":cursor}),
                ctx.clone()
            )
            .await
            .is_err()
        );
    }
    call(&tools, ctx, "bash_cancel", json!({"process_id":started.data["process_id"]})).await;
    let bash = tools.iter().find(|tool| tool.spec().id == "bash").unwrap();
    assert!(
        bash.call(json!({"command":"echo denied"}))
            .await
            .unwrap_err()
            .to_string()
            .contains("trusted session")
    );
}

#[tokio::test]
async fn shell_session_tools_expose_running_without_exit_code() {
    let dir = tempfile::tempdir().unwrap();
    let tools = registry(security(dir.path()));
    let ctx = context(dir.path(), "running-protocol");
    std::fs::write(dir.path().join("empty"), "").unwrap();
    let schema = tools.iter().find(|tool| tool.spec().id == "bash").unwrap().parameters_schema();
    assert_eq!(schema["properties"]["timeout"]["type"], "integer");
    assert!(schema["properties"]["timeout"].get("default").is_none());
    let running = start(&tools, ctx.clone()).await;
    assert!(running.is_success());
    assert!(running.data.get("exit_code").is_none());
    assert_eq!(running.model_result["state"], "running");
    for field in ["process_id", "output", "next_cursor", "truncated", "dropped_bytes"] {
        assert!(running.model_result.get(field).is_some());
    }
    call(&tools, ctx.clone(), "bash_cancel", json!({"process_id":running.data["process_id"]}))
        .await;
    let finished = call(
        &tools,
        ctx.clone(),
        "bash",
        json!({"command":"grep absent empty","description":"verify no matches exit semantics"}),
    )
    .await;
    assert_eq!(finished.data["state"], "exited");
    assert_eq!(finished.data["exit_code"], 1);
    assert!(finished.is_success());
    let mut timed = call(&tools,ctx,"bash",json!({"command":"sleep 60","description":"verify explicit execution deadline","timeout":30,"yield_time_ms":250})).await;
    let deadline = tokio::time::Instant::now() + Duration::from_secs(10);
    while timed.data["state"] == "running" && tokio::time::Instant::now() < deadline {
        timed = call(&tools, context(dir.path(), "running-protocol"), "bash_wait", json!({"process_id":timed.data["process_id"],"cursor":timed.data["next_cursor"],"yield_time_ms":250})).await;
    }
    assert_eq!(timed.data["state"], "timed_out");
    assert!(!timed.is_success());
    eprintln!("real grep exit_code=1 accepted by ExitSemantics; explicit deadline timed_out");
}
