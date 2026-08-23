//! 验证代理环境变量存在时代理状态探测结果保持为真。
//! 用例只在原环境未配置时移除临时变量，避免破坏调用者已有设置。

use super::proxied::proxied;

#[test]
fn proxied_reflects_proxy_environment_presence() {
    let before = proxied();
    unsafe { std::env::set_var("HTTP_PROXY", "http://127.0.0.1:1") };
    assert!(proxied());
    if !before {
        unsafe { std::env::remove_var("HTTP_PROXY") };
    }
}
