//! WhatsApp 验证查询的可选字段测试。

use super::WhatsAppVerifyQuery;

/// 验证 WhatsApp 验证查询的所有字段均为可选
///
/// 此测试确保 `WhatsAppVerifyQuery` 结构体的所有字段
/// （mode、verify_token、challenge）都可以为 None，
/// 以支持 WhatsApp webhook 验证流程的灵活性。
#[test]
fn whatsapp_query_fields_are_optional() {
    let q = WhatsAppVerifyQuery { mode: None, verify_token: None, challenge: None };
    assert!(q.mode.is_none());
}
