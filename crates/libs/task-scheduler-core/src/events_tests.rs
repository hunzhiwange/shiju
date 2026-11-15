//! 测试调度事件种类解析对未知存储值返回包含原输入的明确错误。
//! 该测试锁定持久化数据损坏时的诊断契约，不为未知值提供伪兼容。

use super::AgentEventKind;

#[test]
fn agent_event_kind_parse_rejects_unknown_kind_with_input_context() {
    let error = AgentEventKind::parse("future_event").expect_err("unknown event kind must fail");

    assert_eq!(error.to_string(), "unsupported agent event kind: future_event");
}
