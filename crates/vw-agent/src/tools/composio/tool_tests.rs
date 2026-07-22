use super::super::core::{ApiFuture, ComposioApiClient};
use super::super::types;
use super::ComposioTool;
use crate::app::agent::security::SecurityPolicy;
use crate::app::agent::tools::traits::Tool;
use parking_lot::Mutex;
use serde_json::json;
use std::collections::VecDeque;
use std::sync::Arc;

fn test_tool() -> ComposioTool {
    ComposioTool::new("test-api-key", Some(" test-entity "), Arc::new(SecurityPolicy::default()))
}

fn test_tool_with_api(api: Arc<MockComposioApi>) -> ComposioTool {
    ComposioTool::new_with_api(
        "test-api-key",
        Some(" test-entity "),
        Arc::new(SecurityPolicy::default()),
        api,
    )
}

#[derive(Default)]
struct MockComposioApi {
    list_actions: Mutex<VecDeque<Result<types::ComposioToolsResponse, String>>>,
    connected_accounts: Mutex<VecDeque<Result<Vec<types::ComposioConnectedAccount>, String>>>,
    execute_results: Mutex<VecDeque<Result<serde_json::Value, String>>>,
    auth_config_ids: Mutex<VecDeque<Result<String, String>>>,
    connection_links: Mutex<VecDeque<Result<types::ComposioConnectionLink, String>>>,
    schemas: Mutex<VecDeque<Result<serde_json::Value, String>>>,
    execute_slugs: Mutex<Vec<String>>,
}

impl MockComposioApi {
    fn push_list_actions(&self, result: Result<types::ComposioToolsResponse, String>) {
        self.list_actions.lock().push_back(result);
    }

    fn push_connected_accounts(
        &self,
        result: Result<Vec<types::ComposioConnectedAccount>, String>,
    ) {
        self.connected_accounts.lock().push_back(result);
    }

    fn push_execute_result(&self, result: Result<serde_json::Value, String>) {
        self.execute_results.lock().push_back(result);
    }

    fn push_connection_link(&self, result: Result<types::ComposioConnectionLink, String>) {
        self.connection_links.lock().push_back(result);
    }

    fn push_schema(&self, result: Result<serde_json::Value, String>) {
        self.schemas.lock().push_back(result);
    }
}

fn pop_result<T>(queue: &Mutex<VecDeque<Result<T, String>>>, operation: &str) -> anyhow::Result<T> {
    match queue.lock().pop_front() {
        Some(Ok(value)) => Ok(value),
        Some(Err(message)) => anyhow::bail!(message),
        None => anyhow::bail!("unexpected {operation} call"),
    }
}

impl ComposioApiClient for MockComposioApi {
    fn list_actions_v3<'a>(
        &'a self,
        _client: &'a reqwest::Client,
        _api_key: &'a str,
        _app_name: Option<&'a str>,
    ) -> ApiFuture<'a, types::ComposioToolsResponse> {
        let result = pop_result(&self.list_actions, "list_actions_v3");
        Box::pin(async move { result })
    }

    fn list_connected_accounts<'a>(
        &'a self,
        _client: &'a reqwest::Client,
        _api_key: &'a str,
        _app_name: Option<&'a str>,
        _entity_id: Option<&'a str>,
    ) -> ApiFuture<'a, Vec<types::ComposioConnectedAccount>> {
        let result = pop_result(&self.connected_accounts, "list_connected_accounts");
        Box::pin(async move { result })
    }

    fn execute_action_v3<'a>(
        &'a self,
        _client: &'a reqwest::Client,
        _api_key: &'a str,
        tool_slug: &'a str,
        _params: serde_json::Value,
        _text: Option<&'a str>,
        _entity_id: Option<&'a str>,
        _connected_account_ref: Option<&'a str>,
    ) -> ApiFuture<'a, serde_json::Value> {
        self.execute_slugs.lock().push(tool_slug.to_string());
        let result = pop_result(&self.execute_results, "execute_action_v3");
        Box::pin(async move { result })
    }

    fn resolve_auth_config_id<'a>(
        &'a self,
        _client: &'a reqwest::Client,
        _api_key: &'a str,
        _app_name: &'a str,
    ) -> ApiFuture<'a, String> {
        let result = pop_result(&self.auth_config_ids, "resolve_auth_config_id");
        Box::pin(async move { result })
    }

    fn get_connection_url_v3<'a>(
        &'a self,
        _client: &'a reqwest::Client,
        _api_key: &'a str,
        _auth_config_id: &'a str,
        _entity_id: &'a str,
    ) -> ApiFuture<'a, types::ComposioConnectionLink> {
        let result = pop_result(&self.connection_links, "get_connection_url_v3");
        Box::pin(async move { result })
    }

    fn get_tool_schema<'a>(
        &'a self,
        _client: &'a reqwest::Client,
        _api_key: &'a str,
        _slug: &'a str,
    ) -> ApiFuture<'a, serde_json::Value> {
        let result = pop_result(&self.schemas, "get_tool_schema");
        Box::pin(async move { result })
    }
}

fn v3_tool(index: usize) -> types::ComposioV3Tool {
    types::ComposioV3Tool {
        slug: Some(format!("github-tool-{index}")),
        name: Some(format!("GITHUB_TOOL_{index}")),
        description: Some(format!("desc {index}")),
        app_name: Some("GitHub".to_string()),
        toolkit: None,
        input_parameters: Some(json!({
            "type": "object",
            "properties": {"repo": {"type": "string"}},
            "required": ["repo"],
        })),
    }
}

fn account(index: usize) -> types::ComposioConnectedAccount {
    types::ComposioConnectedAccount {
        id: format!("acct-{index}"),
        status: "ACTIVE".to_string(),
        toolkit: Some(types::ComposioToolkitRef {
            slug: Some("gmail".to_string()),
            name: Some("Gmail".to_string()),
        }),
    }
}

#[test]
fn composio_tool_exposes_action_contract() {
    let tool = test_tool();

    assert_eq!(tool.name(), "composio");
    assert!(tool.description().contains("action='execute'"));
    let schema = tool.parameters_schema();
    assert_eq!(schema["type"], json!("object"));
    assert_eq!(schema["required"], json!(["action"]));
    assert_eq!(
        schema["properties"]["action"]["enum"],
        json!(["list", "list_accounts", "connected_accounts", "execute", "connect"])
    );
}

#[tokio::test]
async fn composio_list_formats_action_summary_and_truncates_after_twenty() {
    let api = Arc::new(MockComposioApi::default());
    api.push_list_actions(Ok(types::ComposioToolsResponse {
        items: (0..21).map(v3_tool).collect(),
    }));
    let tool = test_tool_with_api(api);

    let result =
        tool.execute(json!({"action": "list", "app": "github"})).await.expect("list action");

    assert!(result.success);
    assert!(result.output.contains("Found 21 available actions:"));
    assert!(result.output.contains("- github-tool-0 (GitHub): desc 0 [params: repo*]"));
    assert!(result.output.contains("... and 1 more"));
}

#[tokio::test]
async fn composio_connected_accounts_reports_empty_and_formats_account_summary() {
    let api = Arc::new(MockComposioApi::default());
    api.push_connected_accounts(Ok(Vec::new()));
    api.push_connected_accounts(Ok((0..21).map(account).collect()));
    let tool = test_tool_with_api(api);

    let empty = tool
        .execute(json!({"action": "list_accounts", "app": "github"}))
        .await
        .expect("empty accounts");
    let populated =
        tool.execute(json!({"action": "connected_accounts"})).await.expect("connected accounts");

    assert!(empty.success);
    assert_eq!(
        empty.output,
        "No connected accounts found for app 'github' for entity 'test-entity'. Run action='connect' first."
    );
    assert!(populated.success);
    assert!(populated.output.contains("Found 21 connected accounts (entity 'test-entity'):"));
    assert!(populated.output.contains("- acct-0 [ACTIVE] toolkit=gmail"));
    assert!(populated.output.contains("... and 1 more"));
}

#[tokio::test]
async fn composio_execute_rejects_missing_action_before_io() {
    let error = test_tool()
        .execute(json!({"entity_id": "entity-a"}))
        .await
        .expect_err("missing action should be rejected");

    assert_eq!(error.to_string(), "Missing 'action' parameter");
}

#[tokio::test]
async fn composio_execute_reports_unknown_action_without_io() {
    let result = test_tool()
        .execute(json!({"action": "archive", "entity_id": "entity-a"}))
        .await
        .expect("unknown action should return a tool result");

    assert!(!result.success);
    assert!(result.output.is_empty());
    assert_eq!(
        result.error.as_deref(),
        Some("Unknown action 'archive'. Use 'list', 'list_accounts', 'execute', or 'connect'.")
    );
}

#[tokio::test]
async fn composio_execute_requires_action_name_for_execute_action() {
    let error = test_tool()
        .execute(json!({"action": "execute"}))
        .await
        .expect_err("missing action_name should be rejected");

    assert_eq!(error.to_string(), "Missing 'action_name' (or 'tool_slug') for execute");
}

#[tokio::test]
async fn composio_execute_accepts_action_name_and_pretty_prints_success() {
    let api = Arc::new(MockComposioApi::default());
    api.push_execute_result(Ok(json!({"ok": true, "items": [1, 2]})));
    let tool = test_tool_with_api(api.clone());

    let result = tool
        .execute(json!({
            "action": "execute",
            "action_name": "github-list-repos",
            "app": "github",
            "params": {"limit": 2},
            "text": "list repos",
            "connected_account_id": "acct-1",
        }))
        .await
        .expect("execute action");

    assert!(result.success);
    assert!(result.output.contains("\"ok\": true"));
    assert_eq!(api.execute_slugs.lock().as_slice(), &["github-list-repos".to_string()]);
}

#[tokio::test]
async fn composio_execute_error_includes_schema_hint_when_available() {
    let api = Arc::new(MockComposioApi::default());
    for _ in 0..8 {
        api.push_execute_result(Err("bad params".to_string()));
    }
    api.push_schema(Ok(json!({
        "input_parameters": {
            "type": "object",
            "properties": {
                "repo": {"type": "string", "description": "Repository name"}
            },
            "required": ["repo"],
        }
    })));
    let tool = test_tool_with_api(api);

    let result = tool
        .execute(json!({
            "action": "execute",
            "tool_slug": "github-list-repos",
            "connected_account_id": "acct-1",
        }))
        .await
        .expect("execute result");

    assert!(!result.success);
    let error = result.error.expect("error text");
    assert!(error.contains("Action execution failed: Composio execute failed on v3"));
    assert!(error.contains("Expected input parameters:"));
    assert!(error.contains("repo: string (required) - Repository name"));
}

#[tokio::test]
async fn composio_connect_outputs_link_and_caches_connected_account() {
    let api = Arc::new(MockComposioApi::default());
    api.push_connection_link(Ok(types::ComposioConnectionLink {
        redirect_url: "https://example.test/connect".to_string(),
        connected_account_id: Some("acct-connected".to_string()),
    }));
    let tool = test_tool_with_api(api);

    let result = tool
        .execute(json!({
            "action": "connect",
            "app": "github",
            "auth_config_id": "auth-1",
            "entity_id": "workspace-user",
        }))
        .await
        .expect("connect action");

    assert!(result.success);
    assert_eq!(
        result.output,
        "Open this URL to connect github:\nhttps://example.test/connect\nConnected account ID: acct-connected"
    );
    assert_eq!(
        tool.resolve_connected_account_ref(Some("github"), Some("workspace-user"))
            .await
            .expect("cached account")
            .as_deref(),
        Some("acct-connected")
    );
}
