// Tests migrated from desktop host state; production types retain their original owners.
mod behavior_0 {
use crate::state::*;
use desktop_settings_skills::types::SkillsDirectoryScope;
#[test]
fn tool_selector_defaults_enable_every_bucket_without_filters() {
    let selector = SessionToolSelectorState::default();

    for bucket in SessionToolBucket::ALL {
        assert!(selector.is_bucket_enabled(bucket));
    }
    assert!(!selector.has_tool_filter());
    assert!(!selector.has_manual_context_selection());
    assert_eq!(selector.active_tab(), SessionToolSelectorTab::Agent);
    assert_eq!(selector.query(), "");
    assert_eq!(selector.skill_directory_scope(), SkillsDirectoryScope::All);

}
}
