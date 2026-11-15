use super::*;
use desktop_workflow_model::{WorkflowAppMeta, WorkflowDocument};
use desktop_workflow_state::state::*;
use iced::widget::{button, text_editor};
use iced::{Theme, Vector};
use serde_yaml::Value;
#[test]
fn workflow_app_display_name_uses_fallback_only_for_blank_names() {
    assert_eq!(
        workflow_app_display_name(desktop_i18n::I18n::new(Default::default()), ""),
        "未命名应用"
    );
    assert_eq!(
        workflow_app_display_name(desktop_i18n::I18n::new(Default::default()), "   "),
        "未命名应用"
    );
    assert_eq!(
        workflow_app_display_name(desktop_i18n::I18n::new(Default::default()), " 客服助手 "),
        " 客服助手 "
    );
}

#[test]
fn workflow_app_switch_option_display_returns_the_prepared_label() {
    let saved = WorkflowAppSwitchOption {
        id: "saved".to_string(),
        label: "🤖 已保存应用".to_string(),
        source: WorkflowAppSwitchSource::SavedUuid("saved".to_string()),
    };
    let open = WorkflowAppSwitchOption {
        id: "open".to_string(),
        label: "🧪 草稿应用 *".to_string(),
        source: WorkflowAppSwitchSource::OpenApp("open".to_string()),
    };

    assert_eq!(saved.to_string(), "🤖 已保存应用");
    assert_eq!(open.to_string(), "🧪 草稿应用 *");
}

#[test]
fn workflow_app_switch_options_include_saved_apps_with_dirty_active_marker() {
    let state = WorkflowState {
        local_uuid: Some("saved-a".to_string()),
        active_is_dirty: true,
        saved_apps: vec![
            WorkflowSavedAppSummary {
                uuid: "saved-a".to_string(),
                name: "  ".to_string(),
                description: String::new(),
                created_at_ms: 0,
                updated_at_ms: 0,
            },
            WorkflowSavedAppSummary {
                uuid: "saved-b".to_string(),
                name: "知识助手".to_string(),
                description: String::new(),
                created_at_ms: 0,
                updated_at_ms: 0,
            },
        ],
        ..WorkflowState::default()
    };

    let options =
        workflow_app_switch_options(desktop_i18n::I18n::new(Default::default()), &state);

    assert_eq!(options.len(), 2);
    assert_eq!(options[0].id, "saved-a");
    assert_eq!(options[0].label, "🤖 未命名应用 *");
    assert_eq!(options[0].source, WorkflowAppSwitchSource::SavedUuid("saved-a".to_string()));
    assert_eq!(options[1].label, "🤖 知识助手");
}

#[test]
fn workflow_app_switch_options_include_only_open_apps_missing_from_saved_list() {
    let saved_uuid = "saved-a".to_string();
    let state = WorkflowState {
        active_app_id: Some("open-active".to_string()),
        source_name: "运行中应用".to_string(),
        active_is_dirty: true,
        saved_apps: vec![WorkflowSavedAppSummary {
            uuid: saved_uuid.clone(),
            name: "已保存".to_string(),
            description: String::new(),
            created_at_ms: 0,
            updated_at_ms: 0,
        }],
        apps: vec![
            test_app_entry("open-active", None, "旧名称", "🧪", false),
            test_app_entry("open-dirty", None, "草稿应用", "🧠", true),
            test_app_entry("open-saved", Some(saved_uuid), "不应重复", "🤖", true),
        ],
        ..WorkflowState::default()
    };

    let options =
        workflow_app_switch_options(desktop_i18n::I18n::new(Default::default()), &state);

    assert_eq!(options.len(), 3);
    assert_eq!(options[0].label, "🤖 已保存");
    assert_eq!(options[1].label, "🧪 运行中应用 *");
    assert_eq!(options[1].source, WorkflowAppSwitchSource::OpenApp("open-active".to_string()));
    assert_eq!(options[2].label, "🧠 草稿应用 *");
}

#[test]
fn build_app_switcher_handles_empty_saved_and_open_apps() {
    let empty_state = WorkflowState::default();
    let _ = build_app_switcher(desktop_i18n::I18n::new(Default::default()), &empty_state);

    let saved_state = WorkflowState {
        local_uuid: Some("saved".to_string()),
        saved_apps: vec![WorkflowSavedAppSummary {
            uuid: "saved".to_string(),
            name: "已保存应用".to_string(),
            description: String::new(),
            created_at_ms: 0,
            updated_at_ms: 0,
        }],
        ..WorkflowState::default()
    };
    let _ = build_app_switcher(desktop_i18n::I18n::new(Default::default()), &saved_state);

    let open_state = WorkflowState {
        active_app_id: Some("open".to_string()),
        apps: vec![test_app_entry("open", None, "打开应用", "🤖", false)],
        ..WorkflowState::default()
    };
    let _ = build_app_switcher(desktop_i18n::I18n::new(Default::default()), &open_state);
}

#[test]
fn build_app_switcher_prefers_saved_selection_when_saved_uuid_is_active() {
    let state = WorkflowState {
        local_uuid: Some("saved".to_string()),
        active_app_id: Some("open".to_string()),
        saved_apps: vec![WorkflowSavedAppSummary {
            uuid: "saved".to_string(),
            name: "已保存应用".to_string(),
            description: String::new(),
            created_at_ms: 0,
            updated_at_ms: 0,
        }],
        apps: vec![test_app_entry("open", None, "打开应用", "🧪", false)],
        ..WorkflowState::default()
    };
    let options =
        workflow_app_switch_options(desktop_i18n::I18n::new(Default::default()), &state);

    assert!(matches!(options[0].source, WorkflowAppSwitchSource::SavedUuid(_)));
    assert!(matches!(options[1].source, WorkflowAppSwitchSource::OpenApp(_)));
    let _ = build_app_switcher(desktop_i18n::I18n::new(Default::default()), &state);
}

#[test]
fn style_helpers_cover_light_dark_and_input_statuses() {
    for theme in [Theme::Light, Theme::Dark] {
        assert_eq!(is_dark_theme(&theme), matches!(theme, Theme::Dark));

        let _ = editor_style(&theme, text_editor::Status::Active);
        let _ = editor_style(&theme, text_editor::Status::Hovered);
        let jump_active = next_step_jump_card_style(&theme, button::Status::Active);
        let jump_hovered = next_step_jump_card_style(&theme, button::Status::Hovered);
        let jump_pressed = next_step_jump_card_style(&theme, button::Status::Pressed);
        assert!(jump_active.background.is_some());
        assert!(jump_hovered.background.is_some());
        assert!(jump_pressed.background.is_some());
    }
}


fn test_app_entry(
    id: &str,
    local_uuid: Option<String>,
    name: &str,
    icon: &str,
    is_dirty: bool,
) -> WorkflowAppEntry {
    let document = WorkflowDocument::default();
    let snapshot = WorkflowHistorySnapshot {
        meta: WorkflowAppMeta {
            name: name.to_string(),
            icon: icon.to_string(),
            ..WorkflowAppMeta::default()
        },
        document: document.clone(),
        environment_variables: Vec::new(),
        conversation_variables: Vec::new(),
        pan: Vector::new(0.0, 0.0),
        zoom: 1.0,
        selected_node_id: None,
        selected_edge_id: None,
    };

    WorkflowAppEntry {
        id: id.to_string(),
        local_uuid,
        meta: snapshot.meta.clone(),
        source_path: None,
        raw_root: Value::Null,
        document,
        environment_variables: Vec::new(),
        conversation_variables: Vec::new(),
        pan: Vector::new(0.0, 0.0),
        zoom: 1.0,
        selected_node_id: None,
        selected_edge_id: None,
        connection_draft: None,
        is_dirty,
        undo_stack: Vec::new(),
        redo_stack: Vec::new(),
        saved_snapshot: snapshot,
    }
}
