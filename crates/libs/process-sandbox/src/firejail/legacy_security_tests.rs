use std::process::Command;

use crate::Sandbox;
use crate::firejail::FirejailSandbox;

// 测试 FirejailSandbox 返回正确的沙箱名称
#[test]
fn firejail_sandbox_name() {
    assert_eq!(FirejailSandbox.name(), "firejail");
}

// 测试沙箱描述中包含 firejail 依赖信息
#[test]
fn firejail_description_mentions_dependency() {
    let desc = FirejailSandbox.description();
    assert!(desc.contains("firejail"));
}

// 测试在未安装 firejail 时创建实例会返回正确的错误
#[test]
fn firejail_new_fails_if_not_installed() {
    let result = FirejailSandbox::new();
    match result {
        Ok(_) => {}
        Err(e) => assert!(
            e.kind() == std::io::ErrorKind::NotFound || e.kind() == std::io::ErrorKind::Unsupported
        ),
    }
}

// 测试 wrap_command 将原命令包装为以 firejail 开头
#[test]
fn firejail_wrap_command_prepends_firejail() {
    let sandbox = FirejailSandbox;
    let mut cmd = Command::new("echo");
    cmd.arg("test");

    let _ = sandbox.wrap_command(&mut cmd);

    if sandbox.is_available() {
        assert_eq!(cmd.get_program().to_string_lossy(), "firejail");
    }
}

// 测试 wrap_command 包含所有必要的安全隔离标志
#[cfg(target_os = "linux")]
#[test]
fn firejail_wrap_command_includes_all_security_flags() {
    let sandbox = FirejailSandbox;
    if !sandbox.is_available() {
        return;
    }

    let mut cmd = Command::new("echo");
    cmd.arg("test");
    sandbox.wrap_command(&mut cmd).unwrap();

    assert_eq!(
        cmd.get_program().to_string_lossy(),
        "firejail",
        "wrapped command should use firejail as program"
    );

    let args: Vec<String> = cmd.get_args().map(|s| s.to_string_lossy().to_string()).collect();

    let expected_flags = [
        "--private=home",
        "--private-dev",
        "--nosound",
        "--no3d",
        "--novideo",
        "--nowheel",
        "--notv",
        "--noprofile",
        "--quiet",
    ];

    for flag in &expected_flags {
        assert!(args.contains(&flag.to_string()), "must include security flag: {flag}");
    }
}

// 测试 wrap_command 在包装后保留原始命令及其参数
#[cfg(target_os = "linux")]
#[test]
fn firejail_wrap_command_preserves_original_command() {
    let sandbox = FirejailSandbox;
    if !sandbox.is_available() {
        return;
    }

    let mut cmd = Command::new("ls");
    cmd.arg("-la");
    cmd.arg("/workspace");
    sandbox.wrap_command(&mut cmd).unwrap();

    let args: Vec<String> = cmd.get_args().map(|s| s.to_string_lossy().to_string()).collect();

    assert!(args.contains(&"ls".to_string()), "original program must be passed as argument");
    assert!(args.contains(&"-la".to_string()), "original args must be preserved");
    assert!(args.contains(&"/workspace".to_string()), "original args must be preserved");
}
