//! 验证聊天 DTO 的协议名称、默认值和推理强度兼容性。

use super::chat::{
    ChatContextDto, ChatOptionsDto, ChatRole, GatewayChatStreamEvent, GatewayChatStreamRequest,
    MessagePartDto, MessageStatus, ReasoningEffortDto,
};
use serde_json::json;

// 聊天角色、消息状态和带标签消息片段必须使用网关约定的 snake_case 名称。
#[test]
fn chat_enums_use_snake_case_protocol_names() {
    assert_eq!(serde_json::to_value(ChatRole::Assistant).expect("role"), json!("assistant"));
    assert_eq!(serde_json::to_value(MessageStatus::Streaming).expect("status"), json!("streaming"));
    assert_eq!(
        serde_json::to_value(MessagePartDto::Text { text: "hi".into() }).expect("part"),
        json!({"type": "text", "text": "hi"})
    );
}

// 默认请求不应凭空选择模型、文件或消息内容。
#[test]
fn chat_defaults_keep_optional_fields_empty() {
    let request = GatewayChatStreamRequest::default();
    assert!(request.messages.is_empty());
    assert_eq!(ChatOptionsDto::default().model, None);
    assert!(ChatContextDto::default().selected_file_paths.is_empty());
    assert_eq!(
        GatewayChatStreamEvent::Error("bad".into()),
        GatewayChatStreamEvent::Error("bad".into())
    );
}

// 推理强度在聊天选项和流请求中共享同一枚举，并拒绝未知协议值。
#[test]
fn reasoning_effort_dto_roundtrips_known_values_and_default() {
    let cases = [
        (None, None),
        (Some(ReasoningEffortDto::None), Some("none")),
        (Some(ReasoningEffortDto::Low), Some("low")),
        (Some(ReasoningEffortDto::Medium), Some("medium")),
        (Some(ReasoningEffortDto::High), Some("high")),
        (Some(ReasoningEffortDto::Xhigh), Some("xhigh")),
        (Some(ReasoningEffortDto::Max), Some("max")),
    ];

    for (reasoning_effort, serialized) in cases {
        let options = ChatOptionsDto { reasoning_effort, ..ChatOptionsDto::default() };
        let options_json = serde_json::to_value(&options).expect("serialize chat options");
        assert_eq!(
            options_json.get("reasoning_effort").cloned(),
            serialized.map(|value| json!(value))
        );
        assert_eq!(
            serde_json::from_value::<ChatOptionsDto>(options_json)
                .expect("deserialize chat options")
                .reasoning_effort,
            reasoning_effort
        );

        let request =
            GatewayChatStreamRequest { reasoning_effort, ..GatewayChatStreamRequest::default() };
        let request_json = serde_json::to_value(&request).expect("serialize gateway request");
        assert_eq!(
            request_json.get("reasoning_effort").cloned(),
            serialized.map(|value| json!(value))
        );
        assert_eq!(
            serde_json::from_value::<GatewayChatStreamRequest>(request_json)
                .expect("deserialize gateway request")
                .reasoning_effort,
            reasoning_effort
        );
    }

    assert!(serde_json::from_value::<ReasoningEffortDto>(json!("invalid")).is_err());
}
