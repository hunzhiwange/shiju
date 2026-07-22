use super::{ApiFuture, ComposioApiClient, ComposioTool, RealComposioApiClient, types};
use crate::app::agent::security::SecurityPolicy;
use parking_lot::Mutex;
use serde_json::json;
use std::collections::VecDeque;
use std::sync::Arc;

fn test_tool(default_entity_id: Option<&str>) -> ComposioTool {
    ComposioTool::new("test-api-key", default_entity_id, Arc::new(SecurityPolicy::default()))
}

fn test_tool_with_api(api: Arc<MockComposioApi>) -> ComposioTool {
    ComposioTool::new_with_api(
        "test-api-key",
        Some("workspace-user"),
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
    list_action_apps: Mutex<Vec<Option<String>>>,
    connected_account_queries: Mutex<Vec<(Option<String>, Option<String>)>>,
    execute_calls: Mutex<Vec<ExecuteCall>>,
    auth_config_apps: Mutex<Vec<String>>,
    connection_calls: Mutex<Vec<(String, String)>>,
    schema_slugs: Mutex<Vec<String>>,
}

#[derive(Debug, Clone, PartialEq)]
struct ExecuteCall {
    tool_slug: String,
    params: serde_json::Value,
    text: Option<String>,
    entity_id: Option<String>,
    connected_account_ref: Option<String>,
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

    fn push_auth_config_id(&self, result: Result<String, String>) {
        self.auth_config_ids.lock().push_back(result);
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
        app_name: Option<&'a str>,
    ) -> ApiFuture<'a, types::ComposioToolsResponse> {
        self.list_action_apps.lock().push(app_name.map(str::to_string));
        let result = pop_result(&self.list_actions, "list_actions_v3");
        Box::pin(async move { result })
    }

    fn list_connected_accounts<'a>(
        &'a self,
        _client: &'a reqwest::Client,
        _api_key: &'a str,
        app_name: Option<&'a str>,
        entity_id: Option<&'a str>,
    ) -> ApiFuture<'a, Vec<types::ComposioConnectedAccount>> {
        self.connected_account_queries
            .lock()
            .push((app_name.map(str::to_string), entity_id.map(str::to_string)));
        let result = pop_result(&self.connected_accounts, "list_connected_accounts");
        Box::pin(async move { result })
    }

    fn execute_action_v3<'a>(
        &'a self,
        _client: &'a reqwest::Client,
        _api_key: &'a str,
        tool_slug: &'a str,
        params: serde_json::Value,
        text: Option<&'a str>,
        entity_id: Option<&'a str>,
        connected_account_ref: Option<&'a str>,
    ) -> ApiFuture<'a, serde_json::Value> {
        self.execute_calls.lock().push(ExecuteCall {
            tool_slug: tool_slug.to_string(),
            params: params.clone(),
            text: text.map(str::to_string),
            entity_id: entity_id.map(str::to_string),
            connected_account_ref: connected_account_ref.map(str::to_string),
        });
        let result = pop_result(&self.execute_results, "execute_action_v3");
        Box::pin(async move { result })
    }

    fn resolve_auth_config_id<'a>(
        &'a self,
        _client: &'a reqwest::Client,
        _api_key: &'a str,
        app_name: &'a str,
    ) -> ApiFuture<'a, String> {
        self.auth_config_apps.lock().push(app_name.to_string());
        let result = pop_result(&self.auth_config_ids, "resolve_auth_config_id");
        Box::pin(async move { result })
    }

    fn get_connection_url_v3<'a>(
        &'a self,
        _client: &'a reqwest::Client,
        _api_key: &'a str,
        auth_config_id: &'a str,
        entity_id: &'a str,
    ) -> ApiFuture<'a, types::ComposioConnectionLink> {
        self.connection_calls.lock().push((auth_config_id.to_string(), entity_id.to_string()));
        let result = pop_result(&self.connection_links, "get_connection_url_v3");
        Box::pin(async move { result })
    }

    fn get_tool_schema<'a>(
        &'a self,
        _client: &'a reqwest::Client,
        _api_key: &'a str,
        slug: &'a str,
    ) -> ApiFuture<'a, serde_json::Value> {
        self.schema_slugs.lock().push(slug.to_string());
        let result = pop_result(&self.schemas, "get_tool_schema");
        Box::pin(async move { result })
    }
}

fn v3_tool(slug: Option<&str>, name: Option<&str>) -> types::ComposioV3Tool {
    types::ComposioV3Tool {
        slug: slug.map(str::to_string),
        name: name.map(str::to_string),
        description: Some("List repositories".to_string()),
        app_name: Some("GitHub".to_string()),
        toolkit: None,
        input_parameters: Some(json!({"type": "object"})),
    }
}

fn account(id: &str, status: &str) -> types::ComposioConnectedAccount {
    types::ComposioConnectedAccount {
        id: id.to_string(),
        status: status.to_string(),
        toolkit: None,
    }
}

#[test]
fn new_normalizes_entity_ids_and_initializes_empty_caches() {
    let default_tool = test_tool(None);
    assert_eq!(default_tool.default_entity_id(), "default");
    assert!(default_tool.recent_connected_accounts.read().is_empty());
    assert!(default_tool.action_slug_cache.read().is_empty());

    let explicit_tool = test_tool(Some("  workspace-user  "));
    assert_eq!(explicit_tool.default_entity_id(), "workspace-user");

    let blank_tool = test_tool(Some("   "));
    assert_eq!(blank_tool.default_entity_id(), "default");
}

#[test]
fn security_returns_the_configured_policy() {
    let security =
        Arc::new(SecurityPolicy { max_actions_per_hour: 7, ..SecurityPolicy::default() });
    let tool = ComposioTool::new("test-api-key", None, security);

    assert_eq!(tool.security().max_actions_per_hour, 7);
}

#[test]
fn connected_account_cache_normalizes_keys_and_overwrites_entries() {
    let tool = test_tool(None);

    tool.cache_connected_account(" GITHUB_APP ", " workspace-user ", "account-old");
    assert_eq!(
        tool.get_cached_connected_account("github-app", "workspace-user").as_deref(),
        Some("account-old")
    );

    tool.cache_connected_account("github--app", "workspace-user", "account-new");
    assert_eq!(
        tool.get_cached_connected_account("GITHUB_APP", " workspace-user ").as_deref(),
        Some("account-new")
    );
    assert!(tool.get_cached_connected_account("github-app", "another-user").is_none());
    assert_eq!(tool.recent_connected_accounts.read().len(), 1);
}

#[tokio::test]
async fn resolve_connected_account_returns_none_for_missing_or_blank_app() {
    let tool = test_tool(None);

    assert!(
        tool.resolve_connected_account_ref(None, Some("workspace-user")).await.unwrap().is_none()
    );
    assert!(
        tool.resolve_connected_account_ref(Some("   "), Some("workspace-user"))
            .await
            .unwrap()
            .is_none()
    );
    assert!(tool.resolve_connected_account_ref(Some("github"), None).await.unwrap().is_none());
    assert!(tool.recent_connected_accounts.read().is_empty());
}

#[tokio::test]
async fn resolve_connected_account_uses_normalized_cache_hit() {
    let tool = test_tool(None);
    tool.cache_connected_account("GITHUB_APP", " workspace-user ", "account-cached");

    let resolved = tool
        .resolve_connected_account_ref(Some(" github--app "), Some("workspace-user"))
        .await
        .unwrap();

    assert_eq!(resolved.as_deref(), Some("account-cached"));
    assert_eq!(tool.recent_connected_accounts.read().len(), 1);
}

#[test]
fn action_slug_cache_rejects_blank_aliases_and_slugs() {
    let tool = test_tool(None);

    tool.cache_action_slug("   ", "github-list-repos");
    tool.cache_action_slug("GITHUB_LIST_REPOS", "   ");

    assert!(tool.lookup_cached_action_slug("GITHUB_LIST_REPOS").is_none());
    assert!(tool.lookup_cached_action_slug("   ").is_none());
    assert!(tool.action_slug_cache.read().is_empty());
}

#[test]
fn action_slug_cache_normalizes_aliases_trims_slugs_and_overwrites() {
    let tool = test_tool(None);

    tool.cache_action_slug(" GITHUB__LIST_REPOS ", "  github-list-repos-v1  ");
    assert_eq!(
        tool.lookup_cached_action_slug("github--list-repos").as_deref(),
        Some("github-list-repos-v1")
    );

    tool.cache_action_slug("github-list-repos", "github-list-repos-v2");
    assert_eq!(
        tool.lookup_cached_action_slug("GITHUB_LIST_REPOS").as_deref(),
        Some("github-list-repos-v2")
    );
    assert_eq!(tool.action_slug_cache.read().len(), 1);
}

#[test]
fn v3_slug_candidates_put_cached_slug_first_and_deduplicate() {
    let tool = test_tool(None);
    tool.cache_action_slug("GITHUB_LIST_REPOS", "github-list-repos");

    let candidates = tool.build_v3_slug_candidates("GITHUB_LIST_REPOS");

    assert_eq!(candidates.first().map(String::as_str), Some("github-list-repos"));
    assert_eq!(candidates.iter().filter(|slug| slug.as_str() == "github-list-repos").count(), 1);
    assert!(candidates.contains(&"GITHUB_LIST_REPOS".to_string()));
    assert!(candidates.contains(&"github_list_repos".to_string()));
}

#[test]
fn v3_slug_candidates_are_empty_for_blank_action() {
    let tool = test_tool(None);

    assert!(tool.build_v3_slug_candidates("   ").is_empty());
}

#[tokio::test]
async fn execute_action_rejects_blank_action_without_external_request() {
    let tool = test_tool(None);

    let error = tool
        .execute_action("   ", Some("   "), json!({}), None, Some(" workspace-user "), Some("   "))
        .await
        .unwrap_err();

    let message = error.to_string();
    assert!(message.contains("Unable to determine tool slug"));
    assert!(message.contains("Run action='list'"));
    assert!(tool.action_slug_cache.read().is_empty());
    assert!(tool.recent_connected_accounts.read().is_empty());
}

#[tokio::test]
async fn connection_url_rejects_missing_target_without_external_request() {
    let tool = test_tool(None);

    let error = tool.get_connection_url(None, None, "workspace-user").await.unwrap_err();

    assert_eq!(error.to_string(), "Missing 'app' or 'auth_config_id' for v3 connect");
}

#[test]
fn request_builder_wrapper_preserves_optional_fields() {
    let params = json!({"repository": "vibe-window"});

    let (url, body) = ComposioTool::build_execute_action_v3_request(
        "github-get-repository",
        params,
        None,
        Some("workspace-user"),
        Some(" account-7 "),
    );

    assert_eq!(url, "https://backend.composio.dev/api/v3/tools/execute/github-get-repository");
    assert_eq!(
        body,
        json!({
            "version": "latest",
            "arguments": {"repository": "vibe-window"},
            "user_id": "workspace-user",
            "connected_account_id": "account-7",
        })
    );
}

#[test]
fn list_query_wrapper_distinguishes_present_and_absent_app() {
    assert_eq!(
        ComposioTool::build_list_actions_v3_query(None),
        vec![
            ("limit".to_string(), "200".to_string()),
            ("toolkit_versions".to_string(), "latest".to_string()),
        ]
    );
    assert_eq!(
        ComposioTool::build_list_actions_v3_query(Some(" gmail ")),
        vec![
            ("limit".to_string(), "200".to_string()),
            ("toolkit_versions".to_string(), "latest".to_string()),
            ("toolkits".to_string(), "gmail".to_string()),
            ("toolkit_slug".to_string(), "gmail".to_string()),
        ]
    );
}

#[test]
fn real_api_client_methods_construct_futures_without_polling_network() {
    let api = RealComposioApiClient;
    let client = reqwest::Client::new();

    drop(api.list_actions_v3(&client, "api-key", Some("github")));
    drop(api.list_connected_accounts(&client, "api-key", Some("github"), Some("user")));
    drop(api.execute_action_v3(
        &client,
        "api-key",
        "github-list-repos",
        json!({"limit": 1}),
        Some("list repos"),
        Some("user"),
        Some("account"),
    ));
    drop(api.resolve_auth_config_id(&client, "api-key", "github"));
    drop(api.get_connection_url_v3(&client, "api-key", "auth", "user"));
    drop(api.get_tool_schema(&client, "api-key", "github-list-repos"));
}

#[tokio::test]
async fn list_actions_uses_api_response_updates_cache_and_maps_actions() {
    let api = Arc::new(MockComposioApi::default());
    api.push_list_actions(Ok(types::ComposioToolsResponse {
        items: vec![v3_tool(Some("github-list-repos"), Some("GITHUB_LIST_REPOS"))],
    }));
    let tool = test_tool_with_api(api.clone());

    let actions = tool.list_actions(Some("github")).await.expect("list actions");

    assert_eq!(api.list_action_apps.lock().as_slice(), &[Some("github".to_string())]);
    assert_eq!(actions.len(), 1);
    assert_eq!(actions[0].name, "github-list-repos");
    assert_eq!(actions[0].app_name.as_deref(), Some("GitHub"));
    assert!(actions[0].enabled);
    assert_eq!(
        tool.lookup_cached_action_slug("GITHUB_LIST_REPOS").as_deref(),
        Some("github-list-repos")
    );
}

#[tokio::test]
async fn resolve_connected_account_queries_api_and_caches_first_usable_account() {
    let api = Arc::new(MockComposioApi::default());
    api.push_connected_accounts(Ok(vec![account("failed", "FAILED"), account("active", "ACTIVE")]));
    let tool = test_tool_with_api(api.clone());

    let resolved = tool
        .resolve_connected_account_ref(Some(" GitHub_App "), Some(" workspace-user "))
        .await
        .expect("resolve account");

    assert_eq!(resolved.as_deref(), Some("active"));
    assert_eq!(
        api.connected_account_queries.lock().as_slice(),
        &[(Some("github-app".to_string()), Some("workspace-user".to_string()))]
    );
    assert_eq!(
        tool.resolve_connected_account_ref(Some("github-app"), Some("workspace-user"))
            .await
            .expect("cached account")
            .as_deref(),
        Some("active")
    );
    assert_eq!(api.connected_account_queries.lock().len(), 1);
}

#[tokio::test]
async fn resolve_connected_account_returns_none_when_api_has_no_usable_accounts() {
    let api = Arc::new(MockComposioApi::default());
    api.push_connected_accounts(Ok(vec![account("bad", "FAILED")]));
    let tool = test_tool_with_api(api);

    let resolved = tool
        .resolve_connected_account_ref(Some("github"), Some("workspace-user"))
        .await
        .expect("resolve no usable account");

    assert!(resolved.is_none());
    assert!(tool.recent_connected_accounts.read().is_empty());
}

#[tokio::test]
async fn execute_action_uses_explicit_account_and_returns_first_successful_candidate() {
    let api = Arc::new(MockComposioApi::default());
    api.push_execute_result(Ok(json!({"ok": true})));
    let tool = test_tool_with_api(api.clone());
    tool.cache_action_slug("GITHUB_LIST_REPOS", "github-list-repos");

    let result = tool
        .execute_action(
            "GITHUB_LIST_REPOS",
            Some("github"),
            json!({"limit": 10}),
            None,
            Some(" workspace-user "),
            Some(" account-1 "),
        )
        .await
        .expect("execute action");

    assert_eq!(result, json!({"ok": true}));
    assert!(api.connected_account_queries.lock().is_empty());
    assert_eq!(
        api.execute_calls.lock().as_slice(),
        &[ExecuteCall {
            tool_slug: "github-list-repos".to_string(),
            params: json!({"limit": 10}),
            text: None,
            entity_id: Some("workspace-user".to_string()),
            connected_account_ref: Some("account-1".to_string()),
        }]
    );
    assert_eq!(
        tool.lookup_cached_action_slug("github-list-repos").as_deref(),
        Some("github-list-repos")
    );
}

#[tokio::test]
async fn execute_action_refresh_error_is_reported_when_slug_candidates_stay_empty() {
    let api = Arc::new(MockComposioApi::default());
    api.push_list_actions(Err("list unavailable".to_string()));
    let tool = test_tool_with_api(api);

    let error = tool
        .execute_action(
            "   ",
            Some(" GitHub "),
            json!({}),
            None,
            Some("workspace-user"),
            Some("account-1"),
        )
        .await
        .expect_err("blank action cannot resolve slug");

    let message = error.to_string();
    assert!(message.contains("Unable to determine tool slug"));
    assert!(message.contains("Failed to refresh action list for app 'github': list unavailable"));
}

#[tokio::test]
async fn execute_action_refresh_success_still_rejects_blank_action_without_candidates() {
    let api = Arc::new(MockComposioApi::default());
    api.push_list_actions(Ok(types::ComposioToolsResponse {
        items: vec![v3_tool(Some("github-list-repos"), Some("GITHUB_LIST_REPOS"))],
    }));
    let tool = test_tool_with_api(api);

    let error = tool
        .execute_action(
            "   ",
            Some(" GitHub "),
            json!({}),
            None,
            Some("workspace-user"),
            Some("account-1"),
        )
        .await
        .expect_err("blank action cannot resolve slug after refresh");

    assert!(error.to_string().contains("Unable to determine tool slug"));
    assert_eq!(
        tool.lookup_cached_action_slug("GITHUB_LIST_REPOS").as_deref(),
        Some("github-list-repos")
    );
}

#[tokio::test]
async fn execute_action_aggregates_candidate_errors_for_nlp_requests() {
    let api = Arc::new(MockComposioApi::default());
    api.push_execute_result(Err("first failed".to_string()));
    api.push_execute_result(Err("second failed".to_string()));
    let tool = test_tool_with_api(api.clone());
    tool.cache_action_slug("GITHUB_LIST_REPOS", "github-list-repos");

    let error = tool
        .execute_action(
            "GITHUB_LIST_REPOS",
            Some("github"),
            json!({"ignored": true}),
            Some("list my repos"),
            Some("workspace-user"),
            Some("account-1"),
        )
        .await
        .expect_err("all candidates fail");

    let message = error.to_string();
    assert!(message.contains("Composio v3 NLP execute failed"));
    assert!(message.contains("github-list-repos: first failed"));
    assert!(message.contains("GITHUB_LIST_REPOS: second failed"));
    assert!(api.execute_calls.lock().len() >= 2);
}

#[tokio::test]
async fn execute_action_aggregates_candidate_errors_for_structured_requests() {
    let api = Arc::new(MockComposioApi::default());
    api.push_execute_result(Err("structured failed".to_string()));
    let tool = test_tool_with_api(api);
    tool.cache_action_slug("GITHUB_LIST_REPOS", "github-list-repos");

    let error = tool
        .execute_action(
            "GITHUB_LIST_REPOS",
            Some("github"),
            json!({"limit": 10}),
            None,
            Some("workspace-user"),
            Some("account-1"),
        )
        .await
        .expect_err("structured candidates fail");

    let message = error.to_string();
    assert!(message.contains("Composio execute failed on v3"));
    assert!(message.contains("github-list-repos: structured failed"));
}

#[tokio::test]
async fn connection_url_uses_explicit_auth_config_or_resolves_app_before_linking() {
    let api = Arc::new(MockComposioApi::default());
    api.push_connection_link(Ok(types::ComposioConnectionLink {
        redirect_url: "https://app.example/connect-explicit".to_string(),
        connected_account_id: Some("acct-explicit".to_string()),
    }));
    api.push_auth_config_id(Ok("auth-from-app".to_string()));
    api.push_connection_link(Ok(types::ComposioConnectionLink {
        redirect_url: "https://app.example/connect-app".to_string(),
        connected_account_id: None,
    }));
    let tool = test_tool_with_api(api.clone());

    let explicit = tool
        .get_connection_url(None, Some("auth-explicit"), "workspace-user")
        .await
        .expect("explicit auth config");
    let resolved = tool
        .get_connection_url(Some("github"), None, "workspace-user")
        .await
        .expect("resolved auth config");

    assert_eq!(explicit.redirect_url, "https://app.example/connect-explicit");
    assert_eq!(explicit.connected_account_id.as_deref(), Some("acct-explicit"));
    assert_eq!(resolved.redirect_url, "https://app.example/connect-app");
    assert_eq!(api.auth_config_apps.lock().as_slice(), &["github".to_string()]);
    assert_eq!(
        api.connection_calls.lock().as_slice(),
        &[
            ("auth-explicit".to_string(), "workspace-user".to_string()),
            ("auth-from-app".to_string(), "workspace-user".to_string()),
        ]
    );
}

#[tokio::test]
async fn get_tool_schema_normalizes_slug_before_api_lookup() {
    let api = Arc::new(MockComposioApi::default());
    api.push_schema(Ok(json!({"schema": true})));
    let tool = test_tool_with_api(api.clone());

    let schema = tool.get_tool_schema(" GITHUB_LIST_REPOS ").await.expect("schema");

    assert_eq!(schema, json!({"schema": true}));
    assert_eq!(api.schema_slugs.lock().as_slice(), &["github-list-repos".to_string()]);
}
