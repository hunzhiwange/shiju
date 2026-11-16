use super::*;
use crate::{config::schema::AcpAgentConfig, model_relay::RelayIdentity};



#[tokio::test]
async fn acp_model_source_gemini_project_cli_override_reaches_actual_subprocess() {
    let _guard = crate::test_support::global_async_env_lock().lock().await;
    let root = tempfile::tempdir().unwrap();
    let marker = root.path().join("project-cli-started");
    let tenant = crate::session::ui_store::current_tenant_scope();
    let (project, _) = crate::project::from_directory(&tenant, root.path()).await.unwrap();
    crate::config::update_project_for_tenant(&tenant, root.path(), serde_json::json!({
        "acp":{"__gemini_project_probe__":{"command":"python3","args":["-c","import pathlib,sys; pathlib.Path(sys.argv[1]).write_text('PROJECT_OVERRIDE_USED')",marker]}},
        "providers":{"gemini-project-probe":{"api":"http://127.0.0.1:1/v1/chat/completions","adapter":"openai-compatible","models":{"probe":{"status":"active"}}}}
    })).await.unwrap();
    let context = crate::project::instance::Context {
        directory: root.path().to_string_lossy().into(),
        worktree: project.worktree.clone(),
        project,
    };
    crate::project::instance::provide_context(context, async {
        let model = crate::provider::provider::get_model("gemini-project-probe", "probe").await.unwrap();
        let info = crate::provider::provider::get_provider("gemini-project-probe").await.unwrap().unwrap();
        let result = crate::session::llm::acp::request::do_stream_request_acp(
            &model, &info, None,
            &serde_json::json!({"acp_agent":"__gemini_project_probe__","acp_model_source":"cli_local","cwd":root.path(),"acp_close_client_after_request":true}),
            &serde_json::json!([{"role":"user","content":"project CLI boundary probe"}]),
            "gemini-project-override-test", None, &mut |_| {},
        ).await;
        assert!(result.is_err(), "probe exits before ACP handshake");
        assert_eq!(std::fs::read_to_string(&marker).unwrap(), "PROJECT_OVERRIDE_USED");
    }).await;
}
