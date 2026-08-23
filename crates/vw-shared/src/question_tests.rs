//! 交互问题请求的前端线协议与拒绝错误测试。
//!
//! 验证驼峰字段名、空选项省略和稳定的用户取消提示。

#[test]
fn request_serialization_uses_frontend_field_names() {
    let request = super::Request {
        id: "req-1".to_string(),
        session_id: "session-1".to_string(),
        questions: vec![super::Info {
            question: "Pick one".to_string(),
            header: "Mode".to_string(),
            options: vec![super::OptionInfo {
                label: "Fast".to_string(),
                description: "Use defaults".to_string(),
                preview: None,
            }],
            multiple: None,
            custom: Some(true),
        }],
        tool: Some(super::ToolMeta {
            message_id: "msg-1".to_string(),
            call_id: "call-1".to_string(),
        }),
    };

    let value = serde_json::to_value(request).unwrap();

    assert_eq!(value["sessionID"], "session-1");
    assert_eq!(value["tool"]["messageID"], "msg-1");
    assert_eq!(value["tool"]["callID"], "call-1");
    assert!(value["questions"][0].get("multiple").is_none());
    assert_eq!(value["questions"][0]["custom"], true);
}

#[test]
fn rejected_error_message_is_stable() {
    assert_eq!(super::RejectedError.to_string(), "The user dismissed this question");
}
