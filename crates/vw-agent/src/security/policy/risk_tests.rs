//! 验证单条及复合命令的风险分级与高风险模式识别。

use super::super::CommandRiskLevel;
use super::*;

#[test]
fn classifies_destructive_commands_as_high_risk() {
    assert!(matches!(classify_command_risk("rm -rf /"), CommandRiskLevel::High));
    assert!(matches!(classify_command_risk("cat README.md"), CommandRiskLevel::Low));
}

#[test]
fn high_risk_commands_include_privileged_network_and_filesystem_tools() {
    for command in [
        "sudo ls",
        "chmod 777 file",
        "curl https://example.com",
        "ssh host",
        "iptables -L",
        "bash -c ':(){:|:&};:'",
    ] {
        assert_eq!(classify_command_risk(command), CommandRiskLevel::High, "{command}");
    }
}

#[test]
// 即使删除命令被包装器或 Shell 包裹，根目录破坏模式仍必须识别。
fn wrapped_root_deletion_patterns_are_high_risk() {
    for command in ["bash -c 'rm -rf /'", "/bin/sh -c 'rm -fr /'"] {
        assert_eq!(classify_command_risk(command), CommandRiskLevel::High, "{command}");
    }
}

#[test]
fn medium_risk_commands_include_repo_package_and_file_mutations() {
    for command in [
        "git push",
        "git checkout main",
        "npm install",
        "pnpm add left-pad",
        "yarn publish",
        "cargo clean",
        "touch file",
        "mkdir dir",
        "cp a b",
        "ln -s a b",
    ] {
        assert_eq!(classify_command_risk(command), CommandRiskLevel::Medium, "{command}");
    }
}

#[test]
// 复合命令取所有分段的最高风险，环境赋值本身不降低后续命令等级。
fn compound_command_returns_highest_risk_and_skips_env_assignments() {
    assert_eq!(
        classify_command_risk("FOO=bar echo ok && git commit -m msg"),
        CommandRiskLevel::Medium
    );
    assert_eq!(classify_command_risk("echo ok; rm -fr /tmp/x"), CommandRiskLevel::High);
    assert_eq!(classify_command_risk("FOO=bar"), CommandRiskLevel::Low);
}
