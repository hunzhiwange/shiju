use super::*;
use vw_gateway_client::vw_api_types::agent_apps::AppId;
fn target(id: &str) -> SkillBindingTarget { SkillBindingTarget { installation_id: id.into(), app_id: AppId::Codex } }
fn item(id: &str, status: SkillBindingStatus) -> SkillBindingItem {
    SkillBindingItem { target: target(id), directory: "skill".into(), status }
}
#[test]
fn cp12_bindings_view_success() {
    let targets = vec![target("one"), target("two")];
    assert!(complete_success(&[item("one", SkillBindingStatus::Enabled), item("two", SkillBindingStatus::Enabled)], &targets, "skill", true));
    assert!(complete_success(&[item("one", SkillBindingStatus::Disabled), item("two", SkillBindingStatus::Disabled)], &targets, "skill", false));
}
#[test]
fn cp12_bindings_view_rejects_without_side_effects() {
    let targets = vec![target("one"), target("two")];
    let partial = vec![item("one", SkillBindingStatus::Enabled), item("two", SkillBindingStatus::Failed)];
    assert!(!complete_success(&partial, &targets, "skill", true));
    assert_eq!(partial[0].status, SkillBindingStatus::Enabled);
    assert_eq!(partial[1].status, SkillBindingStatus::Failed);
    assert!(!complete_success(&partial[..1], &targets, "skill", true));
    assert!(!complete_success(&[item("one", SkillBindingStatus::Enabled), item("one", SkillBindingStatus::Enabled)], &targets, "skill", true));
    assert!(!complete_success(&[], &[], "skill", true));
}
