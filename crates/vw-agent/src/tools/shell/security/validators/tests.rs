//! 验证校验器构建顺序、配置开关及 shell 边界辅助函数的保守判定。

use super::*;
use crate::tools::shell::ast::ParsedCommand;
use regex::Regex;
use std::sync::LazyLock;
use vw_config_types::security::BashValidatorChecksConfig;

static WORD_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\bdanger\b").expect("valid test regex"));

fn fallback(raw: &str, tokens: &[&str]) -> ParsedCommand {
    ParsedCommand::Fallback {
        raw: raw.to_string(),
        tokens: tokens.iter().map(|token| token.to_string()).collect(),
    }
}

#[test]
fn build_validators_keeps_expected_order_and_strict_substitution() {
    let mut config = BashValidatorChecksConfig::default();
    config.quoted_newline = true;
    let validators = build_validators(true, &config);
    let names = validators.iter().map(|validator| validator.name()).collect::<Vec<_>>();

    assert_eq!(
        names,
        vec![
            "empty_command",
            "heredoc",
            "git_commit",
            "jq",
            "metacharacters",
            "dangerous_vars",
            "substitution",
            "redirection",
            "newline_injection",
            "ifs_injection",
            "proc_environ",
            "malformed",
            "obfuscated_flags",
            "backslash",
            "brace_expansion",
            "unicode_whitespace",
            "hash_comment",
            "comment_quote_desync",
            "quoted_newline",
            "zsh_dangerous",
            "control_chars",
        ]
    );
}

#[test]
fn build_validators_honors_group_and_every_item_switch() {
    let mut disabled_group = BashValidatorChecksConfig::default();
    disabled_group.enabled = false;
    assert!(build_validators(true, &disabled_group).is_empty());

    macro_rules! assert_omitted {
        ($field:ident, $name:literal) => {{
            let mut config = BashValidatorChecksConfig::default();
            config.quoted_newline = true;
            config.$field = false;
            let validators = build_validators(true, &config);
            let names = validators.iter().map(|validator| validator.name()).collect::<Vec<_>>();
            assert!(!names.contains(&$name));
            assert_eq!(names.len(), 20);
        }};
    }

    assert_omitted!(empty_command, "empty_command");
    assert_omitted!(heredoc, "heredoc");
    assert_omitted!(git_commit, "git_commit");
    assert_omitted!(jq, "jq");
    assert_omitted!(metacharacters, "metacharacters");
    assert_omitted!(dangerous_vars, "dangerous_vars");
    assert_omitted!(substitution, "substitution");
    assert_omitted!(redirection, "redirection");
    assert_omitted!(newline_injection, "newline_injection");
    assert_omitted!(ifs_injection, "ifs_injection");
    assert_omitted!(proc_environ, "proc_environ");
    assert_omitted!(malformed, "malformed");
    assert_omitted!(obfuscated_flags, "obfuscated_flags");
    assert_omitted!(backslash, "backslash");
    assert_omitted!(brace_expansion, "brace_expansion");
    assert_omitted!(unicode_whitespace, "unicode_whitespace");
    assert_omitted!(hash_comment, "hash_comment");
    assert_omitted!(comment_quote_desync, "comment_quote_desync");
    assert_omitted!(quoted_newline, "quoted_newline");
    assert_omitted!(zsh_dangerous, "zsh_dangerous");
    assert_omitted!(control_chars, "control_chars");
}

#[test]
fn finding_helpers_preserve_fields() {
    let blocked = block(SecurityCategory::Injection, "blocked", Some("fix"));
    let warned = warn(SecurityCategory::Obfuscation, "warned", None);

    assert_eq!(blocked.severity, Severity::Block);
    assert_eq!(blocked.category, SecurityCategory::Injection);
    assert_eq!(blocked.suggestion.as_deref(), Some("fix"));
    assert_eq!(warned.severity, Severity::Warn);
    assert_eq!(warned.suggestion, None);
}

#[test]
fn shell_boundary_helpers_detect_only_unquoted_risks() {
    assert!(looks_unbalanced_shell("echo 'open"));
    assert!(looks_unbalanced_shell("echo $(date"));
    assert!(!looks_unbalanced_shell("echo '\\'' ok"));
    assert!(has_unquoted_hash("echo test#hidden"));
    assert!(!has_unquoted_hash("echo 'test#literal'"));
    assert!(has_quoted_newline("echo 'a\nb'"));
    assert!(has_unquoted_newline("echo one\necho two"));
    assert!(!has_unquoted_newline("echo 'one\ntwo'"));
    assert!(!has_quoted_newline("printf a\\\nb"));
    assert!(has_control_characters("echo \u{7f}"));
    assert!(!has_control_characters("echo\tok\n"));
}

#[test]
fn lower_tokens_supports_fallback() {
    let cmd = fallback("ECHO Hi", &["ECHO", "Hi"]);

    assert_eq!(raw(&cmd), "ECHO Hi");
    assert!(info(&cmd).is_none());
    assert_eq!(lower_tokens(&cmd), vec!["echo", "hi"]);
}

#[test]
fn lower_tokens_supports_ast_command_info() {
    let info =
        crate::tools::shell::ast::CommandInfo::from_command("Git STATUS").expect("command info");
    let ast = crate::tools::shell::ast::BashAst::parse("Git STATUS").0;
    let cmd = ParsedCommand::Ast(ast, info);

    assert_eq!(lower_tokens(&cmd), vec!["git", "status"]);
}

#[test]
fn shell_boundary_helpers_cover_closing_paren_backtick_and_quoted_hash_edges() {
    assert!(looks_unbalanced_shell("echo ok)"));
    assert!(looks_unbalanced_shell("echo `date"));
    assert!(!looks_unbalanced_shell("echo (ok)"));
    assert!(!has_unquoted_hash(r"echo test\#literal"));
    assert!(!has_unquoted_hash("echo \"test#literal\""));
    assert!(has_quoted_newline("echo \"a\nb\""));
}

#[test]
fn fallback_and_text_helpers_return_conservative_results() {
    let cmd = fallback("echo safe > out", &["echo", "safe", ">", "out"]);

    assert!(redirect_targets(&cmd).is_empty());
    assert!(contains_regex(&WORD_RE, "a danger appears"));
    assert!(!contains_regex(&WORD_RE, "dangerous is a different word"));
    assert!(contains_unicode_whitespace("echo\u{00a0}hidden"));
    assert!(!contains_unicode_whitespace("echo hidden"));
}
