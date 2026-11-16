//! Real host project/configuration injection for the shared CLI acceptance suite.
use serde_json::Value;
async fn run(value: &str) {
    let _guard = crate::test_support::global_async_env_lock().lock().await;
    crate::session::llm::prompt::ensure_host();
    let input = llm_runtime::acp::acceptance_support::model_source::load_manifest(value);
    let root = std::path::Path::new(input["root"].as_str().unwrap());
    agent_env::set("VW_ACP_OFFLINE_FIXTURE_KEY", input["key"].as_str().unwrap());
    let tenant = crate::session::ui_store::current_tenant_scope();
    let (project, _) = crate::project::from_directory(&tenant, root).await.unwrap();
    let context = crate::project::instance::Context { directory: root.to_string_lossy().into(), worktree: project.worktree.clone(), project };
    crate::project::instance::provide_context(context, llm_runtime::acp::acceptance_support::model_source::run::<super::super::request::RuntimeHost>(input, value)).await;
}
pub(in crate::session::llm::acp) use llm_runtime::acp::acceptance_support::model_source::observe_rpc;
impl llm_runtime::acp::acceptance_support::AcceptanceHost for super::super::request::RuntimeHost {
    fn update_project<'a>(root: &'a std::path::Path, patch: Value) -> llm_runtime::acp::host::HostFuture<'a, anyhow::Result<()>> {
        Box::pin(async move { crate::config::update_project_for_tenant(&crate::session::ui_store::current_tenant_scope(), root, patch).await?; Ok(()) })
    }
    fn update(patch: Value) -> llm_runtime::acp::host::HostFuture<'static, anyhow::Result<()>> {
        Box::pin(async move { crate::config::update(patch).await?; Ok(()) })
    }
}
#[tokio::test]
#[ignore = "requires the pinned OpenCode CLI and an owned offline Chat fixture"]
async fn acp_model_source_opencode_effort_real_cli() {
    run("session-model").await;
}
#[tokio::test]
#[ignore = "requires the pinned Claude Code CLI and an owned offline Messages fixture"]
async fn acp_model_source_claude_effort_real_cli() {
    run("session-model").await;
}
#[tokio::test]
#[ignore = "requires the pinned Copilot CLI and an owned offline Chat fixture"]
async fn acp_model_source_copilot_effort_real_cli() {
    run("session-model").await;
}
#[tokio::test]
#[ignore = "requires explicit pinned real CLI manifest and owned offline model fixture"]
async fn acp_model_source_real_cli_session_model() {
    run("session-model").await;
}
#[tokio::test]
#[ignore = "requires explicit pinned real CLI manifest and owned offline model fixture"]
async fn acp_model_source_real_cli_cli_local() {
    run("cli-local").await;
}
