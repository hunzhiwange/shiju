use super::*;
use upgrades::main_selections;
#[test]
fn main_upgrade_includes_native_and_only_selected_agent_directories() {
    let mut state = crate::skill_hub::ExpertHubState::default();
    state.install_guide.install_scope = SkillInstallScope::Project;
    state.install_guide.selected_targets =
        vec![SkillInstallTarget::Codex, SkillInstallTarget::ClaudeCode];
    assert_eq!(
        main_selections(&state),
        vec![
            SkillInstallSelection {
                target: SkillInstallTarget::VibeWindow,
                scope: SkillInstallScope::Global
            },
            SkillInstallSelection {
                target: SkillInstallTarget::Codex,
                scope: SkillInstallScope::Project
            },
            SkillInstallSelection {
                target: SkillInstallTarget::ClaudeCode,
                scope: SkillInstallScope::Project
            },
        ]
    );
    state.install_guide.selected_targets = vec![SkillInstallTarget::VibeWindow];
    state.install_guide.install_scope = SkillInstallScope::Global;
    assert_eq!(main_selections(&state).len(), 1);
}
