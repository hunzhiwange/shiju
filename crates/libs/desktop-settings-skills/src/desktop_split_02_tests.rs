// Tests migrated from desktop host state; production types retain their original owners.
mod behavior_0 {
use crate::types::*;
use crate::update::state::State as SkillsSettingsState;
#[test]
fn skills_enums_and_default_state_are_stable() {
    assert_eq!(SkillsSettingsState::default().active_tab, SkillsSettingsTab::Skills);
    assert_ne!(SkillsSettingsTab::Skills, SkillsSettingsTab::SystemConfig);
    assert_ne!(SkillsDirectoryScope::Project, SkillsDirectoryScope::Global);

    let item = SkillsCatalogItem {
        id: "skill".to_string(),
        title: "Skill".to_string(),
        description: "Description".to_string(),
        kind: SkillsCatalogKind::Recommended,
        resource_count: 2,
        installed: true,
        enabled: true,
        source: "system".to_string(),
        source_path: Some("/tmp/skill".to_string()),
    };
    assert_eq!(item.id, "skill");
    assert_eq!(item.kind, SkillsCatalogKind::Recommended);
    assert_eq!(item.resource_count, 2);
    assert!(item.installed);
    assert!(item.enabled);

    let detail = SkillsSelectedDetail {
        id: "skill".to_string(),
        title: "Skill".to_string(),
        description: "Description".to_string(),
        kind: SkillsCatalogKind::System,
        installed: false,
        enabled: false,
        source: "bundle".to_string(),
        source_path: None,
        document_name: "SKILL.md".to_string(),
        document_content: "body".to_string(),
        can_install: true,
        can_toggle: false,
        can_delete: false,
    };
    assert_eq!(detail.document_name, "SKILL.md");
    assert!(detail.can_install);
    assert!(!detail.can_toggle);
    assert!(!detail.can_delete);

    let state = SkillsSettingsState::default();
    assert!(state.auto_load_project_skills);
    assert_eq!(
        state.directory_provider,
        config_skills::SkillsDirectoryProvider::Vibewindow
    );
    assert_eq!(
        state.prompt_injection_mode,
        config_skills::SkillsPromptInjectionMode::Compact
    );
    assert_eq!(state.active_tab, SkillsSettingsTab::Skills);
    assert_eq!(state.directory_scope, SkillsDirectoryScope::Project);
    assert!(!state.loading);
    assert!(state.catalog.is_empty());
    assert!(state.selected_skill_id.is_none());
    assert!(state.selected_skill_detail.is_none());
    assert!(!state.detail_loading);
    assert!(state.detail_error.is_none());
    assert!(state.status_message.is_none());
    assert!(!state.status_is_error);
    assert!(!state.show_help_modal);
    assert!(state.save_error.is_none());
}
}

mod behavior_1 {
use crate::types::*;
use crate::update::state::State as SkillsSettingsState;
#[test]
fn skills_catalog_models_preserve_metadata_and_support_every_kind() {
    let tabs = [
        SkillsSettingsTab::Skills,
        SkillsSettingsTab::DiscoveryOrder,
        SkillsSettingsTab::SystemConfig,
    ];
    assert_eq!(tabs.len(), 3);

    let scopes = [
        SkillsDirectoryScope::Project,
        SkillsDirectoryScope::Ancestor,
        SkillsDirectoryScope::Global,
        SkillsDirectoryScope::Bundled,
        SkillsDirectoryScope::All,
    ];
    assert_eq!(scopes.len(), 5);

    let kinds =
        [SkillsCatalogKind::Recommended, SkillsCatalogKind::System, SkillsCatalogKind::Personal];
    assert_eq!(kinds.len(), 3);

    let plugin = PluginCatalogItem {
        id: "plugin-id".to_string(),
        display_name: "Plugin".to_string(),
        version: "1.2.3".to_string(),
        description: "Plugin description".to_string(),
        enabled: true,
        component_count: 4,
        source_path: Some("plugins/plugin-id".to_string()),
        dependency_checks: Vec::new(),
        origin: Default::default(),
        hook_review: String::new(),
    };
    assert_eq!(plugin.id, "plugin-id");
    assert_eq!(plugin.display_name, "Plugin");
    assert_eq!(plugin.version, "1.2.3");
    assert_eq!(plugin.description, "Plugin description");
    assert!(plugin.enabled);
    assert_eq!(plugin.component_count, 4);
    assert_eq!(plugin.source_path.as_deref(), Some("plugins/plugin-id"));

    let state = SkillsSettingsState::default();
    assert!(state.query.is_empty());
    assert!(state.plugin_catalog.is_empty());
    assert!(!state.plugin_loading);
    assert!(state.selected_plugin_id.is_none());
    assert!(state.plugin_status_message.is_none());
    assert!(!state.plugin_status_is_error);
}
}
