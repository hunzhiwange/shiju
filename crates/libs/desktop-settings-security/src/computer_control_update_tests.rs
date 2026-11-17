use super::safe_error;
use gateway_computer_control::ComputerControlApiError;

#[test]
fn unauthorized_error_explains_required_gateway_credentials() {
    let message = safe_error(ComputerControlApiError::Http(401));
    assert!(message.contains("当前网关已启用鉴权"));
    assert!(message.contains("检查网关连接设置中的 skey"));
}

#[test]
fn other_errors_preserve_status_without_authentication_guidance() {
    let message = safe_error(ComputerControlApiError::Http(503));
    assert!(message.contains("HTTP 503"));
    assert!(!message.contains("SKey"));
}
