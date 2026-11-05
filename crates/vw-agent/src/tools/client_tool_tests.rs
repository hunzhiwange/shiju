use super::catalog::ToolCatalog;
use super::client_tool::ClientTool;
use super::{Tool, ToolSpec, ToolUseContext};
use crate::session::runtime::SessionScope;
use crate::session::runtime::dynamic_calls::{
    ClientToolHost, ClientToolRequests, DynamicCallOwner,
};
use serde_json::json;
use std::sync::Arc;
use vw_api_types::tools::{DynamicToolContent, DynamicToolResponse, ToolSource};

fn setup() -> (Arc<ClientToolHost>, ClientToolRequests, ToolSpec, Arc<ToolUseContext>) {
    let root = std::env::current_dir().unwrap();
    let context = Arc::new(
        ToolUseContext::new("client", Some(root.to_string_lossy().into())).with_message_id("turn"),
    );
    let scope = SessionScope::new(context.tenant_scope(), &root).unwrap();
    let (host, requests) =
        ClientToolHost::register(DynamicCallOwner::new(scope, "client", "host").unwrap());
    let mut spec = ToolSpec::new("client_echo", "Echo", json!({"type":"object"}));
    spec.source = ToolSource::Client;
    (host, requests, spec, context)
}

#[tokio::test]
async fn dynamic_tools_callback_response_owner_is_validated() {
    let (host, _requests, spec, _) = setup();
    let tools: Vec<Box<dyn Tool>> = vec![Box::new(ClientTool::new(spec, host).unwrap())];
    let context = Arc::new(
        ToolUseContext::new(
            "wrong",
            Some(std::env::current_dir().unwrap().to_string_lossy().into()),
        )
        .with_message_id("turn"),
    );
    assert!(
        super::execute_tool_from_registry(&tools, "client_echo", json!({}), context).await.is_err()
    );
}

#[tokio::test]
async fn dynamic_tools_callback_registered_tool_preserves_media_across_catalog_update() {
    let (host, mut requests, spec, context) = setup();
    let catalog = ToolCatalog::default();
    let snapshot = catalog
        .replace(
            vec![Arc::new(ClientTool::new(spec, host.clone()).unwrap())],
            ["client_echo".into()],
        )
        .unwrap();
    let task = tokio::spawn(async move {
        snapshot
            .bind("client_echo")
            .unwrap()
            .execute(&snapshot, json!({"hello":"world"}), context)
            .await
    });
    let request = requests.recv().await.unwrap();
    assert_eq!(request.turn_id, "turn");
    assert_eq!(request.tool_id.0.as_str(), "client_echo");
    assert_eq!(request.arguments, json!({"hello":"world"}));
    catalog.replace(vec![], []).unwrap();
    let response = DynamicToolResponse {
        success: false,
        content: vec![
            DynamicToolContent::Text { text: "text".into() },
            DynamicToolContent::Image { image_url: "data:image/png;base64,eA==".into() },
            DynamicToolContent::Audio { audio_url: "data:audio/wav;base64,eA==".into() },
        ],
    };
    host.respond(host.owner(), &request.call_id, response.clone()).unwrap();
    let result = task.await.unwrap().unwrap().result;
    assert!(!result.is_success());
    assert_eq!(result.model_result, serde_json::to_value(response.content).unwrap());
    assert_eq!(result.to_dto().content.len(), 3);
    assert_eq!(
        result.to_dto().content[1],
        vw_api_types::tools::ToolResultContentDto::Json {
            value: json!({"type":"image","image_url":"data:image/png;base64,eA=="})
        }
    );
    assert!(catalog.snapshot().bind("client_echo").is_err());
}

#[tokio::test]
async fn dynamic_tools_callback_registration_rejects_source_schema_and_direct_execution() {
    let (host, _, mut spec, _) = setup();
    spec.source = ToolSource::Builtin;
    assert!(ClientTool::new(spec.clone(), host.clone()).is_err());
    spec.source = ToolSource::Client;
    spec.input_schema = json!({"description":"x".repeat(64 * 1024)});
    assert!(ClientTool::new(spec.clone(), host.clone()).is_err());
    spec.input_schema = json!({"type":"object"});
    let tool = ClientTool::new(spec, host).unwrap();
    assert!(tool.call(json!({})).await.is_err());
    assert!(tool.execute(json!({})).await.is_err());
}

#[tokio::test]
async fn dynamic_tools_callback_permission_denial_never_emits_request() {
    use crate::agent::defaults::{DefaultSelection, DefaultsContext, resolve};
    use vw_config_types::agent::{AgentDefault, AgentDefinitionConfig};
    let (host, mut requests, spec, context) = setup();
    let tools: Vec<Box<dyn Tool>> = vec![Box::new(ClientTool::new(spec, host.clone()).unwrap())];
    let mut agent = AgentDefinitionConfig::default();
    agent.defaults.permission = AgentDefault::Fixed(json!({"client_echo":"deny"}));
    let effective = resolve(
        &agent,
        &DefaultSelection::default(),
        &DefaultSelection::default(),
        &DefaultsContext::default(),
    )
    .unwrap();
    let context = Arc::new(context.as_ref().clone().with_native_permission(Arc::new(effective)));
    let error = super::execute_tool_from_registry(&tools, "client_echo", json!({}), context)
        .await
        .unwrap_err();
    assert!(error.message().contains("denied"));
    host.disconnect();
    drop(tools);
    drop(host);
    assert!(requests.recv().await.is_none());
}

#[tokio::test]
async fn dynamic_tools_callback_execution_rejects_wrong_tenant_workspace_and_missing_turn() {
    let (host, _requests, spec, context) = setup();
    let wrong_dir = tempfile::tempdir().unwrap();
    for owner in [
        DynamicCallOwner::new(
            SessionScope::new(
                vw_api_types::id::TenantScope::new("other", "company").unwrap(),
                host.owner().scope.directory(),
            )
            .unwrap(),
            "client",
            "host",
        )
        .unwrap(),
        DynamicCallOwner::new(
            SessionScope::new(context.tenant_scope(), wrong_dir.path()).unwrap(),
            "client",
            "host",
        )
        .unwrap(),
    ] {
        let (other, _requests) = ClientToolHost::register(owner);
        let tools: Vec<Box<dyn Tool>> =
            vec![Box::new(ClientTool::new(spec.clone(), other).unwrap())];
        assert!(
            super::execute_tool_from_registry(&tools, "client_echo", json!({}), context.clone())
                .await
                .is_err()
        );
    }
    let context = Arc::new(ToolUseContext::new("client", context.root().map(str::to_string)));
    let tools: Vec<Box<dyn Tool>> = vec![Box::new(ClientTool::new(spec, host).unwrap())];
    assert!(
        super::execute_tool_from_registry(&tools, "client_echo", json!({}), context).await.is_err()
    );
}
