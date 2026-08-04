//! shell 安全 validator 流水线的回归测试。
//!
//! 覆盖每个 validator 的代表性阻断/警告场景，确保新增规则不会无意削弱已有安全边界。

use super::SecurityPipeline;
use super::injection::has_blocking_injection;
use super::obfuscation::has_blocking_obfuscation;

fn pipeline() -> SecurityPipeline {
    SecurityPipeline::new_with_quoted_newline_blocking(true, true)
}

#[test]
fn blocks_empty_command() {
    let report = pipeline().validate_command("   ");
    assert!(report.blocked);
}

#[test]
fn blocks_unquoted_heredoc_marker() {
    let report = pipeline().validate_command("cat <<EOF");
    assert!(report.blocked);
}

#[test]
fn allows_quoted_heredoc_marker() {
    let report = pipeline().validate_command("cat <<'EOF'");
    assert!(!report.blocked);
}

#[test]
fn blocks_git_commit_message_substitution() {
    let report = pipeline().validate_command("git commit -m \"$(whoami)\"");
    assert!(has_blocking_injection(&report));
}

#[test]
fn blocks_dangerous_jq_keywords() {
    let report = pipeline().validate_command("jq 'def x: .; x'");
    assert!(report.blocked);
}

#[test]
fn blocks_eval_metacharacters() {
    let report = pipeline().validate_command("eval echo hi");
    assert!(report.blocked);
}

#[test]
fn flags_dangerous_vars() {
    let report = pipeline().validate_command("echo $BASH_EXECUTION_STRING");
    assert!(report.blocked);
}

#[test]
fn blocks_command_substitution_in_strict_mode() {
    let report = pipeline().validate_command("echo $(whoami)");
    assert!(report.blocked);
}

#[test]
fn warns_on_regular_output_redirection() {
    let report = pipeline().validate_command("echo hi > out.txt");
    assert!(!report.blocked);
    assert!(!report.findings.is_empty());
}

#[test]
fn blocks_newline_injection() {
    let report = SecurityPipeline::new(true).validate_command("echo hi\nrm -rf /");
    assert!(report.blocked);
}

#[test]
fn allows_quoted_shell_newlines_when_quoted_newline_blocking_is_disabled() {
    let report = SecurityPipeline::new(true).validate_command("git commit -m \"title\n\nbody\"");

    assert!(!report.blocked);
}

#[test]
fn blocks_ifs_injection() {
    let report = pipeline().validate_command("IFS=/; echo test");
    assert!(report.blocked);
}

#[test]
fn blocks_proc_environ_access() {
    let report = pipeline().validate_command("cat /proc/self/environ");
    assert!(report.blocked);
}

#[test]
fn blocks_malformed_command() {
    let report = pipeline().validate_command("echo \"unterminated");
    assert!(report.blocked);
}

#[test]
fn blocks_obfuscated_flags() {
    let report = pipeline().validate_command("rm $'\\x2d\\x2drf' target");
    assert!(has_blocking_obfuscation(&report));
}

#[test]
fn blocks_backslash_obfuscation() {
    let report = pipeline().validate_command("echo hello\\ world");
    assert!(report.blocked);
}

#[test]
fn blocks_brace_expansion() {
    let report = pipeline().validate_command("echo {1..3}");
    assert!(report.blocked);
}

#[test]
fn blocks_unicode_whitespace() {
    let report = pipeline().validate_command("echo\u{3000}hello");
    assert!(report.blocked);
}

#[test]
fn blocks_hash_comment_trick() {
    let report = pipeline().validate_command("echo#hidden");
    assert!(report.blocked);
}

#[test]
fn blocks_comment_quote_desync() {
    let report = pipeline().validate_command("echo \"foo # bar");
    assert!(report.blocked);
}

#[test]
fn blocks_quoted_newline() {
    let report = pipeline().validate_command("echo \"hello\nworld\"");
    assert!(report.blocked);
}

#[test]
fn blocks_zsh_only_expansion() {
    let report = pipeline().validate_command("=git status");
    assert!(report.blocked);
}

#[test]
fn blocks_control_characters() {
    let report = pipeline().validate_command("echo hi\u{0007}");
    assert!(report.blocked);
}
