#[cfg(not(all(feature = "sandbox-landlock", target_os = "linux")))]
use std::process::Command;

use crate::Sandbox;
use crate::landlock::LandlockSandbox;

// 测试 landlock 沙箱的名称是否正确返回 "landlock"
#[cfg(all(feature = "sandbox-landlock", target_os = "linux"))]
#[test]
fn landlock_sandbox_name() {
    if let Ok(sandbox) = LandlockSandbox::new() {
        assert_eq!(sandbox.name(), "landlock");
    }
}

// 测试在非 Linux 系统或未启用 landlock 特性时沙箱不可用
#[cfg(not(all(feature = "sandbox-landlock", target_os = "linux")))]
#[test]
fn landlock_not_available_on_non_linux() {
    assert!(!LandlockSandbox.is_available());
    assert_eq!(LandlockSandbox.name(), "landlock");
}

// 测试使用 None 工作空间创建 landlock 沙箱的行为
#[test]
fn landlock_with_none_workspace() {
    let result = LandlockSandbox::with_workspace(None);
    match result {
        Ok(sandbox) => assert!(sandbox.is_available()),
        Err(_) => assert!(!cfg!(all(feature = "sandbox-landlock", target_os = "linux"))),
    }
}

// 测试在非 Linux 系统上 stub 实现的 wrap_command 方法返回 Unsupported 错误
#[cfg(not(all(feature = "sandbox-landlock", target_os = "linux")))]
#[test]
fn landlock_stub_wrap_command_returns_unsupported() {
    let sandbox = LandlockSandbox;
    let mut cmd = Command::new("echo");
    let result = sandbox.wrap_command(&mut cmd);
    assert!(result.is_err());
    assert_eq!(result.unwrap_err().kind(), std::io::ErrorKind::Unsupported);
}

// 测试在非 Linux 系统上 stub 实现的 new() 方法返回 Unsupported 错误
#[cfg(not(all(feature = "sandbox-landlock", target_os = "linux")))]
#[test]
fn landlock_stub_new_returns_unsupported() {
    let result = LandlockSandbox::new();
    assert!(result.is_err());
    assert_eq!(result.unwrap_err().kind(), std::io::ErrorKind::Unsupported);
}

// 测试在非 Linux 系统上 stub 实现的 probe() 方法返回错误
#[cfg(not(all(feature = "sandbox-landlock", target_os = "linux")))]
#[test]
fn landlock_stub_probe_returns_unsupported() {
    let result = LandlockSandbox::probe();
    assert!(result.is_err());
}
