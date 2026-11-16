use super::*;
use crate::{config::schema::AcpAgentConfig, model_relay::RelayIdentity};


use crate::project::instance;
use std::path::Path;


#[tokio::test]
async fn acp_model_source_codex_missing_credentials_and_disabled_model_never_use_native_account() {
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
    crate::config::update_project_for_tenant(
        &tenant,
        root.path(),
        serde_json::json!({
            "providers": {"codex-offline-no-account": {
                "api": "http://127.0.0.1:1/v1/responses",
                "adapter":"openai-responses", "env":[],
                "models": {
                    "active-model":{"status":"active","tool_call":true},
                    "disabled-model":{"status":"disabled","tool_call":true}
                }
            }}
        }),
    )
    .await
    .unwrap();
    let context = crate::project::instance::Context {
        directory: root.path().to_string_lossy().into(),
        worktree: project.worktree.clone(),
        project,
    };
    let relay_context = context.clone();
    crate::project::instance::provide_context(context, async {
        let config = AcpAgentConfig { command: "codex".into(), ..Default::default() };
        let active =
            crate::provider::provider::get_model("codex-offline-no-account", "active-model")
                .await
                .unwrap();
        // Exercise the real relay/configuration boundary without requiring an installed CLI.
        // Supplier executable identity is covered separately by real-CLI acceptance.
        let mut profile = AcpLaunchProfile::new(
            &config,
            RelayIdentity {
                tenant: tenant.clone(),
                project: relay_context.project.id.clone(),
                session: "missing-key".into(),
                provider: active.provider_id.clone(),
                model: active.id.clone(),
                generation: 0,
            },
        )
        .unwrap();
        profile.model_api = Some(active.api.clone());
        let server = crate::model_relay::RelayServer::start(
            profile.binding.clone(),
            std::sync::Arc::new(crate::model_relay::ProviderRelayBackend::new().unwrap()),
            relay_context,
        )
        .await
        .unwrap();
        let relay_endpoint = format!("http://{}", server.address());
        profile.attach_server(server);
        llm_runtime::acp::acceptance_support::negative::codex_rejects_native_account::<super::super::super::super::request::RuntimeHost>(profile, active, config, relay_endpoint).await;
    })
    .await;
}

async fn codex_run(mode: &str, effort_check: bool) {
    use llm_runtime::acp::acceptance_support::codex;
    let _guard = crate::test_support::global_async_env_lock().lock().await;
    crate::session::llm::prompt::ensure_host();
    let input = codex::load_manifest(mode);
    let root = Path::new(input["root"].as_str().unwrap());
    agent_env::set("VW_ACP_OFFLINE_FIXTURE_KEY", input["key"].as_str().unwrap());
    let tenant = crate::session::ui_store::current_tenant_scope();
    let (project, _) = crate::project::from_directory(&tenant, root).await.unwrap();
    let context = instance::Context { directory: root.to_string_lossy().into(), worktree: project.worktree.clone(), project };
    instance::provide_context(context, codex::run::<super::super::super::super::request::RuntimeHost>(input, mode, effort_check)).await;
}

#[tokio::test]
#[ignore = "requires explicit pinned real CLI manifest and owned offline model fixture"]
async fn acp_model_source_codex_real_cli_session_model() {
    codex_run("session-model", false).await;
}

#[tokio::test]
#[ignore = "requires the pinned Codex CLI and an owned offline Responses fixture"]
async fn acp_model_source_codex_effort_real_cli() {
    codex_run("session-model", true).await;
}

#[tokio::test]
#[ignore = "requires explicit pinned real CLI manifest and owned offline Chat model fixture"]
async fn acp_model_source_codex_chat_real_cli() {
    codex_run("session-model-chat", false).await;
}

#[tokio::test]
#[ignore = "requires explicit pinned real CLI manifest and owned offline model fixture"]
async fn acp_model_source_codex_real_cli_cli_local() {
    codex_run("cli-local", false).await;
}
