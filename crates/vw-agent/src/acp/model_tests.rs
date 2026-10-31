//! 通过原生配置和运行时请求验证模型选择与会话隔离。

use super::super::{VibeWindowBackend, start_prompt_with_runtime_runner};
use crate::session::processor::StreamEvent;
use crate::session::ui_types::TokenUsage;
use agent_client_protocol::SessionModelState;
use serde_json::json;
use vw_acp::server::{BackendEvent, PromptContent, SessionBackend};
use vw_api_types::id::TenantScope;

fn ids(state: &SessionModelState) -> Vec<&str> {
    state.available_models.iter().map(|model| model.model_id.0.as_ref()).collect()
}

#[tokio::test]
async fn model_selection_reaches_runtime_and_preserves_other_sessions() {
    let root = tempfile::tempdir().unwrap();
    let tenant = TenantScope::new("acp-model-platform", "acp-model-company").unwrap();
    let mut config = crate::config::Config::default();
    config.providers.insert(
        "fixture".into(),
        json!({
            "name": "Fixture",
            "models": {
                "second": {"name": "Second", "status": "active"},
                "first": {"name": "First", "status": "active"},
                "disabled": {"status": "disabled"}
            }
        }),
    );
    crate::config::save_config_for_tenant(&config, &tenant).await.unwrap();
    let backend = VibeWindowBackend::new(tenant.clone());
    let first = backend.create_session(root.path().into()).await.unwrap();
    let second = backend.create_session(root.path().into()).await.unwrap();
    let models = backend.session_models(&first.id).await.unwrap().unwrap();
    let fixture_ids =
        ids(&models).into_iter().filter(|id| id.starts_with("fixture/")).collect::<Vec<_>>();
    assert_eq!(fixture_ids, ["fixture/first", "fixture/second"]);
    assert_eq!(ids(&models)[0], "auto");
    assert_eq!(models.current_model_id.0.as_ref(), "auto");
    backend.set_session_model(&first.id, "fixture/second").await.unwrap();
    for invalid in ["", "second", "fixture/disabled", "fixture/missing"] {
        assert!(backend.set_session_model(&first.id, invalid).await.is_err());
    }
    assert_eq!(
        backend.session_models(&second.id).await.unwrap().unwrap().current_model_id.0.as_ref(),
        "auto"
    );
    backend.load_session(&first.id, root.path().into()).await.unwrap();
    let mut turn = start_prompt_with_runtime_runner(
        &backend,
        &first.id,
        vec![PromptContent::Text("hello".into())],
        |request, mut emit| {
            assert_eq!(request.model.as_deref(), Some("fixture/second"));
            emit(StreamEvent::Done(TokenUsage::default()));
        },
    )
    .await
    .unwrap();
    let terminal = tokio::time::timeout(std::time::Duration::from_secs(5), turn.events.recv())
        .await
        .unwrap()
        .unwrap();
    assert!(matches!(terminal, BackendEvent::Terminal(_)), "{terminal:?}");
    let reconnected = VibeWindowBackend::new(tenant);
    reconnected.load_session(&first.id, root.path().into()).await.unwrap();
    assert_eq!(
        reconnected.session_models(&first.id).await.unwrap().unwrap().current_model_id.0.as_ref(),
        "fixture/second"
    );
    backend.set_session_model(&first.id, "auto").await.unwrap();
    assert_eq!(
        backend.session_models(&first.id).await.unwrap().unwrap().current_model_id.0.as_ref(),
        "auto"
    );
    let other =
        VibeWindowBackend::new(TenantScope::new("acp-model-platform", "other-company").unwrap());
    assert!(other.set_session_model(&first.id, "fixture/first").await.is_err());
    let foreign_session = other.create_session(root.path().into()).await.unwrap();
    let foreign_models = other.session_models(&foreign_session.id).await.unwrap().unwrap();
    assert!(!ids(&foreign_models).iter().any(|id| id.starts_with("fixture/")));
}
