use super::project_profiles::*;
use serde_json::json;

#[test]
fn profile_t02_payload_distinguishes_uncaptured_and_empty() {
    let payload: ProjectProfilePayload = serde_json::from_value(json!({
        "mcp": {"claude": [], "codex": null},
        "skills": {"claude": []}
    })).unwrap();
    assert_eq!(payload.mcp.claude, Some(vec![]));
    assert_eq!(payload.skills.claude, Some(vec![]));
    assert_eq!(payload.mcp.codex, None);
    assert!(payload.scope_captured(ProjectProfileScope::Claude));
    assert!(!payload.scope_captured(ProjectProfileScope::Codex));
    assert_eq!(serde_json::from_value::<ProjectProfilePayload>(
        serde_json::to_value(&payload).unwrap()).unwrap(), payload);
}

#[test]
fn profile_t02_scopes_map_to_independent_apps() {
    for (scope, key) in ProjectProfileScope::ALL.into_iter()
        .zip(["claude", "claude-desktop", "codex"])
    {
        assert_eq!(serde_json::to_value(scope).unwrap(), json!(key));
        assert_eq!(ProjectProfileScope::for_app(scope.app_id()), Some(scope));
    }
    let mut payload = ProjectProfilePayload::default();
    payload.providers.claude = Some("cli".into());
    let mut other = ProjectProfilePayload::default();
    other.providers.claude_desktop = Some("desktop".into());
    payload.merge_scope_from(&other, ProjectProfileScope::ClaudeDesktop);
    assert_eq!(payload.providers.claude.as_deref(), Some("cli"));
    assert_eq!(payload.providers.claude_desktop.as_deref(), Some("desktop"));
    assert_eq!(payload.providers.codex, None);
    let wire = serde_json::to_value(payload).unwrap();
    assert_eq!(wire["providers"]["claude-desktop"], "desktop");
}

#[test]
fn profile_t02_empty_prompt_is_explicit_deactivation() {
    let payload: ProjectProfilePayload = serde_json::from_value(json!({
        "prompts": {"claude-desktop": "", "codex": null}
    })).unwrap();
    assert_eq!(payload.prompts.claude_desktop.as_deref(), Some(PROJECT_PROFILE_NO_ACTIVE_PROMPT));
    assert!(payload.scope_captured(ProjectProfileScope::ClaudeDesktop));
    assert!(!payload.scope_captured(ProjectProfileScope::Codex));
    assert_eq!(serde_json::to_value(payload).unwrap()["prompts"]["claude-desktop"], "");
}

#[test]
fn profile_t02_unknown_groups_are_rejected() {
    use crate::agent_apps::catalog::AppId;
    assert_eq!(ProjectProfileScope::for_app(AppId::Gemini), None);
    for key in ["gemini", "claude_desktop", "Claude", "pi"] {
        assert!(serde_json::from_value::<ProjectProfileScope>(json!(key)).is_err());
        assert!(serde_json::from_value::<ProjectProfilePayload>(json!({
            "mcp": {key: []}
        })).is_err());
    }
}

#[test]
fn profile_t02_partial_and_selected_are_distinct_from_applied() {
    let result = ProjectProfileApplyResult {
        profile_id: ProjectProfileId("profile-1".into()),
        group: ProjectProfileScope::ClaudeDesktop,
        selected: true,
        status: ProjectProfileApplyStatus::Partial,
        items: vec![ProjectProfileItemResult {
            category: ProjectProfileCategory::Mcp,
            reference_id: Some("server-1".into()),
            status: ProjectProfileItemStatus::Unsupported,
            reason_code: Some("unsupported_application".into()),
        }],
    };
    let wire = serde_json::to_value(&result).unwrap();
    assert_eq!(wire["status"], "partial");
    assert_eq!(wire["selected"], true);
    assert_eq!(serde_json::from_value::<ProjectProfileApplyResult>(wire).unwrap(), result);
}
