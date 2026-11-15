// 验证对话流程权限配置的读取、重置与错误边界。
// 测试夹具围绕目标分支构造可控状态；跨模块行为由对应集成测试负责。

use super::*;
use crate::app::App;

fn app() -> App {
    App::new().0
}

fn loaded(result: Result<(bool, bool, bool), String>) -> SettingsMessage {
    SettingsMessage::DialogueFlowUiSettingsLoaded(
        preferences_scope(&gateway_client_endpoint()),
        0,
        result,
    )
}

fn has_scheduled_action(task: Task<Message>) -> bool {
    task.units() > 0
}

// 任务与结果：覆盖任务构造、调度或结果回写对公开状态的影响。
// 完成、失败与过期响应均按当前场景的既定契约处理。
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
fn action_messages_schedule_follow_up_work() {
    let mut app = app();
    app.project_path = None;

    assert!(has_scheduled_action(
        update(&mut app, SettingsMessage::DialogueFlowPermissionRefresh,)
    ));
    assert!(has_scheduled_action(update(&mut app, SettingsMessage::DialogueFlowPermissionReset,)));
    assert!(has_scheduled_action(update(&mut app, SettingsMessage::DialogueFlowUiSettingsSave,)));
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

// 失败与边界：覆盖非法、缺失、过期或冲突场景的既定处理规则。
// 重点确认现有状态与错误反馈不会被意外的部分更新污染。
#[test]
fn unrelated_message_leaves_dialogue_flow_state_unchanged_and_stays_idle() {
    let mut app = app();
    app.dialogue_flow_show_reasoning_summary = false;
    app.dialogue_flow_expand_shell_tool_section = true;
    app.dialogue_flow_expand_edit_tool_section = true;
    app.dialogue_flow_settings_save_message = Some("keep".to_string());
    app.dialogue_flow_permission_editor =
        iced::widget::text_editor::Content::with_text("keep permission");

    let task = update(&mut app, SettingsMessage::ToggleSettingsSidebar);

    assert!(!has_scheduled_action(task));
    assert!(!app.dialogue_flow_show_reasoning_summary);
    assert!(app.dialogue_flow_expand_shell_tool_section);
    assert!(app.dialogue_flow_expand_edit_tool_section);
    assert_eq!(app.dialogue_flow_settings_save_message.as_deref(), Some("keep"));
    assert_eq!(app.dialogue_flow_permission_editor.text(), "keep permission");
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn dialogue_flow_patch_failure_reaches_localized_feedback_and_queued_edits_save() {
    use crate::app::config::{drive_system_settings_patch, with_editor_settings_gateway};
    use crate::app::i18n::I18n;
    use serde_json::json;
    use config_ui::DesktopLocale;

    with_editor_settings_gateway(|mut app, requests, responses| {
        app.dialogue_flow_show_reasoning_summary = true;
        app.dialogue_flow_expand_shell_tool_section = false;
        app.dialogue_flow_expand_edit_tool_section = true;
        let stale_read = loaded(Ok((true, false, false)));
        let save = app.update(Message::Settings(SettingsMessage::DialogueFlowUiSettingsSave));
        assert!(requests.try_recv().is_err());
        let failed = drive_system_settings_patch(
            save,
            &requests,
            &responses,
            json!({
                "dialogue_flow_show_reasoning_summary": true,
                "dialogue_flow_expand_shell_tool_section": false,
                "dialogue_flow_expand_edit_tool_section": true
            }),
            None,
            || {
                assert_eq!(
                    app.update(Message::Settings(
                        SettingsMessage::DialogueFlowExpandShellToolSectionToggled(true)
                    ))
                    .units(),
                    0
                );
                assert_eq!(
                    app.update(Message::Settings(
                        SettingsMessage::DialogueFlowExpandShellToolSectionToggled(false)
                    ))
                    .units(),
                    0
                );
                assert_eq!(
                    app.update(Message::Settings(
                        SettingsMessage::DialogueFlowShowReasoningSummaryToggled(false)
                    ))
                    .units(),
                    0
                );
                let _ = app.update(Message::Settings(stale_read));
                assert!(!app.dialogue_flow_show_reasoning_summary);
                assert!(!app.dialogue_flow_expand_shell_tool_section);
                assert!(app.dialogue_flow_expand_edit_tool_section);
            },
        );
        assert!(matches!(&failed, Message::SystemSettingsSaved(_, _, Err(_))));
        let pending = app.update(failed);
        assert!(
            app.dialogue_flow_settings_translation
                .as_ref()
                .unwrap()
                .render(I18n::new(DesktopLocale::English))
                .unwrap()
                .starts_with("Failed to save:")
        );

        let saved = drive_system_settings_patch(
            pending,
            &requests,
            &responses,
            json!({
                "dialogue_flow_show_reasoning_summary": false,
                "dialogue_flow_expand_shell_tool_section": false
            }),
            Some("{}"),
            || {},
        );
        assert!(matches!(&saved, Message::SystemSettingsSaved(_, _, Ok(()))));
        assert_eq!(app.update(saved).units(), 0);
        assert_eq!(app.dialogue_flow_settings_save_message.as_deref(), Some("已保存对话流配置"));
    });
}
