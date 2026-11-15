// 验证网关 API 请求与响应类型的必填字段和序列化结构。

// 类型测试保持 JSON 契约最小且明确，防止可选字段意外替代必填字段。
use super::*;

#[test]
fn memory_store_body_requires_key_and_content() {
    let body: MemoryStoreBody = serde_json::from_value(serde_json::json!({
        "key": "theme",
        "content": "dark",
        "category": "prefs"
    }))
    .expect("valid body");

    assert_eq!(body.key, "theme");
    assert_eq!(body.category.as_deref(), Some("prefs"));
}

#[test]
fn integration_settings_payload_serializes_integrations() {
    let payload = IntegrationSettingsPayload {
        revision: "1".to_string(),
        active_default_provider_integration_id: None,
        integrations: Vec::new(),
    };
    let value = serde_json::to_value(payload).expect("serializable payload");

    assert_eq!(value.get("revision").and_then(|v| v.as_str()), Some("1"));
    assert_eq!(value.get("integrations").and_then(|v| v.as_array()).map(Vec::len), Some(0));
}
