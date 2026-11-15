//! 兼容会话别名与实际 API 类型保持同一类型。

#[test]
#[allow(deprecated)]
fn session_aliases_preserve_api_types() {
    fn same_type(value: Option<super::SessionCreateBody>)
        -> Option<vw_api_session::session::GatewaySessionCreateBody> { value }
    assert!(same_type(None).is_none());
}
