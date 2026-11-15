//! 验证 API 错误载荷能够保留或省略可选详情。

use crate::error::{ApiErrorBody, ApiErrorDetail};
use serde_json::json;

// 错误详情存在时必须原样保留，缺失时则从响应中省略而非写入空对象。
#[test]
fn api_error_body_preserves_optional_details() {
    let body: ApiErrorBody = serde_json::from_value(json!({
        "error": {
            "code": "invalid_input",
            "message": "bad path",
            "details": { "field": "path" }
        }
    }))
    .expect("valid error body");

    assert_eq!(body.error.code, "invalid_input");
    assert_eq!(body.error.details, Some(json!({ "field": "path" })));
    assert_eq!(
        serde_json::to_value(ApiErrorBody {
            error: ApiErrorDetail {
                code: "x".to_string(),
                message: "missing".to_string(),
                details: None,
            },
        })
        .expect("serialize"),
        json!({ "error": { "code": "x", "message": "missing" } })
    );
}
