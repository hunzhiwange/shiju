//! 验证环境、超时、优先级及 shell 包装层的安全剥离规则。

use super::*;
use crate::tools::shell::ast::CommandInfo;

fn info(name: &str, args: &[&str]) -> CommandInfo {
    CommandInfo {
        name: name.into(),
        args: args.iter().map(|arg| (*arg).into()).collect(),
        ..CommandInfo::default()
    }
}

#[test]
fn strip_wrappers_removes_env_and_timeout_layers() {
    let stripped =
        strip_wrappers(&info("env", &["RUST_LOG=debug", "timeout", "5", "git", "status"]));
    assert_eq!(stripped.name, "git");
    assert_eq!(stripped.args, vec!["status".to_string()]);
}

#[test]
fn strip_env_wrapper_skips_unset_forms_separator_and_options() {
    let stripped =
        strip_wrappers(&info("env", &["-uPATH", "--ignore-environment", "--", "python", "-V"]));

    assert_eq!(stripped.name, "python");
    assert_eq!(stripped.args, vec!["-V".to_string()]);
}

#[test]
fn wrapper_helpers_classify_env_assignments_and_nice_priorities() {
    assert!(is_env_assignment("KEY=value"));
    assert!(!is_env_assignment("=value"));
    assert!(is_nice_priority("-10"));
    assert!(!is_nice_priority("--flag"));
}

#[test]
fn strip_wrappers_parses_shell_command_string_and_nested_wrapper() {
    let stripped = strip_wrappers(&info("bash", &["-lc", "env MODE=test nice -n 5 git status"]));

    assert_eq!(stripped.name, "git");
    assert_eq!(stripped.args, vec!["status".to_string()]);
}

#[test]
fn strip_timeout_wrapper_skips_long_options_separator_and_unknown_flags() {
    let stripped = strip_wrappers(&info(
        "timeout",
        &[
            "--foreground",
            "--preserve-status",
            "--signal=TERM",
            "--kill-after=5s",
            "-v",
            "10s",
            "cargo",
            "test",
        ],
    ));

    assert_eq!(stripped.name, "cargo");
    assert_eq!(stripped.args, vec!["test".to_string()]);

    let stripped = strip_wrappers(&info("timeout", &["--", "cargo", "check"]));

    assert_eq!(stripped.name, "cargo");
    assert_eq!(stripped.args, vec!["check".to_string()]);
}

#[test]
fn strip_nice_wrapper_skips_separator_adjustment_and_unknown_flags() {
    let stripped = strip_wrappers(&info("nice", &["--adjustment=7", "-5", "-q", "cargo", "build"]));

    assert_eq!(stripped.name, "cargo");
    assert_eq!(stripped.args, vec!["build".to_string()]);

    let stripped = strip_wrappers(&info("nice", &["--", "cargo", "fmt"]));

    assert_eq!(stripped.name, "cargo");
    assert_eq!(stripped.args, vec!["fmt".to_string()]);
}

#[test]
fn strip_shell_wrapper_handles_separator_options_direct_and_unparseable_command() {
    let stripped = strip_wrappers(&info("sh", &["--", "cargo", "check"]));

    assert_eq!(stripped.name, "cargo");
    assert_eq!(stripped.args, vec!["check".to_string()]);

    let stripped = strip_wrappers(&info("zsh", &["-e", "cargo", "test"]));

    assert_eq!(stripped.name, "cargo");
    assert_eq!(stripped.args, vec!["test".to_string()]);

    let wrapper = info("bash", &["-c", ""]);
    let stripped = strip_wrappers(&wrapper);

    assert_eq!(stripped.name, wrapper.name);
    assert_eq!(stripped.args, wrapper.args);
}

#[test]
fn strip_wrappers_keeps_incomplete_wrapper_for_safe_classification() {
    for wrapper in [
        info("env", &["-u", "NAME"]),
        info("timeout", &["--signal", "TERM"]),
        info("nice", &["--adjustment", "2"]),
        info("sh", &["-c"]),
        info("nohup", &["--"]),
    ] {
        let stripped = strip_wrappers(&wrapper);
        assert_eq!(stripped.name, wrapper.name);
        assert_eq!(stripped.args, wrapper.args);
    }
}

#[test]
fn strip_generic_wrapper_skips_options_and_separator() {
    let stripped = strip_wrappers(&info("stdbuf", &["-oL", "--", "cargo", "metadata"]));

    assert_eq!(stripped.name, "cargo");
    assert_eq!(stripped.args, vec!["metadata".to_string()]);
}

#[test]
fn strip_generic_wrapper_keeps_direct_command_arguments() {
    let stripped = strip_wrappers(&info("nohup", &["cargo", "test"]));

    assert_eq!(stripped.name, "cargo");
    assert_eq!(stripped.args, vec!["test".to_string()]);
}

#[test]
fn strip_wrappers_preserves_command_security_metadata() {
    let mut wrapped = info("timeout", &["5", "echo", "ok"]);
    wrapped.has_command_substitution = true;
    wrapped.has_process_substitution = true;
    wrapped.has_glob = true;
    wrapped.has_variable_expansion = true;

    let stripped = strip_wrappers(&wrapped);

    assert_eq!(stripped.name, "echo");
    assert!(stripped.has_command_substitution);
    assert!(stripped.has_process_substitution);
    assert!(stripped.has_glob);
    assert!(stripped.has_variable_expansion);
}

#[test]
fn wrapper_helpers_reject_invalid_identifiers_and_priorities() {
    assert!(is_env_assignment("_KEY9=value"));
    assert!(!is_env_assignment("9KEY=value"));
    assert!(!is_env_assignment("BAD-NAME=value"));
    assert!(!is_env_assignment("KEY"));
    assert!(!is_nice_priority("-"));
    assert!(!is_nice_priority("-1x"));
}
