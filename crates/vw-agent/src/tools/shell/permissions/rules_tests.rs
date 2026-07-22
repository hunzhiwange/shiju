//! 权限规则测试，覆盖 allow/deny/ask 规则、沙箱和路径约束组合。

use std::path::PathBuf;

use regex::Regex;

use crate::security::AutonomyLevel;
use crate::tools::shell::ast::parse_command;

use super::mode::PermissionMode;
use super::rules::{PermissionRule, RuleAction, RuleCondition, RuleEngine, RulePattern};
use super::{Permission, PermissionContext};

fn context() -> PermissionContext {
    PermissionContext {
        autonomy: AutonomyLevel::Supervised,
        in_sandbox: false,
        mode: PermissionMode::Normal,
        approved: false,
        workspace_dir: PathBuf::from("/workspace"),
        allowed_roots: Vec::new(),
    }
}

#[test]
fn deny_rule_precedes_allow_rule() {
    let mut engine = RuleEngine::new(false);
    engine.push_rule(PermissionRule {
        action: RuleAction::Allow,
        pattern: RulePattern::Prefix { command: "git ".into() },
        condition: None,
        reason: "allow git".into(),
    });
    engine.push_rule(PermissionRule {
        action: RuleAction::Deny,
        pattern: RulePattern::Exact { command: "git push".into() },
        condition: None,
        reason: "deny push".into(),
    });

    let result = engine.check(&parse_command("git push"), &context());
    assert_eq!(result.permission, Some(Permission::Deny { reason: "deny push".into() }));
}

#[test]
fn ask_rule_precedes_allow_rule() {
    let mut engine = RuleEngine::new(false);
    engine.push_rule(PermissionRule {
        action: RuleAction::Ask,
        pattern: RulePattern::Prefix { command: "git reset".into() },
        condition: None,
        reason: "needs approval".into(),
    });
    engine.push_rule(PermissionRule {
        action: RuleAction::Allow,
        pattern: RulePattern::Prefix { command: "git ".into() },
        condition: None,
        reason: "allow git".into(),
    });

    let result = engine.check(&parse_command("git reset --hard HEAD~1"), &context());
    assert_eq!(
        result.permission,
        Some(Permission::Ask {
            reason: "needs approval".into(),
            warning: Some("This will discard all uncommitted changes".into()),
        })
    );
}

#[test]
fn default_deny_without_matching_rule() {
    let engine = RuleEngine::new(false);
    let result = engine.check(&parse_command("echo hello"), &context());
    assert_eq!(
        result.permission,
        Some(Permission::Deny { reason: "No matching allow rule".into() })
    );
}

#[test]
fn exact_match_distinguishes_arguments() {
    let rule = PermissionRule {
        action: RuleAction::Allow,
        pattern: RulePattern::Exact { command: "git status".into() },
        condition: None,
        reason: "ok".into(),
    };

    assert!(rule.matches(&parse_command("git status"), &context()));
    assert!(!rule.matches(&parse_command("git status --short"), &context()));
}

#[test]
fn prefix_match_honors_boundaries() {
    let rule = PermissionRule {
        action: RuleAction::Allow,
        pattern: RulePattern::Prefix { command: "git".into() },
        condition: None,
        reason: "ok".into(),
    };

    assert!(rule.matches(&parse_command("git status"), &context()));
    assert!(!rule.matches(&parse_command("gitlab version"), &context()));
}

#[test]
fn glob_match_supports_wildcards() {
    let rule = PermissionRule {
        action: RuleAction::Allow,
        pattern: RulePattern::Glob { pattern: "git *status*".into() },
        condition: None,
        reason: "ok".into(),
    };

    assert!(rule.matches(&parse_command("git status --short"), &context()));
    assert!(!rule.matches(&parse_command("git diff"), &context()));
}

#[test]
fn regex_match_works() {
    let rule = PermissionRule {
        action: RuleAction::Allow,
        pattern: RulePattern::Regex { pattern: Regex::new(r"^git\s+show\b").unwrap() },
        condition: None,
        reason: "ok".into(),
    };

    assert!(rule.matches(&parse_command("git show HEAD"), &context()));
    assert!(!rule.matches(&parse_command("git status"), &context()));
}

#[test]
fn sandbox_condition_only_matches_inside_sandbox() {
    let rule = PermissionRule {
        action: RuleAction::Allow,
        pattern: RulePattern::Prefix { command: "cat".into() },
        condition: Some(RuleCondition::InSandbox),
        reason: "ok".into(),
    };
    let mut sandboxed = context();
    sandboxed.in_sandbox = true;

    assert!(rule.matches(&parse_command("cat file.txt"), &sandboxed));
    assert!(!rule.matches(&parse_command("cat file.txt"), &context()));
}

#[test]
fn has_argument_condition_detects_argument() {
    let rule = PermissionRule {
        action: RuleAction::Allow,
        pattern: RulePattern::Prefix { command: "grep".into() },
        condition: Some(RuleCondition::HasArgument { arg: "-r".into() }),
        reason: "ok".into(),
    };

    assert!(rule.matches(&parse_command("grep -r foo ."), &context()));
    assert!(!rule.matches(&parse_command("grep foo file.txt"), &context()));
}

#[test]
fn not_has_argument_condition_rejects_matching_argument() {
    let rule = PermissionRule {
        action: RuleAction::Allow,
        pattern: RulePattern::Prefix { command: "git status".into() },
        condition: Some(RuleCondition::NotHasArgument { arg: "--porcelain".into() }),
        reason: "ok".into(),
    };

    assert!(rule.matches(&parse_command("git status --short"), &context()));
    assert!(!rule.matches(&parse_command("git status --porcelain"), &context()));
}

#[test]
fn workdir_condition_requires_exact_workspace_path() {
    let rule = PermissionRule {
        action: RuleAction::Allow,
        pattern: RulePattern::Exact { command: "pwd".into() },
        condition: Some(RuleCondition::WorkdirMatches { path: PathBuf::from("/workspace") }),
        reason: "ok".into(),
    };
    let mut elsewhere = context();
    elsewhere.workspace_dir = PathBuf::from("/other");

    assert!(rule.matches(&parse_command("pwd"), &context()));
    assert!(!rule.matches(&parse_command("pwd"), &elsewhere));
}

#[test]
fn approved_ask_rule_allows_without_returning_prompt() {
    let mut engine = RuleEngine::new(false);
    engine.push_rule(PermissionRule {
        action: RuleAction::Ask,
        pattern: RulePattern::Exact { command: "git reset --hard".into() },
        condition: None,
        reason: "needs approval".into(),
    });
    let mut approved = context();
    approved.approved = true;

    let result = engine.check(&parse_command("git reset --hard"), &approved);

    assert_eq!(result.permission, Some(Permission::Allow));
}

#[test]
fn first_matching_deny_rule_wins() {
    let mut engine = RuleEngine::new(false);
    engine.push_rule(PermissionRule {
        action: RuleAction::Deny,
        pattern: RulePattern::Prefix { command: "git ".into() },
        condition: None,
        reason: "general deny".into(),
    });
    engine.push_rule(PermissionRule {
        action: RuleAction::Deny,
        pattern: RulePattern::Exact { command: "git push".into() },
        condition: None,
        reason: "specific deny".into(),
    });

    let result = engine.check(&parse_command("git push"), &context());
    assert_eq!(result.permission, Some(Permission::Deny { reason: "general deny".into() }));
}

#[test]
fn empty_rule_lists_still_default_deny() {
    let engine = RuleEngine::new(false);
    let result = engine.check(&parse_command("ls"), &context());
    assert!(matches!(result.permission, Some(Permission::Deny { .. })));
}
