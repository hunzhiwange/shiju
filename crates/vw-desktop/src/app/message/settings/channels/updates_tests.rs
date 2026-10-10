// 验证频道设置消息的状态同步、数值钳制与派生输入刷新。
// 测试夹具围绕目标分支构造可控状态；跨模块行为由对应集成测试负责。

use super::*;
use crate::app::App;
use std::sync::Arc;
use vw_config_types::channels::LarkReceiveMode;

fn app() -> App {
    let mut app = App::new().0;
    app.channels_settings = Default::default();
    app
}

// 失败与边界：覆盖非法、缺失、过期或冲突场景的既定处理规则。
// 重点确认现有状态与错误反馈不会被意外的部分更新污染。
#[test]
fn channel_update_ignores_non_channel_messages() {
    let mut app = app();
    app.channels_settings.save_error = Some("keep".to_string());

    let _ = update(&mut app, SettingsMessage::ToggleSettingsSidebar);

    assert_eq!(app.channels_settings.save_error.as_deref(), Some("keep"));
    assert!(app.channels_settings.cli);
}

#[test]
fn channel_refresh_clears_error_and_tab_selection_updates_state() {
    let mut app = app();
    app.channels_settings.save_error = Some("old".to_string());

    let _ = update(&mut app, SettingsMessage::Channels(ChannelsMessage::Refresh));
    assert!(app.channels_settings.save_error.is_none());

    let _ = update(
        &mut app,
        SettingsMessage::Channels(ChannelsMessage::TabSelected(
            crate::app::state::ChannelsSettingsTab::Telegram,
        )),
    );
    assert_eq!(app.channels_settings.active_tab, crate::app::state::ChannelsSettingsTab::Telegram);
}

#[test]
fn channel_scalar_updates_store_values_and_clamp_timeout() {
    let mut app = app();

    let _ = update(&mut app, SettingsMessage::Channels(ChannelsMessage::CliToggled(true)));
    let _ = update(
        &mut app,
        SettingsMessage::Channels(ChannelsMessage::ProjectDirChanged(" /tmp/work ".to_string())),
    );
    let _ =
        update(&mut app, SettingsMessage::Channels(ChannelsMessage::MessageTimeoutSecsChanged(0)));

    assert!(app.channels_settings.cli);
    assert_eq!(app.channels_settings.project_dir_input, " /tmp/work ");
    assert_eq!(app.channels_settings.message_timeout_secs, 1);
    assert!(app.channels_settings.save_error.is_none());
}


#[test]
fn channel_text_update_mirrors_input_and_applies_field() {
    let mut app = app();
    let _ = update(
        &mut app,
        SettingsMessage::Channels(ChannelsMessage::EnabledToggled("telegram".to_string(), true)),
    );

    let _ = update(
        &mut app,
        SettingsMessage::Channels(ChannelsMessage::TextChanged(
            "telegram.bot_token".to_string(),
            "token".to_string(),
        )),
    );

    assert_eq!(app.channels_settings.text_inputs["telegram.bot_token"], "token");
    assert_eq!(
        app.channels_settings.telegram.as_ref().expect("telegram enabled").bot_token,
        "token"
    );
}

#[test]
fn channel_text_editor_edit_updates_editor_mirror_and_channel_field() {
    let mut app = app();
    let key = "telegram.allowed_users".to_string();
    let _ = update(
        &mut app,
        SettingsMessage::Channels(ChannelsMessage::EnabledToggled("telegram".to_string(), true)),
    );
    app.channels_settings.save_error = Some("old".to_string());

    let _ = update(
        &mut app,
        SettingsMessage::Channels(ChannelsMessage::TextEditorAction(
            key.clone(),
            text_editor::Action::Edit(text_editor::Edit::Paste(Arc::new("alice\nbob".to_string()))),
        )),
    );

    assert_eq!(app.channels_settings.text_editors[&key].text(), "alice\nbob");
    assert_eq!(app.channels_settings.text_inputs[&key], "alice\nbob");
    assert_eq!(
        app.channels_settings.telegram.as_ref().expect("telegram enabled").allowed_users,
        vec!["alice".to_string(), "bob".to_string()]
    );
    assert!(app.channels_settings.save_error.is_none());
}

// 提交与删除：覆盖保存、安装或移除操作前后的状态衔接。
// 这里关注校验、任务载荷和结果反馈，不绑定底层实现。
#[test]
fn channel_text_editor_non_edit_does_not_persist_or_change_text() {
    let mut app = app();
    let key = "telegram.allowed_users".to_string();
    let _ = update(
        &mut app,
        SettingsMessage::Channels(ChannelsMessage::EnabledToggled("telegram".to_string(), true)),
    );
    app.channels_settings.save_error = Some("keep".to_string());

    let _ = update(
        &mut app,
        SettingsMessage::Channels(ChannelsMessage::TextEditorAction(
            key.clone(),
            text_editor::Action::SelectAll,
        )),
    );

    assert_eq!(app.channels_settings.text_editors[&key].text(), "");
    assert_eq!(app.channels_settings.text_inputs[&key], "");
    assert_eq!(app.channels_settings.save_error.as_deref(), Some("keep"));
}

#[test]
fn channel_text_editor_missing_key_ignores_content_but_edit_still_persists() {
    let mut app = app();
    let key = "missing.editor".to_string();
    app.channels_settings.save_error = Some("old".to_string());

    let _ = update(
        &mut app,
        SettingsMessage::Channels(ChannelsMessage::TextEditorAction(
            key.clone(),
            text_editor::Action::Edit(text_editor::Edit::Insert('x')),
        )),
    );

    assert!(!app.channels_settings.text_editors.contains_key(&key));
    assert!(!app.channels_settings.text_inputs.contains_key(&key));
    assert!(app.channels_settings.save_error.is_none());
}

#[test]
fn channel_typed_updates_apply_bool_number_and_receive_mode_normalization() {
    let mut app = app();
    for channel in ["telegram", "lark"] {
        let _ = update(
            &mut app,
            SettingsMessage::Channels(ChannelsMessage::EnabledToggled(channel.to_string(), true)),
        );
    }

    let _ = update(
        &mut app,
        SettingsMessage::Channels(ChannelsMessage::BoolToggled(
            "telegram.mention_only".to_string(),
            true,
        )),
    );
    let _ = update(
        &mut app,
        SettingsMessage::Channels(ChannelsMessage::NumberChanged(
            "telegram.draft_update_interval_ms".to_string(),
            20,
        )),
    );
    let _ = update(
        &mut app,
        SettingsMessage::Channels(ChannelsMessage::ReceiveModeChanged(
            "lark.receive_mode".to_string(),
            "webhook".to_string(),
        )),
    );

    let telegram = app.channels_settings.telegram.as_ref().expect("telegram enabled");
    assert!(telegram.mention_only);
    assert_eq!(telegram.draft_update_interval_ms, 100);
    assert_eq!(
        app.channels_settings.lark.as_ref().expect("lark enabled").receive_mode,
        LarkReceiveMode::Webhook
    );
}

// 失败与边界：覆盖非法、缺失、过期或冲突场景的既定处理规则。
// 重点确认现有状态与错误反馈不会被意外的部分更新污染。
#[test]
fn channel_cli_toggle_rejects_disabling_the_last_entry() {
    let mut app = app();

    let _ = update(&mut app, SettingsMessage::Channels(ChannelsMessage::CliToggled(false)));

    assert!(!app.channels_settings.cli);
    assert!(app.channels_settings.save_error.as_deref().unwrap_or("").contains("至少保留一个"));
}
