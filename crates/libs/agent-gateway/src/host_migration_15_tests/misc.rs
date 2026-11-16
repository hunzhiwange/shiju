use gateway_misc::{not_implemented, SkillInfo};
use axum::http::StatusCode;
#[tokio::test]
async fn not_implemented_returns_501() {
    let error = not_implemented().await.expect_err("endpoint is intentionally unavailable");

    assert_eq!(error.status, StatusCode::NOT_IMPLEMENTED);
    assert_eq!(error.message, "not implemented");
}

#[test]
fn skill_info_serializes_all_public_response_fields() {
    let info = SkillInfo {
        name: "review".to_string(),
        description: "Review a change".to_string(),
        location: "/workspace/review/SKILL.md".to_string(),
        content: "# Review".to_string(),
    };

    assert_eq!(
        serde_json::to_value(info).expect("skill info should serialize"),
        serde_json::json!({
            "name": "review",
            "description": "Review a change",
            "location": "/workspace/review/SKILL.md",
            "content": "# Review",
        })
    );
}
