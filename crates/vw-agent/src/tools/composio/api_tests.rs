use super::{
    build_execute_action_v3_request, build_list_actions_v3_query, composio_api_base_v3,
    composio_tool_version_latest, update_action_slug_cache_from_v3,
};
use crate::app::agent::tools::composio::types::ComposioV3Tool;
use parking_lot::RwLock;
use serde_json::json;
use std::collections::HashMap;

fn tool(slug: Option<&str>, name: Option<&str>) -> ComposioV3Tool {
    ComposioV3Tool {
        slug: slug.map(str::to_string),
        name: name.map(str::to_string),
        description: None,
        app_name: None,
        toolkit: None,
        input_parameters: None,
    }
}

#[test]
fn build_list_actions_v3_query_omits_filters_for_blank_app() {
    let query = build_list_actions_v3_query(Some(" \t\n "));

    assert_eq!(
        query,
        vec![
            ("limit".to_string(), "200".to_string()),
            ("toolkit_versions".to_string(), "latest".to_string()),
        ]
    );
}

#[test]
fn build_list_actions_v3_query_trims_and_applies_app_filters() {
    let query = build_list_actions_v3_query(Some("  GitHub  "));

    assert_eq!(
        query,
        vec![
            ("limit".to_string(), "200".to_string()),
            ("toolkit_versions".to_string(), "latest".to_string()),
            ("toolkits".to_string(), "GitHub".to_string()),
            ("toolkit_slug".to_string(), "GitHub".to_string()),
        ]
    );
}

#[test]
fn build_execute_action_v3_request_uses_structured_arguments_without_text() {
    let params = json!({
        "count": 0,
        "nested": {"enabled": false},
    });
    let (url, body) =
        build_execute_action_v3_request("github-list-repos", params.clone(), None, None, None);

    assert_eq!(url, "https://backend.composio.dev/api/v3/tools/execute/github-list-repos");
    assert_eq!(body, json!({"version": "latest", "arguments": params}));
}

#[test]
fn build_execute_action_v3_request_prefers_text_and_trims_account_reference() {
    let (url, body) = build_execute_action_v3_request(
        "gmail-send-email",
        json!({"ignored": true}),
        Some("send the message"),
        Some("workspace-user"),
        Some("  account-42  "),
    );

    assert_eq!(url, "https://backend.composio.dev/api/v3/tools/execute/gmail-send-email");
    assert_eq!(
        body,
        json!({
            "version": "latest",
            "text": "send the message",
            "user_id": "workspace-user",
            "connected_account_id": "account-42",
        })
    );
    assert!(body.get("arguments").is_none());
}

#[test]
fn build_execute_action_v3_request_keeps_present_empty_text() {
    let (_, body) = build_execute_action_v3_request(
        "slack-send-message",
        json!({"channel": "general"}),
        Some(""),
        None,
        None,
    );

    assert_eq!(body, json!({"version": "latest", "text": ""}));
    assert!(body.get("arguments").is_none());
}

#[test]
fn build_execute_action_v3_request_omits_blank_account_but_keeps_present_entity() {
    let (_, body) = build_execute_action_v3_request(
        "github-list-repos",
        json!({}),
        None,
        Some(""),
        Some(" \t "),
    );

    assert_eq!(
        body,
        json!({
            "version": "latest",
            "arguments": {},
            "user_id": "",
        })
    );
    assert!(body.get("connected_account_id").is_none());
}

#[test]
fn update_action_slug_cache_maps_normalized_slug_and_name() {
    let cache = RwLock::new(HashMap::from([("existing".to_string(), "kept".to_string())]));
    let items = [tool(Some("GMAIL_SEND_EMAIL"), Some("SEND_GMAIL_MESSAGE"))];

    update_action_slug_cache_from_v3(&cache, &items);

    let cache = cache.read();
    assert_eq!(cache.get("gmail-send-email").map(String::as_str), Some("GMAIL_SEND_EMAIL"));
    assert_eq!(cache.get("send-gmail-message").map(String::as_str), Some("GMAIL_SEND_EMAIL"));
    assert_eq!(cache.get("existing").map(String::as_str), Some("kept"));
}

#[test]
fn update_action_slug_cache_uses_name_when_slug_is_missing() {
    let cache = RwLock::new(HashMap::new());
    let items = [tool(None, Some("GITHUB_LIST_REPOS")), tool(None, None)];

    update_action_slug_cache_from_v3(&cache, &items);

    let cache = cache.read();
    assert_eq!(cache.len(), 1);
    assert_eq!(cache.get("github-list-repos").map(String::as_str), Some("GITHUB_LIST_REPOS"));
}

#[test]
fn update_action_slug_cache_replaces_collisions_with_later_items() {
    let cache = RwLock::new(HashMap::new());
    let items = [tool(Some("GMAIL_SEND"), None), tool(Some("gmail-send"), None)];

    update_action_slug_cache_from_v3(&cache, &items);

    let cache = cache.read();
    assert_eq!(cache.len(), 1);
    assert_eq!(cache.get("gmail-send").map(String::as_str), Some("gmail-send"));
}

#[test]
fn update_action_slug_cache_ignores_blank_identifiers() {
    let cache = RwLock::new(HashMap::new());
    let items = [tool(Some("   "), Some(" \t "))];

    update_action_slug_cache_from_v3(&cache, &items);

    assert!(cache.read().is_empty());
}

#[test]
fn composio_constants_match_request_contract() {
    assert_eq!(composio_tool_version_latest(), "latest");
    assert_eq!(composio_api_base_v3(), "https://backend.composio.dev/api/v3");
}
