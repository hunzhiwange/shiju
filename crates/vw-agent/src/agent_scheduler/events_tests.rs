//! 测试调度事件的构造、序列化与领域字段保留。

use super::AgentEventKind;

#[test]
fn agent_event_kind_parse_rejects_unknown_kind_with_input_context() {
    let error = AgentEventKind::parse("future_event").expect_err("unknown event kind must fail");

    assert_eq!(error.to_string(), "unsupported agent event kind: future_event");
}
