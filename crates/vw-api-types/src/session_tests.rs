//! 验证会话创建默认值及网关字段重命名契约。

use crate::session::{
    CreateSessionRequest, GatewaySessionCreateBody, SessionMetadataDto, SessionStatus,
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
