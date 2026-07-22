//! 退出语义测试，覆盖常见命令的非零退出码是否应被视为错误。

use crate::tools::shell::ast::parse_command;

use super::{ExitInterpretation, ExitSemantics};

#[test]
fn command_names_select_expected_semantics() {
    for command in ["grep", "rg", "ag", "ack"] {
        assert_eq!(ExitSemantics::for_command(command), ExitSemantics::Grep);
    }

    assert_eq!(ExitSemantics::for_command("find"), ExitSemantics::Find);
    assert_eq!(ExitSemantics::for_command("diff"), ExitSemantics::Diff);
    for command in ["test", "[", "[["] {
        assert_eq!(ExitSemantics::for_command(command), ExitSemantics::Test);
    }
    assert_eq!(ExitSemantics::for_command("echo"), ExitSemantics::Default);
}

#[test]
fn parsed_command_without_name_uses_default_semantics() {
    let parsed = parse_command("");

    assert_eq!(ExitSemantics::for_parsed_command(&parsed), ExitSemantics::Default);
}

#[test]
fn grep_exit_one_is_not_error() {
    let parsed = parse_command("grep needle file.txt");
    let semantics = ExitSemantics::for_parsed_command(&parsed);

    assert_eq!(semantics.interpret(Some(1)), ExitInterpretation::NoMatches);
    assert!(!semantics.interpret(Some(1)).is_error_for_llm());
}

#[test]
fn diff_exit_one_reports_differences() {
    let semantics = ExitSemantics::for_command("diff");
    assert_eq!(semantics.interpret(Some(1)), ExitInterpretation::DifferencesFound);
}

#[test]
fn test_exit_one_is_false_not_error() {
    let semantics = ExitSemantics::for_command("test");
    assert_eq!(semantics.interpret(Some(1)), ExitInterpretation::ConditionFalse);
}

#[test]
fn missing_exit_code_is_error() {
    let semantics = ExitSemantics::for_command("echo");
    assert!(semantics.interpret(None).is_error_for_llm());
}

#[test]
fn default_exit_codes_distinguish_success_and_error() {
    let semantics = ExitSemantics::Default;

    assert_eq!(semantics.interpret(Some(0)), ExitInterpretation::Success);
    assert_eq!(
        semantics.interpret(Some(127)),
        ExitInterpretation::Error { message: "Command exited with code 127".into() }
    );
}

#[test]
fn grep_exit_codes_distinguish_success_known_error_and_unexpected_error() {
    let semantics = ExitSemantics::Grep;

    assert_eq!(semantics.interpret(Some(0)), ExitInterpretation::Success);
    assert_eq!(
        semantics.interpret(Some(2)),
        ExitInterpretation::Error { message: "grep encountered an error".into() }
    );
    assert_eq!(
        semantics.interpret(Some(3)),
        ExitInterpretation::Error { message: "Unexpected grep exit code 3".into() }
    );
}

#[test]
fn find_exit_codes_distinguish_success_partial_success_and_error() {
    let semantics = ExitSemantics::Find;

    assert_eq!(semantics.interpret(Some(0)), ExitInterpretation::Success);
    assert_eq!(semantics.interpret(Some(1)), ExitInterpretation::PartialSuccess);
    assert_eq!(
        semantics.interpret(Some(2)),
        ExitInterpretation::Error { message: "find exited with code 2".into() }
    );
}

#[test]
fn diff_exit_codes_distinguish_success_and_error() {
    let semantics = ExitSemantics::Diff;

    assert_eq!(semantics.interpret(Some(0)), ExitInterpretation::Success);
    assert_eq!(
        semantics.interpret(Some(2)),
        ExitInterpretation::Error { message: "diff exited with code 2".into() }
    );
}

#[test]
fn test_exit_codes_distinguish_true_and_error() {
    let semantics = ExitSemantics::Test;

    assert_eq!(semantics.interpret(Some(0)), ExitInterpretation::ConditionTrue);
    assert_eq!(
        semantics.interpret(Some(2)),
        ExitInterpretation::Error { message: "test exited with code 2".into() }
    );
}
