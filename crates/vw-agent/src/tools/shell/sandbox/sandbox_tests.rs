//! shell sandbox 决策逻辑测试。
//!
//! 覆盖全局禁用、上下文覆盖和包装命令排除，确保是否启用沙箱的原因保持可解释。

use std::path::PathBuf;

use crate::tools::shell::ast::{ParsedCommand, parse_command};

use super::{
    SandboxConfig, SandboxReason, is_env_assignment, is_nice_priority, should_use_sandbox,
    strip_wrapper_tokens, stripped_command_name,
};

fn tokens(values: &[&str]) -> Vec<String> {
    values.iter().map(|value| (*value).to_string()).collect()
}

#[test]
fn sandbox_disabled_globally_short_circuits() {
    let mut config = SandboxConfig::for_workspace(PathBuf::from("/workspace"));
    config.enabled = false;

    let decision = should_use_sandbox(&parse_command("cat README.md"), &config);

    assert!(!decision.use_sandbox);
    assert_eq!(decision.reason, SandboxReason::DisabledGlobally);
}

#[test]
fn sandbox_global_disable_takes_precedence_over_override() {
    let mut config = SandboxConfig::for_workspace(PathBuf::from("/workspace"));
    config.enabled = false;
    config.allow_override = true;
    config.override_enabled = true;

    let decision = should_use_sandbox(&parse_command("cat README.md"), &config);

    assert!(!decision.use_sandbox);
    assert_eq!(decision.reason, SandboxReason::DisabledGlobally);
}

#[test]
fn sandbox_override_disables_execution() {
    let mut config = SandboxConfig::for_workspace(PathBuf::from("/workspace"));
    config.allow_override = true;
    config.override_enabled = true;

    let decision = should_use_sandbox(&parse_command("cat README.md"), &config);

    assert!(!decision.use_sandbox);
    assert_eq!(decision.reason, SandboxReason::DisabledByOverride);
}

#[test]
fn sandbox_override_requires_explicit_permission() {
    let mut config = SandboxConfig::for_workspace(PathBuf::from("/workspace"));
    config.allow_override = false;
    config.override_enabled = true;

    let decision = should_use_sandbox(&parse_command("cat README.md"), &config);

    if super::executor::SandboxExecutor::backend_available() {
        assert!(decision.use_sandbox);
        assert_eq!(decision.reason, SandboxReason::EnabledByDefault);
    } else {
        assert!(!decision.use_sandbox);
        assert_eq!(decision.reason, SandboxReason::BackendUnavailable);
    }
}

#[test]
fn sandbox_excluded_command_matches_wrapped_shell_command() {
    let mut config = SandboxConfig::for_workspace(PathBuf::from("/workspace"));
    config.excluded_commands = vec!["rg".into()];

    let decision =
        should_use_sandbox(&parse_command("env FOO=bar bash -lc 'rg needle src'"), &config);

    if super::executor::SandboxExecutor::backend_available() {
        assert!(!decision.use_sandbox);
        assert_eq!(decision.reason, SandboxReason::DisabledForCommand);
    } else {
        assert!(!decision.use_sandbox);
        assert_eq!(decision.reason, SandboxReason::BackendUnavailable);
    }
}

#[test]
fn sandbox_excluded_command_match_is_ascii_case_insensitive() {
    let mut config = SandboxConfig::for_workspace(PathBuf::from("/workspace"));
    config.excluded_commands = vec!["RG".into()];

    let decision = should_use_sandbox(&parse_command("rg needle src"), &config);

    if super::executor::SandboxExecutor::backend_available() {
        assert!(!decision.use_sandbox);
        assert_eq!(decision.reason, SandboxReason::DisabledForCommand);
    } else {
        assert!(!decision.use_sandbox);
        assert_eq!(decision.reason, SandboxReason::BackendUnavailable);
    }
}

#[test]
fn stripped_command_name_handles_ast_and_fallback_commands() {
    assert_eq!(stripped_command_name(&parse_command("env MODE=test rg needle")), Some("rg".into()));

    let fallback = ParsedCommand::Fallback {
        raw: "env MODE=test timeout 5 nice -10 rg needle".into(),
        tokens: tokens(&["env", "MODE=test", "timeout", "5", "nice", "-10", "rg", "needle"]),
    };
    assert_eq!(stripped_command_name(&fallback), Some("rg".into()));

    let empty = ParsedCommand::Fallback { raw: String::new(), tokens: Vec::new() };
    assert_eq!(stripped_command_name(&empty), None);
}

#[test]
fn fallback_wrapper_stripping_covers_supported_argument_forms() {
    let cases = [
        (
            tokens(&["env", "-u", "OLD", "-uSTALE", "MODE=test", "--", "rg", "needle"]),
            tokens(&["rg", "needle"]),
        ),
        (
            tokens(&[
                "timeout",
                "-s",
                "TERM",
                "--kill-after=2s",
                "--foreground",
                "--preserve-status",
                "5s",
                "rg",
                "needle",
            ]),
            tokens(&["rg", "needle"]),
        ),
        (
            tokens(&["nice", "-n", "5", "--adjustment=-10", "--", "rg", "needle"]),
            tokens(&["rg", "needle"]),
        ),
        (tokens(&["bash", "-lc", "env MODE=test rg needle src"]), tokens(&["rg", "needle", "src"])),
        (tokens(&["sh", "-x", "--", "rg", "needle"]), tokens(&["rg", "needle"])),
        (tokens(&["stdbuf", "-oL", "--", "rg", "needle"]), tokens(&["rg", "needle"])),
    ];

    for (input, expected) in cases {
        assert_eq!(strip_wrapper_tokens(input), expected);
    }
}

#[test]
fn fallback_wrapper_stripping_preserves_incomplete_or_unparseable_commands() {
    let cases = [
        tokens(&["env", "-u", "OLD"]),
        tokens(&["timeout", "--signal", "TERM"]),
        tokens(&["nice", "--adjustment", "5"]),
        tokens(&["bash", "-c"]),
        tokens(&["bash", "-c", "'unterminated"]),
    ];

    for input in cases {
        assert_eq!(strip_wrapper_tokens(input.clone()), input);
    }
}

#[test]
fn fallback_wrapper_stripping_leaves_empty_and_unwrapped_tokens_unchanged() {
    assert!(strip_wrapper_tokens(Vec::new()).is_empty());
    assert_eq!(strip_wrapper_tokens(tokens(&["rg", "needle"])), tokens(&["rg", "needle"]));
}

#[test]
fn wrapper_token_classifiers_enforce_identifier_and_priority_boundaries() {
    for value in ["KEY=value", "_KEY9=value", "KEY=value=with-equals"] {
        assert!(is_env_assignment(value), "expected valid assignment: {value}");
    }
    for value in ["KEY", "=value", "9KEY=value", "KEY-NAME=value"] {
        assert!(!is_env_assignment(value), "expected invalid assignment: {value}");
    }

    for value in ["-0", "-10"] {
        assert!(is_nice_priority(value), "expected valid priority: {value}");
    }
    for value in ["", "-", "10", "--10", "-1x"] {
        assert!(!is_nice_priority(value), "expected invalid priority: {value}");
    }
}
