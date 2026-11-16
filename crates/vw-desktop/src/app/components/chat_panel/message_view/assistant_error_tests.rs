//! 验证助手错误载荷识别、链接提取以及可重试错误卡片构造。
//! URL fixture 覆盖引号、括号、空白和消息末尾，避免把相邻标点并入链接。

use super::assistant_error::assistant_api_error_view;
use crate::app::App;

#[test]
fn assistant_api_error_view_rejects_invalid_or_unrelated_json() {
    let app = App::new().0;

    assert!(assistant_api_error_view(&app, 0, None, "not json").is_none());
    assert!(
        assistant_api_error_view(&app, 0, None, r#"{"name":"OtherError","message":"bad"}"#,)
            .is_none()
    );
    assert!(assistant_api_error_view(&app, 0, None, r#"{"name":"APIError"}"#).is_none());
    assert!(
        assistant_api_error_view(&app, 0, None, r#"{"name":"APIError","message":"   "}"#,)
            .is_none()
    );
}

#[test]
fn assistant_api_error_view_accepts_retryable_error_with_url() {
    let app = App::new().0;
    let content = r#"{"name":"APIError","message":"request to https://api.example.test/v1 failed","is_retryable":true}"#;

    let view = assistant_api_error_view(&app, 0, None, content);

    assert!(view.is_some());
}

#[test]
fn assistant_api_error_view_accepts_non_retryable_special_text_error() {
    let app = App::new().0;
    let content = r#"{"name":"APIError","message":"invalid API key","is_retryable":false}"#;

    let view = assistant_api_error_view(&app, 3, Some(2), content);

    assert!(view.is_some());
}

#[test]
fn assistant_api_error_view_accepts_error_without_retryability_hint() {
    let app = App::new().0;
    let content = r#"{"name":"APIError","message":"network unavailable"}"#;

    let view = assistant_api_error_view(&app, 0, None, content);

    assert!(view.is_some());
}
