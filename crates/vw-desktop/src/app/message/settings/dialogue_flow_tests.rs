// 验证对话流程权限配置的读取、重置与错误边界。
// 测试夹具围绕目标分支构造可控状态；跨模块行为由对应集成测试负责。

use super::*;
use crate::app::App;

fn app() -> App {
    App::new().0
}

fn has_scheduled_action(task: Task<Message>) -> bool {
    task.units() > 0
}

async fn collect_task_messages(task: Task<Message>) -> Vec<Message> {
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

fn permission_result(messages: Vec<Message>) -> Result<String, String> {
    let mut messages = messages.into_iter();
    let result = match messages.next() {
        Some(Message::Settings(SettingsMessage::DialogueFlowPermissionLoaded(result))) => result,
        _ => panic!("expected a dialogue flow permission result"),
    };
    assert!(messages.next().is_none());
    result
}

#[cfg(not(target_arch = "wasm32"))]
// 数据边界：覆盖规范化、默认值以及相关状态字段的既定规则。
// 这里固定可观察结果，避免依赖处理过程中的临时实现细节。
#[tokio::test(flavor = "multi_thread")]
async fn permission_load_without_project_or_config_returns_empty_object() {
    let without_project =
        permission_result(collect_task_messages(dialogue_flow_permission_text_task(None)).await);
    assert_eq!(without_project.as_deref(), Ok("{}"));

    let project = tempfile::tempdir().expect("temporary project should be created");
    let missing_config = permission_result(
        collect_task_messages(dialogue_flow_permission_text_task(Some(
            project.path().to_string_lossy().into_owned(),
        )))
        .await,
    );
    assert_eq!(missing_config.as_deref(), Ok("{}"));
    assert!(!project.path().join("config.json").exists());
}

#[cfg(not(target_arch = "wasm32"))]
#[tokio::test(flavor = "multi_thread")]
async fn permission_load_extracts_and_pretty_prints_permission_value() {
    let project = tempfile::tempdir().expect("temporary project should be created");
    std::fs::write(
        project.path().join("config.json"),
        r#"{"permission":{"shell":{"allow":true}},"theme":"dark"}"#,
    )
    .expect("config should be written");

    let result = permission_result(
        collect_task_messages(dialogue_flow_permission_text_task(Some(
            project.path().to_string_lossy().into_owned(),
        )))
        .await,
    );

    assert_eq!(result.as_deref(), Ok("{\n  \"shell\": {\n    \"allow\": true\n  }\n}"));
}

#[cfg(not(target_arch = "wasm32"))]
#[tokio::test(flavor = "multi_thread")]
async fn permission_load_defaults_missing_permission_and_rejects_invalid_json() {
    let project = tempfile::tempdir().expect("temporary project should be created");
    let config_path = project.path().join("config.json");
    std::fs::write(&config_path, r#"{"theme":"dark"}"#).expect("config should be written");

    let missing_permission = permission_result(
        collect_task_messages(dialogue_flow_permission_text_task(Some(
            project.path().to_string_lossy().into_owned(),
        )))
        .await,
    );
    assert_eq!(missing_permission.as_deref(), Ok("{}"));

    std::fs::write(&config_path, "{invalid json").expect("invalid config should be written");
    let invalid_json = permission_result(
        collect_task_messages(dialogue_flow_permission_text_task(Some(
            project.path().to_string_lossy().into_owned(),
        )))
        .await,
    );
    assert!(invalid_json.is_err());
}

#[cfg(not(target_arch = "wasm32"))]
#[tokio::test(flavor = "multi_thread")]
async fn permission_reset_removes_permission_and_preserves_other_fields() {
    let project = tempfile::tempdir().expect("temporary project should be created");
    let config_path = project.path().join("config.json");
    std::fs::write(&config_path, r#"{"permission":{"shell":"allow"},"theme":"dark","retries":3}"#)
        .expect("config should be written");

    let result = permission_result(
        collect_task_messages(dialogue_flow_permission_reset_task(Some(
            project.path().to_string_lossy().into_owned(),
        )))
        .await,
    );
    let saved: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(config_path).expect("updated config should be readable"),
    )
    .expect("updated config should remain valid JSON");

    assert_eq!(result.as_deref(), Ok("{}"));
    assert_eq!(saved.get("theme").and_then(serde_json::Value::as_str), Some("dark"));
    assert_eq!(saved.get("retries").and_then(serde_json::Value::as_u64), Some(3));
    assert!(saved.get("permission").is_none());
}

#[cfg(not(target_arch = "wasm32"))]
// 失败与边界：覆盖非法、缺失、过期或冲突场景的既定处理规则。
// 重点确认现有状态与错误反馈不会被意外的部分更新污染。
#[tokio::test(flavor = "multi_thread")]
async fn permission_reset_replaces_non_object_root_with_empty_object() {
    let project = tempfile::tempdir().expect("temporary project should be created");
    let config_path = project.path().join("config.json");
    std::fs::write(&config_path, r#"["unexpected"]"#).expect("config should be written");

    let result = permission_result(
        collect_task_messages(dialogue_flow_permission_reset_task(Some(
            project.path().to_string_lossy().into_owned(),
        )))
        .await,
    );

    assert_eq!(result.as_deref(), Ok("{}"));
    assert_eq!(
        std::fs::read_to_string(config_path).expect("updated config should be readable"),
        "{}"
    );
}

#[cfg(not(target_arch = "wasm32"))]
#[tokio::test(flavor = "multi_thread")]
async fn permission_reset_without_config_does_not_create_file() {
    let project = tempfile::tempdir().expect("temporary project should be created");

    let result = permission_result(
        collect_task_messages(dialogue_flow_permission_reset_task(Some(
            project.path().to_string_lossy().into_owned(),
        )))
        .await,
    );

    assert_eq!(result.as_deref(), Ok("{}"));
    assert!(!project.path().join("config.json").exists());
}

#[cfg(not(target_arch = "wasm32"))]
#[tokio::test(flavor = "multi_thread")]
async fn permission_reset_without_project_returns_empty_object() {
    let result =
        permission_result(collect_task_messages(dialogue_flow_permission_reset_task(None)).await);

    assert_eq!(result.as_deref(), Ok("{}"));
}

#[cfg(not(target_arch = "wasm32"))]
#[tokio::test(flavor = "multi_thread")]
async fn permission_load_reports_non_missing_file_errors() {
    let project = tempfile::tempdir().expect("temporary project should be created");
    std::fs::create_dir(project.path().join("config.json"))
        .expect("config-shaped directory should be created");

    let result = permission_result(
        collect_task_messages(dialogue_flow_permission_text_task(Some(
            project.path().to_string_lossy().into_owned(),
        )))
        .await,
    );

    assert!(result.is_err());
}

#[cfg(not(target_arch = "wasm32"))]
#[tokio::test(flavor = "multi_thread")]
async fn permission_reset_rejects_invalid_json_without_overwriting_it() {
    let project = tempfile::tempdir().expect("temporary project should be created");
    let config_path = project.path().join("config.json");
    let invalid_json = "{invalid json";
    std::fs::write(&config_path, invalid_json).expect("invalid config should be written");

    let result = permission_result(
        collect_task_messages(dialogue_flow_permission_reset_task(Some(
            project.path().to_string_lossy().into_owned(),
        )))
        .await,
    );

    assert!(result.is_err());
    assert_eq!(
        std::fs::read_to_string(config_path).expect("original config should remain readable"),
        invalid_json
    );
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

    let _ =
        update(&mut app, SettingsMessage::DialogueFlowUiSettingsLoaded(Ok((false, true, true))));
    assert!(!app.dialogue_flow_show_reasoning_summary);
    assert!(app.dialogue_flow_expand_shell_tool_section);
    assert!(app.dialogue_flow_expand_edit_tool_section);
}

#[test]
fn dialogue_flow_toggle_and_save_feedback_update_messages() {
    let mut app = app();

    let _ = update(
        &mut app,
        SettingsMessage::DialogueFlowUiSettingsLoaded(Err("bad config".to_string())),
    );
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
    assert!(has_scheduled_action(update(
        &mut app,
        SettingsMessage::DialogueFlowShowReasoningSummaryToggled(false),
    )));
    assert!(has_scheduled_action(update(
        &mut app,
        SettingsMessage::DialogueFlowExpandShellToolSectionToggled(true),
    )));
    assert!(has_scheduled_action(update(
        &mut app,
        SettingsMessage::DialogueFlowExpandEditToolSectionToggled(true),
    )));
}

#[test]
fn loaded_success_preserves_existing_feedback_and_loaded_error_preserves_toggles() {
    let mut app = app();
    app.dialogue_flow_settings_save_message = Some("existing feedback".to_string());

    let success_task =
        update(&mut app, SettingsMessage::DialogueFlowUiSettingsLoaded(Ok((false, true, false))));
    assert!(!has_scheduled_action(success_task));
    assert_eq!(app.dialogue_flow_settings_save_message.as_deref(), Some("existing feedback"));

    let error_task = update(
        &mut app,
        SettingsMessage::DialogueFlowUiSettingsLoaded(Err("unavailable".to_string())),
    );
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
