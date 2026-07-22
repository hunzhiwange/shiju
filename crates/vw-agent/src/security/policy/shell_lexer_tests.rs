use super::*;

#[test]
fn lexer_ignores_quoted_shell_metacharacters() {
    assert!(!contains_unquoted_char("echo ';'", ';'));
    assert!(contains_unquoted_char("echo hi; pwd", ';'));
    assert_eq!(strip_wrapping_quotes("'value'"), "value");
}

#[test]
fn env_assignment_prefix_is_skipped() {
    assert_eq!(skip_env_assignments("FOO=bar BAR=baz cargo test"), "cargo test");
}

#[test]
fn env_assignment_skipping_stops_at_invalid_names_or_command() {
    assert_eq!(skip_env_assignments("_FOO=bar echo hi"), "echo hi");
    assert_eq!(skip_env_assignments("1FOO=bar echo hi"), "1FOO=bar echo hi");
    assert_eq!(skip_env_assignments("FOO=bar"), "");
}

#[test]
fn env_assignment_skipping_handles_empty_input_and_empty_values() {
    assert_eq!(skip_env_assignments(""), "");
    assert_eq!(skip_env_assignments("   "), "   ");
    assert_eq!(skip_env_assignments("FOO= BAR=baz\nprintf ok"), "printf ok");
    assert_eq!(skip_env_assignments("=value echo hi"), "=value echo hi");
    assert_eq!(skip_env_assignments("-FOO=bar echo hi"), "-FOO=bar echo hi");
    assert_eq!(skip_env_assignments("éFOO=bar echo hi"), "éFOO=bar echo hi");
}

#[test]
fn split_segments_respects_quotes_escapes_and_control_operators() {
    assert_eq!(
        split_unquoted_segments(r#"echo "a;b" && printf 'x|y' || pwd; whoami"#),
        vec![r#"echo "a;b""#, "printf 'x|y'", "pwd", "whoami"]
    );
    assert_eq!(split_unquoted_segments(r#"echo a\;b | grep b"#), vec![r#"echo a\;b"#, "grep b"]);
    assert_eq!(split_unquoted_segments("cmd &"), vec!["cmd &"]);
    assert!(split_unquoted_segments(" ; \n ").is_empty());
}

#[test]
fn split_segments_filters_empty_segments_between_mixed_operators() {
    assert_eq!(split_unquoted_segments("first\n\n; | || && second"), vec!["first", "second"]);
}

#[test]
fn split_segments_preserves_escaped_quotes_and_unclosed_input() {
    assert_eq!(
        split_unquoted_segments(r#"printf "a\";b" && echo done"#),
        vec![r#"printf "a\";b""#, "echo done"]
    );
    assert_eq!(split_unquoted_segments("echo 'a;b"), vec!["echo 'a;b"]);
    assert_eq!(split_unquoted_segments(r"echo tail\"), vec![r"echo tail\"]);
}

#[test]
fn ampersand_detection_ignores_quotes_escapes_and_double_ampersand() {
    assert!(contains_unquoted_single_ampersand("sleep 1 &"));
    assert!(!contains_unquoted_single_ampersand("echo a && echo b"));
    assert!(!contains_unquoted_single_ampersand("echo '&'"));
    assert!(!contains_unquoted_single_ampersand(r#"echo \&"#));
    assert!(!contains_unquoted_single_ampersand(r#"echo "&""#));
}

#[test]
fn ampersand_detection_handles_runs_and_resumes_after_quotes() {
    assert!(!contains_unquoted_single_ampersand("echo a && echo b &&"));
    assert!(contains_unquoted_single_ampersand("echo a &&& echo b"));
    assert!(contains_unquoted_single_ampersand("echo '&' &"));
    assert!(contains_unquoted_single_ampersand(r#"echo "a\&b" &"#));
    assert!(!contains_unquoted_single_ampersand(r#"echo "a\"&b""#));
}

#[test]
fn unquoted_char_detection_respects_double_quote_escapes() {
    assert!(contains_unquoted_char("cat < input", '<'));
    assert!(!contains_unquoted_char(r#"echo \"not quote\" > out"#, '"'));
    assert!(!contains_unquoted_char(r#"echo \>"#, '>'));
    assert!(!contains_unquoted_char(r#"echo ">""#, '>'));
}

#[test]
fn unquoted_char_detection_resumes_after_quoted_regions() {
    assert!(contains_unquoted_char("echo '>' > output", '>'));
    assert!(contains_unquoted_char(r#"echo "a\">b" > output"#, '>'));
    assert!(!contains_unquoted_char(r"echo '\>'", '>'));
    assert!(!contains_unquoted_char(r"echo trailing\", ';'));
}

#[test]
fn variable_expansion_detection_respects_quote_rules() {
    assert!(contains_unquoted_shell_variable_expansion("echo $HOME"));
    assert!(contains_unquoted_shell_variable_expansion(r#"echo "$HOME""#));
    assert!(contains_unquoted_shell_variable_expansion("echo ${HOME}"));
    assert!(contains_unquoted_shell_variable_expansion("echo $(whoami)"));
    assert!(contains_unquoted_shell_variable_expansion("echo $?"));
    assert!(!contains_unquoted_shell_variable_expansion("echo '$HOME'"));
    assert!(!contains_unquoted_shell_variable_expansion(r#"echo \$HOME"#));
    assert!(!contains_unquoted_shell_variable_expansion("cost is $"));
}

#[test]
fn variable_expansion_detection_recognizes_special_parameters() {
    for command in
        ["echo $_", "echo $#", "echo $!", "echo $$", "echo $*", "echo $@", "echo $-", "echo $0"]
    {
        assert!(
            contains_unquoted_shell_variable_expansion(command),
            "expected expansion in {command}"
        );
    }
}

#[test]
fn variable_expansion_detection_rejects_invalid_suffixes() {
    for command in ["", "echo $", "echo $%", "echo $é"] {
        assert!(
            !contains_unquoted_shell_variable_expansion(command),
            "did not expect expansion in {command}"
        );
    }
}

#[test]
fn variable_expansion_detection_handles_quote_and_escape_transitions() {
    assert!(!contains_unquoted_shell_variable_expansion(r#"echo "\$HOME""#));
    assert!(contains_unquoted_shell_variable_expansion(r#"echo "\\$HOME""#));
    assert!(contains_unquoted_shell_variable_expansion("echo '$HOME' $USER"));
    assert!(!contains_unquoted_shell_variable_expansion("echo '$HOME"));
}

#[test]
fn strip_wrapping_quotes_trims_matching_quote_characters_from_edges() {
    assert_eq!(strip_wrapping_quotes(r#""value""#), "value");
    assert_eq!(strip_wrapping_quotes("'value'"), "value");
    assert_eq!(strip_wrapping_quotes(r#""value'"#), "value");
    assert_eq!(strip_wrapping_quotes("plain"), "plain");
}

#[test]
fn strip_wrapping_quotes_handles_empty_and_repeated_edges() {
    assert_eq!(strip_wrapping_quotes(""), "");
    assert_eq!(strip_wrapping_quotes("''''"), "");
    assert_eq!(strip_wrapping_quotes("\"'value'\""), "value");
    assert_eq!(strip_wrapping_quotes("'embedded\"quote'"), "embedded\"quote");
}
