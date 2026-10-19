use super::*;

fn map_value<'a>(map: &'a Mapping, key: &str) -> &'a Value {
    map.get(Value::String(key.to_string())).expect("missing key")
}

#[test]
fn http_request_visual_edit_round_trips_without_secret_leak() {
    let secret = "fixture-secret-never-log";
    let raw = format!(
        "method: post\nurl: https://example.com/api\nauthorization:\n  type: bearer\n  token: {secret}\n  vendor_extension: keep-auth\nheaders: 'X-Test: yes'\nparams: 'page: 1'\nbody:\n  type: json\n  data: '{{\"hello\":\"world\"}}'\n  vendor_body: keep-body\ntimeout: {{ connect: 10, read: 60, write: 20 }}\nssl_verify: true\nretry_config: {{ retry_enabled: true, max_retries: 2, retry_interval: 300 }}\nerror_strategy: fail-branch\nvendor_root: keep-root\n"
    );
    let mut draft =
        build_node_visual_draft("http-request", &raw).expect("draft").expect("visual draft");
    let WorkflowNodeVisualDraft::HttpRequest { method, url, auth_token, body_type, .. } =
        &mut draft
    else {
        panic!("http draft expected")
    };
    *method = "put".to_string();
    *url = "https://example.com/v2".to_string();
    assert_eq!(auth_token, secret);
    *body_type = "raw-text".to_string();

    let synced = apply_visual_draft_to_yaml("http-request", &raw, Some(&draft)).expect("sync");
    assert!(
        !format!("{:?}", validate_node_editor_draft("http-request", "", "", &synced, Some(&draft)))
            .contains(secret)
    );
    let value: Value = serde_yaml::from_str(&synced).expect("yaml");
    let map = value.as_mapping().expect("map");
    assert_eq!(map_value(map, "method").as_str(), Some("put"));
    assert_eq!(map_value(map, "vendor_root").as_str(), Some("keep-root"));
    let auth = map_value(map, "authorization").as_mapping().expect("auth");
    assert_eq!(map_value(auth, "vendor_extension").as_str(), Some("keep-auth"));
    assert_eq!(map_value(auth, "token").as_str(), Some(secret));
    let body = map_value(map, "body").as_mapping().expect("body");
    assert_eq!(map_value(body, "vendor_body").as_str(), Some("keep-body"));
}

#[test]
fn http_request_validation_rejects_invalid_url_timeout_and_empty_auth_without_secret() {
    let draft = WorkflowNodeVisualDraft::HttpRequest {
        method: "get".into(),
        url: "file:///etc/passwd".into(),
        auth_type: "bearer".into(),
        auth_username: String::new(),
        auth_password: "do-not-leak".into(),
        auth_token: String::new(),
        headers_editor: text_editor::Content::with_text(""),
        params_editor: text_editor::Content::with_text(""),
        body_type: "none".into(),
        body_editor: text_editor::Content::with_text(""),
        timeout_connect: "0".into(),
        timeout_read: "bad".into(),
        timeout_write: "20".into(),
        ssl_verify: true,
        retry_config: WorkflowNodeRetryDraft { enabled: true, max_retries: 0, retry_interval: 50 },
        error_strategy: "none".into(),
    };
    let validation = validate_node_editor_draft("http-request", "", "", "{}", Some(&draft));
    let debug = format!("{validation:?}");
    assert!(validation.field_errors.iter().any(|e| e.path == "http.url"));
    assert!(validation.field_errors.iter().any(|e| e.path == "http.authorization"));
    assert!(validation.field_errors.iter().any(|e| e.path == "http.timeout.connect"));
    assert!(!debug.contains("do-not-leak"));
}
