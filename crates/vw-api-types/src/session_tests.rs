//! 验证会话创建默认值及网关字段重命名契约。

use crate::session::{
    CreateSessionRequest, GatewaySessionCreateBody, GatewaySessionMessagePageQuery,
    GatewaySessionUiSummary, SessionMessagePageDirection, SessionMetadataDto, SessionStatus,
};
use serde_json::json;

// 会话创建保持空项目和空元数据默认值，同时固化网关使用的驼峰字段名。
#[test]
fn session_create_defaults_and_gateway_renames_are_stable() {
    let request: CreateSessionRequest = serde_json::from_value(json!({})).expect("valid create");
    assert_eq!(request.project_id, None);
    assert_eq!(request.metadata, SessionMetadataDto::default());

    let gateway = GatewaySessionCreateBody {
        parent_id: Some("parent-1".to_string()),
        title: Some("Title".to_string()),
    };
    assert_eq!(
        serde_json::to_value(gateway).expect("serialize"),
        json!({ "parentID": "parent-1", "title": "Title" })
    );
    assert_eq!(
        serde_json::to_value(SessionStatus::Archived).expect("serialize"),
        json!("archived")
    );
}

#[test]
fn session_message_page_query_defaults_to_bounded_older_page() {
    let query: GatewaySessionMessagePageQuery = serde_json::from_value(json!({})).unwrap();
    assert_eq!(query.limit, None);
    assert_eq!(query.direction, SessionMessagePageDirection::Older);
}

#[test]
fn session_message_page_summary_omits_heavy_payloads() {
    let value = serde_json::to_value(GatewaySessionUiSummary {
        id: "s1".into(),
        title: "title".into(),
        updated_ms: 7,
        message_count: 10_000,
        call_count: 2,
        last_content: Some("tail".into()),
    })
    .unwrap();
    assert!(value.get("messages").is_none());
    assert!(value.get("calls").is_none());
    assert!(value.get("steps").is_none());
}
