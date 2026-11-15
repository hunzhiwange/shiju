use config_security::{SandboxBackend, SandboxConfig, SecurityConfig};
use crate::detect::{create_sandbox, detect_best_sandbox};

// 测试 detect_best_sandbox 函数是否返回可用的沙箱
#[test]
fn detect_best_sandbox_returns_something() {
    let sandbox = detect_best_sandbox();
    assert!(sandbox.is_available());
}

// 测试显式禁用沙箱时返回 noop 实现
#[test]
fn explicit_none_returns_noop() {
    let config = SecurityConfig {
        sandbox: SandboxConfig {
            enabled: Some(false),
            backend: SandboxBackend::None,
            firejail_args: Vec::new(),
        },
        ..Default::default()
    };
    let sandbox = create_sandbox(&config);
    assert_eq!(sandbox.name(), "none");
}

// 测试自动模式是否能检测到可用的沙箱
#[test]
fn auto_mode_detects_something() {
    let config = SecurityConfig {
        sandbox: SandboxConfig {
            enabled: None,
            backend: SandboxBackend::Auto,
            firejail_args: Vec::new(),
        },
        ..Default::default()
    };
    let sandbox = create_sandbox(&config);
    assert!(sandbox.is_available());
}
