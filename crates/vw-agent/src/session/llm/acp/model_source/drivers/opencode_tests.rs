use super::*;
use crate::{config::schema::AcpAgentConfig, model_relay::RelayIdentity};



#[tokio::test]
async fn acp_model_source_opencode_missing_credentials_and_disabled_model_never_use_native_account()
{
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
    crate::config::update_project_for_tenant(&tenant, root.path(), serde_json::json!({
        "providers": {"opencode-offline-no-account": {
            "api":"http://127.0.0.1:1/v1/chat/completions", "adapter":"openai-compatible", "env":[],
            "models": {"active-model":{"status":"active","tool_call":true},
                       "disabled-model":{"status":"disabled","tool_call":true}}
        }}
    })).await.unwrap();
    let context = crate::project::instance::Context {
        directory: root.path().to_string_lossy().into(),
        worktree: project.worktree.clone(),
        project,
    };
    let relay_context = context.clone();
    crate::project::instance::provide_context(context, async {
        let config = AcpAgentConfig { command:"opencode".into(), args:vec!["acp".into()], ..Default::default() };
        let active = crate::provider::provider::get_model("opencode-offline-no-account", "active-model").await.unwrap();
        // Exercise the real relay without requiring a CLI installation in the unit suite.
        let identity = crate::session::llm::acp::model_source::launch_identity(
            relay_context.project.id.clone(), "opencode", &active, "missing-key",
        );
        let mut profile = AcpLaunchProfile::new(&config, identity).unwrap();
        profile.model_api = Some(active.api.clone());
        let server = crate::model_relay::RelayServer::start(
            profile.binding.clone(),
            std::sync::Arc::new(crate::model_relay::ProviderRelayBackend::new().unwrap()),
            relay_context,
        ).await.unwrap();
        let relay_endpoint = format!("http://{}", server.address());
        profile.attach_server(server);
        llm_runtime::acp::acceptance_support::negative::opencode_rejects_native_account::<super::super::super::super::request::RuntimeHost>(profile, active, config, relay_endpoint).await;
    }).await;
}
