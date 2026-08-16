use super::*;
use crate::app::App;
use crate::app::message::settings::AgentsMessage;
use crate::app::state::{SkillsDirectoryScope, SkillsSettingsTab};
use vw_config_types::skills::{SkillsDirectoryProvider, SkillsPromptInjectionMode};
use vw_gateway_client::{
    DesktopPluginCatalogEntryDto, DesktopSkillCatalogEntryDto, DesktopSkillDetailDto,
};

fn app() -> App {
    App::new().0
}

fn catalog_item(id: &str, kind: &str) -> DesktopSkillCatalogEntryDto {
    DesktopSkillCatalogEntryDto {
        id: id.to_string(),
        title: format!("Title {id}"),
        description: "desc".to_string(),
        kind: kind.to_string(),
        resource_count: 2,
        installed: true,
        enabled: true,
        source: "builtin".to_string(),
        source_path: Some("/tmp/skill".to_string()),
        origin: None,
        namespace: None,
        package_type: None,
        source_pack: None,
    }
}

fn detail(id: &str, kind: &str) -> DesktopSkillDetailDto {
    DesktopSkillDetailDto {
        id: id.to_string(),
        title: format!("Title {id}"),
        description: "desc".to_string(),
        kind: kind.to_string(),
        installed: true,
        enabled: false,
        source: "builtin".to_string(),
        source_path: None,
        document_name: "SKILL.md".to_string(),
        document_content: "# Skill".to_string(),
        can_install: true,
        can_toggle: true,
        can_delete: false,
    }
}

fn plugin(id: &str) -> DesktopPluginCatalogEntryDto {
    DesktopPluginCatalogEntryDto {
        id: id.to_string(),
        display_name: format!("Plugin {id}"),
        version: "1.2.3".to_string(),
        description: "plugin desc".to_string(),
        enabled: true,
        component_count: 3,
        source_path: Some(format!("/tmp/plugins/{id}")),
    }
}

async fn collect_task_messages(task: iced::Task<Message>) -> Vec<Message> {
    use iced_runtime::futures::futures::StreamExt;

    let Some(mut stream) = iced_runtime::task::into_stream(task.collect()) else {
        return Vec::new();
    };
    while let Some(action) = stream.next().await {
        if let iced_runtime::Action::Output(messages) = action {
            return messages;
        }
    }
    Vec::new()
}

#[test]
fn catalog_mappers_preserve_dto_fields_and_default_unknown_skill_kind() {
    let skill = map_catalog_items(vec![catalog_item("skill-a", "unknown")]).remove(0);
    assert_eq!(skill.id, "skill-a");
    assert_eq!(skill.title, "Title skill-a");
    assert_eq!(skill.description, "desc");
    assert_eq!(skill.kind, SkillsCatalogKind::System);
    assert_eq!(skill.resource_count, 2);
    assert!(skill.installed);
    assert!(skill.enabled);
    assert_eq!(skill.source, "builtin");
    assert_eq!(skill.source_path.as_deref(), Some("/tmp/skill"));

    let plugin = map_plugin_catalog_items(vec![plugin("plugin-a")]).remove(0);
    assert_eq!(plugin.id, "plugin-a");
    assert_eq!(plugin.display_name, "Plugin plugin-a");
    assert_eq!(plugin.version, "1.2.3");
    assert_eq!(plugin.description, "plugin desc");
    assert!(plugin.enabled);
    assert_eq!(plugin.component_count, 3);
    assert_eq!(plugin.source_path.as_deref(), Some("/tmp/plugins/plugin-a"));

    let detail = map_skill_detail(detail("skill-a", "personal"));
    assert_eq!(detail.id, "skill-a");
    assert_eq!(detail.title, "Title skill-a");
    assert_eq!(detail.description, "desc");
    assert_eq!(detail.kind, SkillsCatalogKind::Personal);
    assert!(detail.installed);
    assert!(!detail.enabled);
    assert_eq!(detail.source, "builtin");
    assert!(detail.source_path.is_none());
    assert_eq!(detail.document_name, "SKILL.md");
    assert_eq!(detail.document_content, "# Skill");
    assert!(detail.can_install);
    assert!(detail.can_toggle);
    assert!(!detail.can_delete);
}

#[tokio::test(flavor = "multi_thread")]
async fn copy_install_command_emits_the_documented_template() {
    let mut app = app();

    let messages =
        collect_task_messages(update(&mut app, SettingsMessage::SkillsCopyInstallCommand)).await;

    assert!(matches!(
        messages.as_slice(),
        [Message::CopyCode(command)] if command == INSTALL_COMMAND_TEMPLATE
    ));
}

#[tokio::test(flavor = "multi_thread")]
async fn skills_project_selection_refreshes_current_and_opens_other_project() {
    let mut app = app();
    app.project_path = Some("/repo/current".to_string());

    let messages = collect_task_messages(update(
        &mut app,
        SettingsMessage::SkillsProjectSelected("/repo/current".to_string()),
    ))
    .await;

    assert!(messages.iter().any(|message| matches!(
        message,
        Message::Settings(SettingsMessage::Agents(AgentsMessage::ProjectChanged(project_path)))
            if project_path == "/repo/current"
    )));
    assert!(
        messages
            .iter()
            .any(|message| matches!(message, Message::Settings(SettingsMessage::SkillsRefresh)))
    );

    let _ = update(&mut app, SettingsMessage::SkillsProjectSelected("/repo/other".to_string()));

    assert_eq!(app.project_path.as_deref(), Some("/repo/other"));
    assert_eq!(app.agents_settings.project_path.as_deref(), Some("/repo/other"));
    assert!(app.agents_settings.loading);
}

#[test]
fn skills_loaded_preserves_present_selection_and_only_clears_catalog_load_errors() {
    let mut app = app();
    app.skills_settings.loading = true;
    app.skills_settings.selected_skill_id = Some("present".to_string());
    app.skills_settings.selected_skill_detail =
        Some(map_skill_detail(detail("present", "personal")));
    app.skills_settings.detail_loading = true;
    app.skills_settings.detail_error = Some("old detail error".to_string());
    app.skills_settings.status_message = Some("读取技能目录失败: unavailable".to_string());
    app.skills_settings.status_is_error = true;

    let _ = update(
        &mut app,
        SettingsMessage::SkillsLoaded(Ok(vec![catalog_item("present", "personal")])),
    );

    assert!(!app.skills_settings.loading);
    assert_eq!(app.skills_settings.selected_skill_id.as_deref(), Some("present"));
    assert!(app.skills_settings.selected_skill_detail.is_some());
    assert!(app.skills_settings.detail_loading);
    assert_eq!(app.skills_settings.detail_error.as_deref(), Some("old detail error"));
    assert!(app.skills_settings.status_message.is_none());
    assert!(!app.skills_settings.status_is_error);

    app.skills_settings.status_message = Some("安装失败".to_string());
    app.skills_settings.status_is_error = true;
    let _ = update(&mut app, SettingsMessage::SkillsLoaded(Ok(Vec::new())));
    assert_eq!(app.skills_settings.status_message.as_deref(), Some("安装失败"));
    assert!(app.skills_settings.status_is_error);
}

#[test]
fn skills_loaded_error_clears_catalog_and_records_contextual_error() {
    let mut app = app();
    app.skills_settings.catalog = map_catalog_items(vec![catalog_item("old", "personal")]);
    app.skills_settings.loading = true;

    let _ = update(&mut app, SettingsMessage::SkillsLoaded(Err("offline".to_string())));

    assert!(!app.skills_settings.loading);
    assert!(app.skills_settings.catalog.is_empty());
    assert_eq!(app.skills_settings.status_message.as_deref(), Some("读取技能目录失败: offline"));
    assert!(app.skills_settings.status_is_error);
}

#[test]
fn plugins_loaded_maps_catalog_manages_selection_and_scopes_error_cleanup() {
    let mut app = app();
    app.skills_settings.plugin_loading = true;
    app.skills_settings.selected_plugin_id = Some("present".to_string());
    app.skills_settings.plugin_status_message = Some("读取插件目录失败: offline".to_string());
    app.skills_settings.plugin_status_is_error = true;

    let _ = update(&mut app, SettingsMessage::PluginsLoaded(Ok(vec![plugin("present")])));
    assert!(!app.skills_settings.plugin_loading);
    assert_eq!(app.skills_settings.plugin_catalog[0].display_name, "Plugin present");
    assert_eq!(app.skills_settings.selected_plugin_id.as_deref(), Some("present"));
    assert!(app.skills_settings.plugin_status_message.is_none());
    assert!(!app.skills_settings.plugin_status_is_error);

    app.skills_settings.plugin_status_message = Some("删除插件失败".to_string());
    app.skills_settings.plugin_status_is_error = true;
    let _ = update(&mut app, SettingsMessage::PluginsLoaded(Ok(vec![plugin("replacement")])));
    assert!(app.skills_settings.selected_plugin_id.is_none());
    assert_eq!(app.skills_settings.plugin_status_message.as_deref(), Some("删除插件失败"));

    let _ = update(&mut app, SettingsMessage::PluginsLoaded(Err("offline".to_string())));
    assert!(app.skills_settings.plugin_catalog.is_empty());
    assert_eq!(
        app.skills_settings.plugin_status_message.as_deref(),
        Some("读取插件目录失败: offline")
    );
    assert!(app.skills_settings.plugin_status_is_error);
}

#[test]
fn plugin_selection_and_requested_operations_enter_loading_state() {
    let mut app = app();

    let _ = update(&mut app, SettingsMessage::PluginsRefresh);
    assert!(app.skills_settings.plugin_loading);

    app.skills_settings.plugin_loading = false;
    let _ = update(&mut app, SettingsMessage::PluginsSelected("plugin-a".to_string()));
    assert_eq!(app.skills_settings.selected_plugin_id.as_deref(), Some("plugin-a"));

    let _ = update(
        &mut app,
        SettingsMessage::PluginsSetEnabledRequested {
            plugin_id: "plugin-a".to_string(),
            enabled: false,
        },
    );
    assert!(app.skills_settings.plugin_loading);

    app.skills_settings.plugin_loading = false;
    let _ = update(&mut app, SettingsMessage::PluginsDeleteRequested("plugin-a".to_string()));
    assert!(app.skills_settings.plugin_loading);
}

#[test]
fn plugin_toggle_completion_reports_both_actions_and_failure() {
    let mut app = app();

    let _ = update(
        &mut app,
        SettingsMessage::PluginsSetEnabledCompleted {
            plugin_id: "plugin-a".to_string(),
            enabled: true,
            result: Ok("/tmp/plugins/plugin-a".to_string()),
        },
    );
    assert_eq!(
        app.skills_settings.plugin_status_message.as_deref(),
        Some("已启用插件: /tmp/plugins/plugin-a")
    );
    assert!(!app.skills_settings.plugin_status_is_error);
    assert!(app.skills_settings.plugin_loading);

    let _ = update(
        &mut app,
        SettingsMessage::PluginsSetEnabledCompleted {
            plugin_id: "plugin-a".to_string(),
            enabled: false,
            result: Ok("/tmp/plugins/plugin-a".to_string()),
        },
    );
    assert_eq!(
        app.skills_settings.plugin_status_message.as_deref(),
        Some("已禁用插件: /tmp/plugins/plugin-a")
    );

    let _ = update(
        &mut app,
        SettingsMessage::PluginsSetEnabledCompleted {
            plugin_id: "plugin-a".to_string(),
            enabled: false,
            result: Err("toggle failed".to_string()),
        },
    );
    assert!(!app.skills_settings.plugin_loading);
    assert_eq!(app.skills_settings.plugin_status_message.as_deref(), Some("toggle failed"));
    assert!(app.skills_settings.plugin_status_is_error);
}

#[test]
fn plugin_toggle_failure_preserves_matching_selection() {
    let mut app = app();
    app.skills_settings.selected_plugin_id = Some("plugin-a".to_string());
    app.skills_settings.plugin_loading = true;

    let _ = update(
        &mut app,
        SettingsMessage::PluginsSetEnabledCompleted {
            plugin_id: "plugin-a".to_string(),
            enabled: true,
            result: Err("toggle failed".to_string()),
        },
    );

    assert!(!app.skills_settings.plugin_loading);
    assert_eq!(app.skills_settings.selected_plugin_id.as_deref(), Some("plugin-a"));
    assert_eq!(app.skills_settings.plugin_status_message.as_deref(), Some("toggle failed"));
    assert!(app.skills_settings.plugin_status_is_error);
}

#[test]
fn plugin_delete_completion_clears_only_matching_selection_and_reports_failure() {
    let mut app = app();
    app.skills_settings.selected_plugin_id = Some("plugin-a".to_string());

    let _ = update(
        &mut app,
        SettingsMessage::PluginsDeleteCompleted {
            plugin_id: "plugin-b".to_string(),
            result: Ok("/tmp/plugins/plugin-b".to_string()),
        },
    );
    assert_eq!(app.skills_settings.selected_plugin_id.as_deref(), Some("plugin-a"));

    let _ = update(
        &mut app,
        SettingsMessage::PluginsDeleteCompleted {
            plugin_id: "plugin-a".to_string(),
            result: Ok("/tmp/plugins/plugin-a".to_string()),
        },
    );
    assert!(app.skills_settings.selected_plugin_id.is_none());
    assert_eq!(
        app.skills_settings.plugin_status_message.as_deref(),
        Some("已删除插件: /tmp/plugins/plugin-a")
    );
    assert!(app.skills_settings.plugin_loading);

    let _ = update(
        &mut app,
        SettingsMessage::PluginsDeleteCompleted {
            plugin_id: "plugin-a".to_string(),
            result: Err("delete failed".to_string()),
        },
    );
    assert!(!app.skills_settings.plugin_loading);
    assert_eq!(app.skills_settings.plugin_status_message.as_deref(), Some("delete failed"));
    assert!(app.skills_settings.plugin_status_is_error);
}

#[test]
fn skill_requested_operations_set_loading_for_catalog_and_selected_detail() {
    let mut app = app();
    app.skills_settings.selected_skill_id = Some("skill-a".to_string());
    app.skills_settings.detail_error = Some("old".to_string());

    let _ = update(&mut app, SettingsMessage::SkillsRefresh);
    assert!(app.skills_settings.loading);

    app.skills_settings.loading = false;
    let _ = update(
        &mut app,
        SettingsMessage::SkillsSetEnabledRequested {
            skill_id: "skill-a".to_string(),
            enabled: true,
        },
    );
    assert!(app.skills_settings.loading);
    assert!(app.skills_settings.detail_loading);
    assert!(app.skills_settings.detail_error.is_none());

    app.skills_settings.loading = false;
    app.skills_settings.detail_loading = false;
    let _ = update(
        &mut app,
        SettingsMessage::SkillsSetEnabledRequested {
            skill_id: "skill-b".to_string(),
            enabled: false,
        },
    );
    assert!(app.skills_settings.loading);
    assert!(!app.skills_settings.detail_loading);

    app.skills_settings.loading = false;
    let _ = update(&mut app, SettingsMessage::SkillsDeleteRequested("skill-a".to_string()));
    assert!(app.skills_settings.loading);
}

#[test]
fn project_scoped_create_and_install_requests_enter_loading_state() {
    let mut app = app();
    app.project_path = Some("/tmp/project".to_string());

    let _ = update(&mut app, SettingsMessage::SkillsCreateNewRequested);
    assert!(app.skills_settings.loading);

    app.skills_settings.loading = false;
    let _ =
        update(&mut app, SettingsMessage::SkillsInstallBuiltInRequested("builtin-a".to_string()));
    assert!(app.skills_settings.loading);
}

#[test]
fn skill_completion_paths_report_actions_refresh_matching_detail_and_stop_on_errors() {
    let mut app = app();
    app.skills_settings.selected_skill_id = Some("skill-a".to_string());

    let _ = update(
        &mut app,
        SettingsMessage::SkillsSetEnabledCompleted {
            skill_id: "skill-a".to_string(),
            enabled: true,
            result: Ok("/tmp/skill-a".to_string()),
        },
    );
    assert_eq!(app.skills_settings.status_message.as_deref(), Some("已启用技能: /tmp/skill-a"));
    assert!(!app.skills_settings.status_is_error);
    assert!(app.skills_settings.loading);
    assert!(app.skills_settings.detail_loading);

    app.skills_settings.detail_loading = false;
    let _ = update(
        &mut app,
        SettingsMessage::SkillsSetEnabledCompleted {
            skill_id: "skill-b".to_string(),
            enabled: false,
            result: Ok("/tmp/skill-b".to_string()),
        },
    );
    assert_eq!(app.skills_settings.status_message.as_deref(), Some("已禁用技能: /tmp/skill-b"));
    assert!(!app.skills_settings.detail_loading);

    let _ = update(
        &mut app,
        SettingsMessage::SkillsSetEnabledCompleted {
            skill_id: "skill-a".to_string(),
            enabled: false,
            result: Err("toggle failed".to_string()),
        },
    );
    assert!(!app.skills_settings.loading);
    assert!(!app.skills_settings.detail_loading);
    assert_eq!(app.skills_settings.status_message.as_deref(), Some("toggle failed"));
    assert!(app.skills_settings.status_is_error);
}

#[test]
fn create_install_and_delete_completions_cover_selected_and_unselected_paths() {
    let mut app = app();
    app.project_path = Some("/tmp/project".to_string());

    let _ = update(
        &mut app,
        SettingsMessage::SkillsCreateNewCompleted(Ok("/tmp/project/.skills/new".to_string())),
    );
    assert_eq!(
        app.skills_settings.status_message.as_deref(),
        Some("已创建新技能: /tmp/project/.skills/new")
    );
    assert!(!app.skills_settings.status_is_error);
    assert!(app.skills_settings.loading);

    app.skills_settings.selected_skill_id = None;
    let _ = update(
        &mut app,
        SettingsMessage::SkillsInstallBuiltInCompleted(Ok("/tmp/project/skill-a".to_string())),
    );
    assert!(app.skills_settings.loading);
    assert!(!app.skills_settings.detail_loading);

    app.skills_settings.selected_skill_id = Some("skill-a".to_string());
    let _ = update(
        &mut app,
        SettingsMessage::SkillsInstallBuiltInCompleted(Ok("/tmp/project/skill-a".to_string())),
    );
    assert!(app.skills_settings.detail_loading);
    assert!(app.skills_settings.detail_error.is_none());

    let _ = update(
        &mut app,
        SettingsMessage::SkillsDeleteCompleted {
            skill_id: "skill-b".to_string(),
            result: Ok("/tmp/project/skill-b".to_string()),
        },
    );
    assert_eq!(app.skills_settings.selected_skill_id.as_deref(), Some("skill-a"));

    let _ = update(
        &mut app,
        SettingsMessage::SkillsDeleteCompleted {
            skill_id: "skill-a".to_string(),
            result: Err("delete failed".to_string()),
        },
    );
    assert!(!app.skills_settings.loading);
    assert_eq!(app.skills_settings.status_message.as_deref(), Some("delete failed"));
    assert!(app.skills_settings.status_is_error);
}

#[test]
fn skill_mapping_loading_detail_and_config_paths() {
    assert_eq!(map_skill_kind("recommended"), SkillsCatalogKind::Recommended);
    assert_eq!(map_skill_kind("personal"), SkillsCatalogKind::Personal);
    assert_eq!(map_skill_kind("anything"), SkillsCatalogKind::System);
    let items =
        map_catalog_items(vec![catalog_item("a", "recommended"), catalog_item("b", "personal")]);
    assert_eq!(items[0].kind, SkillsCatalogKind::Recommended);
    assert_eq!(items[1].kind, SkillsCatalogKind::Personal);
    let mapped = map_skill_detail(detail("a", "recommended"));
    assert_eq!(mapped.document_name, "SKILL.md");

    let mut app = app();
    app.skills_settings.selected_skill_id = Some("missing".to_string());
    let _ = update(
        &mut app,
        SettingsMessage::SkillsLoaded(Ok(vec![catalog_item("present", "recommended")])),
    );
    assert_eq!(app.skills_settings.catalog.len(), 1);
    assert!(app.skills_settings.selected_skill_id.is_none());
    let _ = update(&mut app, SettingsMessage::SkillsLoaded(Err("boom".to_string())));
    assert!(app.skills_settings.status_is_error);

    let _ = update(&mut app, SettingsMessage::SkillsDetailRequested("skill-a".to_string()));
    assert_eq!(app.skills_settings.selected_skill_id.as_deref(), Some("skill-a"));
    let _ = update(
        &mut app,
        SettingsMessage::SkillsDetailLoaded {
            skill_id: "other".to_string(),
            result: Ok(detail("other", "personal")),
        },
    );
    assert!(app.skills_settings.detail_loading);
    let _ = update(
        &mut app,
        SettingsMessage::SkillsDetailLoaded {
            skill_id: "skill-a".to_string(),
            result: Ok(detail("skill-a", "personal")),
        },
    );
    assert_eq!(
        app.skills_settings.selected_skill_detail.as_ref().unwrap().kind,
        SkillsCatalogKind::Personal
    );
    let _ = update(
        &mut app,
        SettingsMessage::SkillsDetailLoaded {
            skill_id: "skill-a".to_string(),
            result: Err("detail failed".to_string()),
        },
    );
    assert_eq!(app.skills_settings.detail_error.as_deref(), Some("detail failed"));
    let _ = update(&mut app, SettingsMessage::SkillsDetailClosed);
    assert!(app.skills_settings.selected_skill_id.is_none());

    app.project_path = None;
    let _ = update(&mut app, SettingsMessage::SkillsTabChanged(SkillsSettingsTab::SystemConfig));
    let _ = update(&mut app, SettingsMessage::SkillsQueryChanged("rust".to_string()));
    let _ = update(
        &mut app,
        SettingsMessage::SkillsDirectoryScopeChanged(SkillsDirectoryScope::Global),
    );
    assert_eq!(app.skills_settings.active_tab, SkillsSettingsTab::SystemConfig);
    assert_eq!(app.skills_settings.query, "rust");
    assert_eq!(app.skills_settings.directory_scope, SkillsDirectoryScope::Global);
    let _ = update(&mut app, SettingsMessage::SkillsCreateNewRequested);
    assert!(
        app.skills_settings.status_message.as_deref().unwrap_or("").contains("请先打开一个项目")
    );
    let _ = update(&mut app, SettingsMessage::SkillsInstallBuiltInRequested("builtin".to_string()));
    assert!(
        app.skills_settings.status_message.as_deref().unwrap_or("").contains("请先打开一个项目")
    );

    app.project_path = Some("/tmp/project".to_string());
    app.skills_settings.selected_skill_id = Some("skill-a".to_string());
    let _ = update(
        &mut app,
        SettingsMessage::SkillsCreateNewCompleted(Ok("/tmp/project/.skills/new".to_string())),
    );
    assert!(app.skills_settings.loading);
    let _ = update(
        &mut app,
        SettingsMessage::SkillsCreateNewCompleted(Err("create failed".to_string())),
    );
    assert_eq!(app.skills_settings.status_message.as_deref(), Some("create failed"));
    let _ = update(
        &mut app,
        SettingsMessage::SkillsInstallBuiltInCompleted(Ok("/tmp/project/skill".to_string())),
    );
    assert!(app.skills_settings.detail_loading);
    let _ = update(
        &mut app,
        SettingsMessage::SkillsSetEnabledCompleted {
            skill_id: "skill-a".to_string(),
            enabled: false,
            result: Err("toggle failed".to_string()),
        },
    );
    assert_eq!(app.skills_settings.status_message.as_deref(), Some("toggle failed"));
    let _ = update(
        &mut app,
        SettingsMessage::SkillsDeleteCompleted {
            skill_id: "skill-a".to_string(),
            result: Ok("/tmp/project/skill".to_string()),
        },
    );
    assert!(app.skills_settings.selected_skill_id.is_none());

    app.skills_settings.save_error = Some("old".to_string());
    let _ = update(
        &mut app,
        SettingsMessage::SkillsDirectoryProviderChanged(SkillsDirectoryProvider::Codex),
    );
    assert_eq!(app.skills_settings.directory_provider, SkillsDirectoryProvider::Codex);
    assert!(app.skills_settings.save_error.is_none());
    let _ = update(&mut app, SettingsMessage::SkillsProjectAutoLoadToggled(false));
    assert!(!app.skills_settings.auto_load_project_skills);
    let _ = update(&mut app, SettingsMessage::SkillsOpenEnabledToggled(true));
    let _ = update(&mut app, SettingsMessage::SkillsOpenDirChanged(" /tmp/skills ".to_string()));
    let _ = update(
        &mut app,
        SettingsMessage::SkillsPromptInjectionModeChanged(SkillsPromptInjectionMode::Full),
    );
    assert_eq!(app.skills_settings.prompt_injection_mode, SkillsPromptInjectionMode::Full);
    let _ = update(&mut app, SettingsMessage::SkillsSave);
    let _ = update(&mut app, SettingsMessage::SkillsHelpOpen);
    assert!(app.skills_settings.show_help_modal);
    let _ = update(&mut app, SettingsMessage::SkillsHelpClose);
    assert!(!app.skills_settings.show_help_modal);
}

#[test]
fn skills_tab_change_clears_skill_detail() {
    let mut app = app();
    app.skills_settings.plugin_catalog = map_plugin_catalog_items(vec![plugin("plugin-a")]);
    app.skills_settings.plugin_loading = false;
    app.skills_settings.selected_skill_id = Some("skill-a".to_string());
    app.skills_settings.selected_skill_detail =
        Some(map_skill_detail(detail("skill-a", "personal")));
    app.skills_settings.detail_loading = true;
    app.skills_settings.detail_error = Some("stale detail error".to_string());

    let _ = update(&mut app, SettingsMessage::SkillsTabChanged(SkillsSettingsTab::DiscoveryOrder));

    assert_eq!(app.skills_settings.active_tab, SkillsSettingsTab::DiscoveryOrder);
    assert!(app.skills_settings.selected_skill_id.is_none());
    assert!(app.skills_settings.selected_skill_detail.is_none());
    assert!(!app.skills_settings.detail_loading);
    assert!(app.skills_settings.detail_error.is_none());
}
