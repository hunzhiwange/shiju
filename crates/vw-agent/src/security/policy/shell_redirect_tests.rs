//! 验证 Shell 重定向解析、移除及相邻令牌边界。

use super::*;

#[test]
fn strips_supported_dev_null_redirects() {
    assert_eq!(strip_supported_redirects("cargo test >/dev/null 2>&1"), "cargo test");
}

#[test]
fn parses_attached_short_option_values_and_redirect_targets() {
    assert_eq!(attached_short_option_value("-ofile"), Some("file"));
    assert_eq!(redirection_target(">out.log"), Some("out.log"));
}

#[test]
fn strip_redirects_handles_stream_merges_spacing_and_quotes() {
    assert_eq!(strip_supported_redirects("cmd 2> &1"), "cmd");
    assert_eq!(strip_supported_redirects("cmd 10>&2 arg"), "cmd  arg");
    assert_eq!(strip_supported_redirects(r#"echo "2>&1" 2>&1"#), r#"echo "2>&1""#);
    assert_eq!(strip_supported_redirects(r#"echo \2>&1"#), r#"echo \2"#);
}

#[test]
fn strip_redirects_handles_dev_null_forms_and_boundaries() {
    assert_eq!(strip_supported_redirects("cmd >/dev/null"), "cmd");
    assert_eq!(strip_supported_redirects("cmd 2>> /dev/null next"), "cmd  next");
    assert_eq!(strip_supported_redirects("cmd &>/dev/null"), "cmd");
    assert_eq!(strip_supported_redirects("cmd </dev/null"), "cmd");
    assert_eq!(strip_supported_redirects("cmd >/dev/nullish"), "cmd >/dev/nullish");
    assert_eq!(strip_supported_redirects("cmd >out.log"), "cmd >out.log");
}

#[test]
fn strip_redirects_preserves_unsupported_or_escaped_syntax() {
    assert_eq!(strip_supported_redirects("cmd |& grep x"), "cmd | grep x");
    assert_eq!(strip_supported_redirects(r#"cmd \>/dev/null"#), r#"cmd \>/dev/null"#);
    assert_eq!(strip_supported_redirects("cmd 2>&"), "cmd 2>&");
}

#[test]
fn attached_short_option_value_rejects_long_or_empty_options() {
    assert_eq!(attached_short_option_value("-o=file"), Some("file"));
    assert_eq!(attached_short_option_value("-o file"), Some("file"));
    assert_eq!(attached_short_option_value("-o"), None);
    assert_eq!(attached_short_option_value("--output=file"), None);
    assert_eq!(attached_short_option_value("output=file"), None);
}

#[test]
fn redirection_target_extracts_paths_and_ignores_fd_only_redirects() {
    assert_eq!(redirection_target("2>out.log"), Some("out.log"));
    assert_eq!(redirection_target(">> out.log"), Some("out.log"));
    assert_eq!(redirection_target("< input.txt"), Some("input.txt"));
    assert_eq!(redirection_target("2>&1"), None);
    assert_eq!(redirection_target("plain"), None);
}

#[test]
fn token_boundary_recognizes_shell_separators_and_rejects_word_characters() {
    for boundary in [' ', '\t', '\n', ';', '|', '&', ')', '('] {
        assert!(is_token_boundary_char(boundary));
    }
    for non_boundary in ['a', '/', '_', '>', '<'] {
        assert!(!is_token_boundary_char(non_boundary));
    }
}

#[test]
fn literal_matching_handles_offsets_unicode_and_short_remaining_input() {
    let chars: Vec<char> = "前/dev/null后".chars().collect();

    assert!(starts_with_literal(&chars, 1, "/dev/null"));
    assert!(!starts_with_literal(&chars, 0, "/dev/null"));
    assert!(!starts_with_literal(&chars, chars.len() - 1, "/dev/null"));
    assert!(!starts_with_literal(&chars, chars.len(), "/dev/null"));
}

#[test]
fn stream_merge_parser_accepts_fd_spacing_and_rejects_incomplete_syntax() {
    fn consumed(text: &str) -> Option<usize> {
        let chars: Vec<char> = text.chars().collect();
        consume_stream_merge_redirect(&chars, 0)
    }

    let spaced = "10> \t& 20";
    assert_eq!(consumed("2>&1"), Some("2>&1".chars().count()));
    assert_eq!(consumed(spaced), Some(spaced.chars().count()));
    assert_eq!(consumed("2&1"), None);
    assert_eq!(consumed("2>"), None);
    assert_eq!(consumed("2> 1"), None);
    assert_eq!(consumed("2>&"), None);
    assert_eq!(consumed("2>&x"), None);
}

#[test]
fn dev_null_parser_accepts_boundaries_and_rejects_malformed_or_similar_targets() {
    fn consumed(text: &str) -> Option<usize> {
        let chars: Vec<char> = text.chars().collect();
        consume_dev_null_redirect(&chars, 0)
    }

    assert_eq!(consumed(">/dev/null;next"), Some(">/dev/null".chars().count()));
    assert_eq!(consumed("&</dev/null"), None);
    assert_eq!(consumed("2x/dev/null"), None);
    assert_eq!(consumed(">/tmp/null"), None);
    assert_eq!(consumed(">/dev/null_suffix"), None);
}

#[test]
fn strip_redirects_preserves_single_quotes_double_quote_escapes_and_trailing_escape() {
    assert_eq!(strip_supported_redirects("cmd '2>&1 >/dev/null' 2>&1"), "cmd '2>&1 >/dev/null'");
    assert_eq!(strip_supported_redirects(r#"echo "a\>b 2>&1" >/dev/null"#), r#"echo "a\>b 2>&1""#);
    assert_eq!(strip_supported_redirects("cmd \\"), "cmd \\");
}

#[test]
fn attached_short_option_value_trims_values_and_rejects_empty_attachments() {
    assert_eq!(attached_short_option_value("-f\tfile.txt"), Some("file.txt"));
    assert_eq!(attached_short_option_value("-f= file.txt "), Some("file.txt"));
    assert_eq!(attached_short_option_value("-f="), None);
    assert_eq!(attached_short_option_value("-f   "), None);
}

#[test]
fn redirection_target_handles_command_prefixes_and_empty_operator_forms() {
    assert_eq!(redirection_target("cat< ./input.txt "), Some("./input.txt"));
    assert_eq!(redirection_target("echo>../output.log"), Some("../output.log"));
    assert_eq!(redirection_target(">"), None);
    assert_eq!(redirection_target("2>>   "), None);
    assert_eq!(redirection_target("2>&123"), None);
}
