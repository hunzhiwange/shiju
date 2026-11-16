use crate::dialogue_flow_update::{self as domain, Message as SettingsMessage};
use desktop_config::config_desktop::{DesktopPreferencesPersistence, preferences_scope};
use iced::Task;
#[derive(Default)]
struct DialogueFixture {
    desktop_preferences_persistence: DesktopPreferencesPersistence,
    dialogue_flow_permission_editor: iced::widget::text_editor::Content,
    dialogue_flow_show_reasoning_summary: bool,
    dialogue_flow_expand_shell_tool_section: bool,
    dialogue_flow_expand_edit_tool_section: bool,
    dialogue_flow_settings_translation: Option<desktop_i18n::format::Message>,
    dialogue_flow_settings_save_message: Option<String>,
}
fn endpoint() -> gateway_endpoint::GatewayEndpoint {
    gateway_endpoint::GatewayEndpoint::new("127.0.0.1", 1)
}
fn app() -> DialogueFixture {
    DialogueFixture::default()
}
fn loaded(result: Result<(bool, bool, bool), String>) -> SettingsMessage {
    SettingsMessage::DialogueFlowUiSettingsLoaded(preferences_scope(&endpoint()), 0, result)
}
fn has_scheduled_action(task: Task<SettingsMessage>) -> bool {
    task.units() > 0
}
fn update(app: &mut DialogueFixture, message: SettingsMessage) -> Task<SettingsMessage> {
    domain::update(
        &mut domain::Update {
            endpoint: endpoint(),
            project_path: None,
            desktop_preferences_persistence: &mut app.desktop_preferences_persistence,
            dialogue_flow_permission_editor: &mut app.dialogue_flow_permission_editor,
            dialogue_flow_show_reasoning_summary: &mut app.dialogue_flow_show_reasoning_summary,
            dialogue_flow_expand_shell_tool_section: &mut app
                .dialogue_flow_expand_shell_tool_section,
            dialogue_flow_expand_edit_tool_section: &mut app.dialogue_flow_expand_edit_tool_section,
            dialogue_flow_settings_translation: &mut app.dialogue_flow_settings_translation,
            dialogue_flow_settings_save_message: &mut app.dialogue_flow_settings_save_message,
            patches: Vec::new(),
        },
        message,
    )
}
#[test]
fn dialogue_flow_loaded_messages_update_ui_state() {
    let mut app = app();

    let _ = update(
        &mut app,
        SettingsMessage::DialogueFlowPermissionLoaded(Ok("{\n  \"allow\": true\n}".to_string())),
    );
    assert!(app.dialogue_flow_permission_editor.text().contains("\"allow\": true"));

    let _ =
        update(&mut app, SettingsMessage::DialogueFlowPermissionLoaded(Err("boom".to_string())));
    assert!(app.dialogue_flow_permission_editor.text().contains("读取失败: boom"));

    let _ = update(&mut app, loaded(Ok((false, true, true))));
    assert!(!app.dialogue_flow_show_reasoning_summary);
    assert!(app.dialogue_flow_expand_shell_tool_section);
    assert!(app.dialogue_flow_expand_edit_tool_section);
}

#[test]
fn dialogue_flow_toggle_and_save_feedback_update_messages() {
    let mut app = app();

    let _ = update(&mut app, loaded(Err("bad config".to_string())));
    assert_eq!(
        app.dialogue_flow_settings_save_message.as_deref(),
        Some("读取配置失败: bad config")
    );

    let _ = update(&mut app, SettingsMessage::DialogueFlowShowReasoningSummaryToggled(true));
    let _ = update(&mut app, SettingsMessage::DialogueFlowExpandShellToolSectionToggled(false));
    let _ = update(&mut app, SettingsMessage::DialogueFlowExpandEditToolSectionToggled(false));
    assert!(app.dialogue_flow_show_reasoning_summary);
    assert!(!app.dialogue_flow_expand_shell_tool_section);
    assert!(!app.dialogue_flow_expand_edit_tool_section);

    let _ = update(&mut app, SettingsMessage::DialogueFlowUiSettingsSaved(Ok(())));
    assert_eq!(app.dialogue_flow_settings_save_message.as_deref(), Some("已保存对话流配置"));
    let _ = update(
        &mut app,
        SettingsMessage::DialogueFlowUiSettingsSaved(Err("write failed".to_string())),
    );
    assert_eq!(app.dialogue_flow_settings_save_message.as_deref(), Some("保存失败: write failed"));
}

#[test]
fn loaded_success_preserves_existing_feedback_and_loaded_error_preserves_toggles() {
    let mut app = app();
    app.dialogue_flow_settings_save_message = Some("existing feedback".to_string());

    let success_task = update(&mut app, loaded(Ok((false, true, false))));
    assert!(!has_scheduled_action(success_task));
    assert_eq!(app.dialogue_flow_settings_save_message.as_deref(), Some("existing feedback"));

    let error_task = update(&mut app, loaded(Err("unavailable".to_string())));
    assert!(!has_scheduled_action(error_task));
    assert!(!app.dialogue_flow_show_reasoning_summary);
    assert!(app.dialogue_flow_expand_shell_tool_section);
    assert!(!app.dialogue_flow_expand_edit_tool_section);
    assert_eq!(
        app.dialogue_flow_settings_save_message.as_deref(),
        Some("读取配置失败: unavailable")
    );
}
