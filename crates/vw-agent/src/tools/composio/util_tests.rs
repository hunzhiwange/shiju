use super::*;

#[test]
fn ensure_https_rejects_non_https_urls() {
    assert!(ensure_https("https://api.example.com").is_ok());
    assert!(ensure_https("http://api.example.com").is_err());
}

#[test]
fn normalizers_trim_case_and_collapse_empty_values() {
    assert_eq!(normalize_entity_id("  user  "), "user");
    assert_eq!(normalize_entity_id("   "), "default");
    assert_eq!(normalize_tool_slug("GITHUB_GET_REPO"), "github-get-repo");
    assert_eq!(normalize_app_slug("--GitHub_App--"), "github-app");
}

#[test]
fn cache_key_helpers_are_stable() {
    assert_eq!(connected_account_cache_key("GitHub_App", " user "), "user:github-app");
    assert_eq!(normalize_action_cache_key("--GITHUB__GET--"), Some("github-get".into()));
    assert_eq!(normalize_action_cache_key("   "), None);
}

#[test]
fn tool_slug_candidates_keep_original_first_and_deduplicate() {
    let candidates = build_tool_slug_candidates("GITHUB_GET_REPO");
    assert_eq!(candidates.first().map(String::as_str), Some("GITHUB_GET_REPO"));
    assert!(candidates.contains(&"github-get-repo".to_string()));
    assert_eq!(candidates.iter().filter(|item| item.as_str() == "github-get-repo").count(), 1);
}

#[test]
fn response_extractors_support_legacy_and_nested_field_names() {
    assert_eq!(
        extract_redirect_url(&serde_json::json!({"redirectUrl": "https://example.com/oauth"})),
        Some("https://example.com/oauth".to_string())
    );
    assert_eq!(
        extract_redirect_url(
            &serde_json::json!({"data": {"redirect_url": "https://example.com/nested"}})
        ),
        Some("https://example.com/nested".to_string())
    );
    assert_eq!(
        extract_connected_account_id(
            &serde_json::json!({"data": {"connectedAccountId": "account-1"}})
        ),
        Some("account-1".to_string())
    );
    assert_eq!(extract_redirect_url(&serde_json::json!({"redirect_url": 7})), None);
    assert_eq!(extract_connected_account_id(&serde_json::json!({})), None);
}

#[test]
fn error_helpers_extract_redact_flatten_and_truncate_messages() {
    assert_eq!(
        extract_api_error_message(r#"{"error":{"message":"bad request"}}"#),
        Some("bad request".to_string())
    );
    assert_eq!(
        extract_api_error_message(r#"{"message":"fallback"}"#),
        Some("fallback".to_string())
    );
    assert_eq!(extract_api_error_message("not-json"), None);

    let sanitized = sanitize_error_message("entity_id=user-1\nconnectedAccountId=account-1");
    assert_eq!(sanitized, "[redacted]=user-1 [redacted]=account-1");

    let long = "x".repeat(241);
    let truncated = sanitize_error_message(&long);
    assert_eq!(truncated.len(), 243);
    assert!(truncated.ends_with("..."));
}

#[test]
fn schema_formatters_mark_required_fields_and_handle_missing_shapes() {
    let input = serde_json::json!({
        "properties": {
            "owner": {"type": "string"},
            "repo": {"type": "string"}
        },
        "required": ["owner"]
    });
    let compact = format_input_params_hint(Some(&input));
    assert!(compact.contains("owner*"));
    assert!(compact.contains("repo"));
    assert_eq!(format_input_params_hint(Some(&serde_json::json!({}))), "");

    let schema = serde_json::json!({
        "input_parameters": {
            "properties": {
                "owner": {
                    "type": "string",
                    "description": "Repository owner"
                },
                "payload": {},
                "long": {
                    "type": "string",
                    "description": "ééééééééééééééééééééééééééééééééééééééééé"
                }
            },
            "required": ["owner"]
        }
    });
    let detailed = format_schema_hint(&schema).expect("valid schema should produce a hint");
    assert!(detailed.contains("owner: string (required) - Repository owner"));
    assert!(detailed.contains("payload: any"));
    assert!(detailed.lines().any(|line| line.contains("long: string - ") && line.ends_with("...")));
    assert_eq!(format_schema_hint(&serde_json::json!({})), None);
}
