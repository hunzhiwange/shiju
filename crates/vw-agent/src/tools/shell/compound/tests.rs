//! 复合命令权限测试，覆盖串联、管道和路径限制对审批结果的影响。

use std::path::PathBuf;

use crate::security::AutonomyLevel;
use crate::tools::shell::ast::parse_command;
use crate::tools::shell::compound::{CompoundAnalysisResult, CompoundCommandAnalyzer};
use crate::tools::shell::permissions::{
    Permission, PermissionContext, PermissionResult, PermissionRule, RuleAction, RuleEngine,
    RulePattern,
};

fn context() -> PermissionContext {
    PermissionContext {
        autonomy: AutonomyLevel::Supervised,
        in_sandbox: false,
        mode: Default::default(),
        approved: false,
        workspace_dir: PathBuf::from("/workspace"),
        allowed_roots: Vec::new(),
    }
}

#[test]
fn split_top_level_segments_respects_quotes_escapes_and_nested_delimiters() {
    for (command, expected) in [
        ("printf 'a|b'; echo done", vec!["printf 'a|b'", "echo done"]),
        ("printf \"a;b\" && echo done", vec!["printf \"a;b\"", "echo done"]),
        ("`echo a|b` | cat", vec!["`echo a|b`", "cat"]),
        ("echo a\\;b; pwd", vec!["echo a\\;b", "pwd"]),
        ("printf $(echo a|b) | cat", vec!["printf $(echo a|b)", "cat"]),
        ("echo {a|b} | cat", vec!["echo {a|b}", "cat"]),
        ("echo [a|b] | cat", vec!["echo [a|b]", "cat"]),
        ("echo a && echo b || echo c", vec!["echo a", "echo b", "echo c"]),
    ] {
        assert_eq!(super::split_top_level_segments(command), expected);
    }

    assert_eq!(super::split_top_level_segments("   "), vec![""]);
}

#[test]
fn structure_scanners_ignore_quoted_escaped_and_nested_tokens() {
    assert!(super::contains_top_level_background("sleep 1 &"));
    assert!(!super::contains_top_level_background("echo a && echo b"));
    assert!(!super::contains_top_level_background("echo '&'"));
    assert!(!super::contains_top_level_background("echo \\&"));
    assert!(!super::contains_top_level_background("(sleep 1 &)"));
    assert!(!super::contains_top_level_background("echo error >&2"));

    assert!(super::contains_top_level_token("cat <<< value", "<<<"));
    assert!(super::contains_top_level_token("diff <(sort a) b", "<("));
    assert!(super::contains_top_level_token("cat >(out)", ">("));
    assert!(!super::contains_top_level_token("echo '<<<'", "<<<"));
    assert!(!super::contains_top_level_token("echo \"<(not process)\"", "<("));
    assert!(!super::contains_top_level_token("echo \\<(", "<("));
    assert!(!super::contains_top_level_token("`cat <(nested)`", "<("));

    assert!(!super::is_wrapped_by("(echo hi) && pwd", '(', ')'));
    assert!(!super::is_wrapped_command_group("{ echo hi; } && pwd"));
}

#[test]
fn resolve_path_handles_empty_home_absolute_and_relative_paths() {
    let workspace = PathBuf::from("/workspace");

    assert_eq!(super::resolve_path("", &workspace), None);
    assert_eq!(super::resolve_path("src", &workspace), Some(PathBuf::from("/workspace/src")));
    assert_eq!(super::resolve_path("/tmp/file", &workspace), Some(PathBuf::from("/tmp/file")));

    if let Some(base_dirs) = directories::BaseDirs::new() {
        assert_eq!(super::resolve_path("~", &workspace), Some(base_dirs.home_dir().to_path_buf()));
        assert_eq!(
            super::resolve_path("~/project", &workspace),
            Some(base_dirs.home_dir().join("project"))
        );
    }
}

#[test]
fn splits_and_allows_simple_pipeline_segments() {
    let engine = RuleEngine::new(false).with_legacy_allowlist(vec!["echo".into(), "grep".into()]);
    let parsed = parse_command("echo hello | grep h");

    let result = CompoundCommandAnalyzer::analyze(&parsed, &engine, &context());

    match result {
        CompoundAnalysisResult::Allowed { segments, .. } => {
            assert_eq!(segments.len(), 2);
            assert_eq!(segments[0].segment.command_name(), Some("echo"));
            assert_eq!(segments[1].segment.command_name(), Some("grep"));
        }
        other => panic!("expected allowed result, got {other:?}"),
    }
}

#[test]
fn aggregate_results_blocks_denied_segment_and_requests_approval_for_ask_segment() {
    let mut deny_engine = RuleEngine::new(false).with_legacy_allowlist(vec!["echo".into()]);
    deny_engine.push_rule(PermissionRule {
        action: RuleAction::Deny,
        pattern: RulePattern::Prefix { command: "rm".into() },
        condition: None,
        reason: "rm is blocked".into(),
    });

    let denied = CompoundCommandAnalyzer::analyze(
        &parse_command("echo keep && rm file"),
        &deny_engine,
        &context(),
    );

    match denied {
        CompoundAnalysisResult::Blocked { reason, .. } => {
            assert_eq!(reason, "rm is blocked");
        }
        other => panic!("expected denied segment to block compound command, got {other:?}"),
    }

    let mut ask_engine = RuleEngine::new(false).with_legacy_allowlist(vec!["echo".into()]);
    ask_engine.push_rule(PermissionRule {
        action: RuleAction::Ask,
        pattern: RulePattern::Prefix { command: "git push".into() },
        condition: None,
        reason: "git push requires review".into(),
    });

    let approval = CompoundCommandAnalyzer::analyze(
        &parse_command("echo ready && git push origin main"),
        &ask_engine,
        &context(),
    );

    match approval {
        CompoundAnalysisResult::NeedsApproval { reasons, warning, segments, .. } => {
            assert_eq!(segments.len(), 2);
            assert_eq!(reasons, vec!["git push requires review"]);
            assert_eq!(warning, None);
        }
        other => panic!("expected ask segment to require approval, got {other:?}"),
    }
}

#[test]
fn blocks_background_compound_commands() {
    let engine = RuleEngine::new(false).with_legacy_allowlist(vec!["echo".into()]);
    let parsed = parse_command("echo hello &");

    let result = CompoundCommandAnalyzer::analyze(&parsed, &engine, &context());

    match result {
        CompoundAnalysisResult::Blocked { reason, .. } => {
            assert!(reason.contains("background"));
        }
        other => panic!("expected blocked result, got {other:?}"),
    }
}

#[test]
fn blocks_multiple_cd_segments() {
    let engine = RuleEngine::new(false).with_legacy_allowlist(vec!["cd".into(), "pwd".into()]);
    let parsed = parse_command("cd a && cd b && pwd");

    let result = CompoundCommandAnalyzer::analyze(&parsed, &engine, &context());

    match result {
        CompoundAnalysisResult::Blocked { reason, .. } => {
            assert!(reason.contains("Multiple cd commands"));
        }
        other => panic!("expected blocked result, got {other:?}"),
    }
}

#[test]
fn blocks_cd_then_git_outside_workspace() {
    let engine =
        RuleEngine::new(false).with_legacy_allowlist(vec!["cd".into(), "git".into(), "pwd".into()]);
    let parsed = parse_command("cd /etc && git status");

    let result = CompoundCommandAnalyzer::analyze(&parsed, &engine, &context());

    match result {
        CompoundAnalysisResult::Blocked { reason, .. } => {
            assert!(reason.contains("may escape workspace"));
        }
        other => panic!("expected blocked result, got {other:?}"),
    }
}

#[test]
fn blocks_each_unsupported_unsafe_structure_with_specific_reason() {
    let engine = RuleEngine::new(false).with_legacy_allowlist(vec![
        "cat".into(),
        "coproc".into(),
        "diff".into(),
        "echo".into(),
    ]);

    for (command, expected_label) in [
        ("(echo hi)", "subshell"),
        ("{ echo hi; }", "command group"),
        ("coproc echo hi", "coprocess"),
        ("cat <<< value", "here-string"),
        ("diff <(sort a) b", "process substitution"),
    ] {
        let result = CompoundCommandAnalyzer::analyze(&parse_command(command), &engine, &context());

        assert!(
            matches!(result, CompoundAnalysisResult::Blocked { ref reason, .. } if reason.contains(expected_label)),
            "command {command:?} should be blocked as {expected_label}"
        );
    }
}

#[test]
fn analysis_results_map_to_permission_decisions_and_join_reasons() {
    let allowed = CompoundAnalysisResult::Allowed { segments: Vec::new(), findings: Vec::new() }
        .into_permission_result();
    assert_eq!(allowed, PermissionResult::allow());

    let approval = CompoundAnalysisResult::NeedsApproval {
        segments: Vec::new(),
        reasons: vec!["first reason".into(), "second reason".into()],
        warning: Some("careful".into()),
        findings: Vec::new(),
    }
    .into_permission_result();
    assert_eq!(
        approval.permission,
        Some(Permission::Ask {
            reason: "first reason; second reason".into(),
            warning: Some("careful".into()),
        })
    );

    let blocked = CompoundAnalysisResult::Blocked { reason: "denied".into(), findings: Vec::new() }
        .into_permission_result();
    assert_eq!(blocked, PermissionResult::deny("denied"));
}
