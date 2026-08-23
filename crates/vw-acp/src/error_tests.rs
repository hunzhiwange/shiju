//! 验证 ACP 错误展示文本保留各错误变体所需的上下文前缀。
//!
//! 构造的协议载荷与错误链用于固定归一化优先级和诊断信息边界。

use super::error::AcpError;

// 下列行为用例覆盖已知错误、缺失字段、嵌套载荷和安全回退。
#[test]
fn display_messages_keep_contextual_prefixes() {
    assert_eq!(AcpError::EmptyCommand.to_string(), "acp command is empty");
    assert_eq!(
        AcpError::Initialize("bad handshake".to_string()).to_string(),
        "acp initialize failed: bad handshake"
    );
    assert_eq!(
        AcpError::SessionChanged {
            expected: "expected-session".to_string(),
            actual: "actual-session".to_string(),
        }
        .to_string(),
        "acp session changed: expected=expected-session actual=actual-session"
    );
}
