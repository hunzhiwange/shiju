//! 沙箱自动放行测试，覆盖只读命令、编辑命令和沙箱开关边界。

use crate::tools::shell::ast::{ParsedCommand, parse_command};

use super::sandbox_allow::SandboxAutoAllow;

#[test]
fn sandbox_auto_allow_requires_sandbox() {
    let allow = SandboxAutoAllow::default();
    assert!(!allow.should_auto_allow(&parse_command("ls -la"), false));
}

#[test]
fn sandbox_auto_allow_allows_readonly_commands() {
    let allow = SandboxAutoAllow::default();
    assert!(allow.should_auto_allow(&parse_command("cat Cargo.toml"), true));
    assert!(allow.should_auto_allow(&parse_command("git status --short"), true));
}

#[test]
fn sandbox_auto_allow_rejects_excluded_commands() {
    let allow = SandboxAutoAllow::with_excluded_commands(vec!["cat".into()]);
    assert!(!allow.should_auto_allow(&parse_command("cat Cargo.toml"), true));
}

#[test]
fn sandbox_auto_allow_allows_expression_interpreters() {
    let allow = SandboxAutoAllow::default();
    assert!(allow.should_auto_allow(&parse_command("python3 -c 'print(1)'"), true));
    assert!(allow.should_auto_allow(&parse_command("node -e 'console.log(1)'"), true));
}

#[test]
fn sandbox_auto_allow_rejects_empty_fallback_command() {
    let allow = SandboxAutoAllow::default();
    let command = ParsedCommand::Fallback { raw: String::new(), tokens: Vec::new() };

    assert!(!allow.should_auto_allow(&command, true));
}

#[test]
fn sandbox_auto_allow_rejects_interpreter_script_files() {
    let allow = SandboxAutoAllow::default();

    assert!(!allow.should_auto_allow(&parse_command("python3 script.py"), true));
    assert!(!allow.should_auto_allow(&parse_command("node script.js"), true));
}
