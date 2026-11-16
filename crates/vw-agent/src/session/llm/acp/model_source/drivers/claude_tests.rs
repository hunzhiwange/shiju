use super::*;
use crate::{config::schema::AcpAgentConfig, model_relay::RelayIdentity};



#[tokio::test]
async fn acp_model_source_claude_disabled_model_and_missing_credentials_do_not_fallback() {
    let _lock = crate::test_support::global_async_env_lock().lock().await;
    struct Restore(Option<std::ffi::OsString>);
    impl Drop for Restore {
        fn drop(&mut self) {
            unsafe {
                match self.0.take() {
                    Some(value) => std::env::set_var("VIBEWINDOW_CONFIG_DIR", value),
                    None => std::env::remove_var("VIBEWINDOW_CONFIG_DIR"),
                }
            }
        }
    }
    let root = tempfile::tempdir().unwrap();
    let _restore = Restore(std::env::var_os("VIBEWINDOW_CONFIG_DIR"));
    unsafe {
        std::env::set_var("VIBEWINDOW_CONFIG_DIR", root.path().join("config"));
    }
    let tenant = crate::session::ui_store::current_tenant_scope();
    let (project, _) = crate::project::from_directory(&tenant, root.path()).await.unwrap();
    // This unique provider has no local account and names an absent credential.
    let missing = format!("VW_CLAUDE_ABSENT_{}", uuid::Uuid::new_v4().simple());
    crate::config::update_project_for_tenant(&tenant, root.path(), serde_json::json!({
        "providers": {"claude-negative-test": {
            "api":"http://127.0.0.1:1/v1/chat/completions", "adapter":"openai-compatible", "env":[missing],
            "models":{"selected-model":{"status":"active","tool_call":true,"temperature":true,"limit":{"context":12000,"output":1024}},"disabled":{"status":"disabled"}}
        }}
    })).await.unwrap();
    let context = crate::project::instance::Context {
        directory: root.path().to_string_lossy().into(),
        worktree: project.worktree.clone(),
        project,
    };
    let config = AcpAgentConfig { command: "claude-agent-acp".into(), ..Default::default() };
    crate::project::instance::provide_context(context, async {
        let active = crate::provider::provider::get_model("claude-negative-test","selected-model").await.unwrap();
        // A persisted active snapshot must not resurrect a now-disabled catalog key.
        let mut disabled = active.clone();
        disabled.id = "disabled".into();
        assert!(super::super::super::prepare("claude",&config,&disabled,"negative").await.is_err());
        let launch = super::super::super::prepare("claude",&config,&active,"negative").await.unwrap();
        assert_eq!(launch.model_context_limit,12000);
        assert_eq!(launch.model_output_limit,1024);
        let response = crate::model_relay::upstream_http_client().unwrap()
            .post(format!("{}/v1/messages",launch.config().env["ANTHROPIC_BASE_URL"]))
            .bearer_auth(launch.binding.bearer_token())
            .json(&serde_json::json!({"model":"selected-model","max_tokens":16,"messages":[{"role":"user","content":"hello"}]}))
            .send().await.unwrap();
        assert_eq!(response.status(),reqwest::StatusCode::UNAUTHORIZED);
    }).await;
}
