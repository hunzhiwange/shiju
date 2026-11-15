//! Shell 命令解析公共入口测试，覆盖保守回退结果的访问器行为。

use super::{ParsedCommand, parse_command};

#[test]
fn fallback_with_valid_tokens_exposes_name_arguments_and_raw_input() {
    let parsed = parse_command("echo unexpected )");

    assert_eq!(parsed.raw(), "echo unexpected )");
    assert!(parsed.info().is_none());
    assert_eq!(parsed.command_name(), Some("echo"));
    assert_eq!(parsed.args(), &["unexpected".to_string(), ")".to_string()]);
    assert!(matches!(parsed, ParsedCommand::Fallback { .. }));
}

#[test]
fn fallback_with_invalid_shell_words_has_no_name_or_arguments() {
    let parsed = parse_command("echo 'unterminated");

    assert_eq!(parsed.raw(), "echo 'unterminated");
    assert!(parsed.info().is_none());
    assert_eq!(parsed.command_name(), None);
    assert!(parsed.args().is_empty());
    assert!(matches!(
        parsed,
        ParsedCommand::Fallback { ref tokens, .. } if tokens.is_empty()
    ));
}
