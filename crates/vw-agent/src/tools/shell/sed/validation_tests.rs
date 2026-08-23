//! 验证 sed 安全子集的解析、允许范围及危险脚本和原地写入拒绝规则。

use super::validation;
use super::*;
use crate::tools::shell::ast::{ParsedCommand, parse_command};

#[test]
fn allows_print_and_substitute_subset() {
    assert!(matches!(
        validate_sed_command(&parse_command("sed -n '1,3p' file.txt")),
        SedValidationResult::Allowed {
            kind: SedCommandKind::Print { .. },
            in_place: false,
            extended_regex: false,
            ..
        }
    ));
    assert!(matches!(
        validate_sed_command(&parse_command("sed -E -i 's/a/b/g' file.txt")),
        SedValidationResult::Allowed {
            kind: SedCommandKind::Substitute { .. },
            in_place: true,
            extended_regex: true,
            ..
        }
    ));
}

#[test]
fn blocks_unsafe_sed_syntax() {
    for command in ["awk '{print}' file", "sed -f script.sed", "sed 's/a/b/w out' file"] {
        assert!(matches!(
            validate_sed_command(&parse_command(command)),
            SedValidationResult::Blocked { .. }
        ));
    }
}

#[test]
fn parses_substitute_escaped_delimiters() {
    let (pattern, replacement, flags) =
        validation::parse_substitute_parts(r"s/a\/b/c\/d/g").expect("valid substitute");

    assert_eq!(pattern, r"a\/b");
    assert_eq!(replacement, r"c\/d");
    assert_eq!(flags, "g");
}

#[test]
fn blocks_non_ascii_and_blocked_script_syntax() {
    assert_eq!(
        validate_sed_command(&parse_command("sed 's/你/好/' file.txt")),
        SedValidationResult::Blocked { reason: "sed script must be ASCII".into() }
    );
    assert_eq!(
        validate_sed_command(&parse_command("sed '1,3p # comment' file.txt")),
        SedValidationResult::Blocked { reason: "sed script uses blocked syntax".into() }
    );
}

#[test]
fn rejects_commands_outside_allowed_subset() {
    assert_eq!(
        validate_sed_command(&parse_command("sed 'd' file.txt")),
        SedValidationResult::Blocked {
            reason: "only sed print and substitute commands are allowed".into()
        }
    );
}

#[test]
fn sed_invocation_parses_fallback_tokens_and_empty_backup_suffix() {
    let cmd = ParsedCommand::Fallback {
        raw: "sed -i '' -r s/a/b/ file.txt".into(),
        tokens: vec![
            "sed".into(),
            "-i".into(),
            "".into(),
            "-r".into(),
            "s/a/b/".into(),
            "file.txt".into(),
        ],
    };

    let invocation = validation::SedInvocation::from_command(&cmd).expect("sed invocation");
    assert_eq!(invocation.script, "s/a/b/");
    assert_eq!(invocation.files, vec!["file.txt"]);
    assert!(invocation.in_place);
    assert!(invocation.extended_regex);
}

#[test]
fn sed_invocation_rejects_non_sed_or_missing_script() {
    assert!(
        validation::SedInvocation::from_command(&parse_command("awk '{print}' file")).is_none()
    );
    assert!(validation::SedInvocation::from_command(&parse_command("sed -n")).is_none());
}

#[test]
fn sed_invocation_parses_full_path_expression_and_backup_suffix() {
    let cmd = ParsedCommand::Fallback {
        raw: "/usr/bin/sed -x -i.bak -e s/a/b/ file.txt".into(),
        tokens: vec![
            "/usr/bin/sed".into(),
            "-x".into(),
            "-i.bak".into(),
            "-e".into(),
            "s/a/b/".into(),
            "file.txt".into(),
        ],
    };

    let invocation = validation::SedInvocation::from_command(&cmd).expect("sed invocation");
    assert_eq!(invocation.script, "s/a/b/");
    assert_eq!(invocation.files, vec!["file.txt"]);
    assert!(invocation.in_place);
}

#[test]
fn blocks_substitute_write_and_execute_flags() {
    for flag in ["W", "e", "E"] {
        assert_eq!(
            validate_sed_command(&parse_command(&format!("sed 's/a/b/{flag}' file.txt"))),
            SedValidationResult::Blocked {
                reason: "sed substitute uses blocked write/exec flags".into()
            }
        );
    }
}

#[test]
fn malformed_substitute_parts_are_rejected() {
    assert!(validation::parse_substitute_parts("s/a/b").is_none());
    assert!(validation::parse_substitute_parts("print").is_none());
}
