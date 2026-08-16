use super::*;
use crate::app::{App, message};
use iced::Element;
use iced::widget::text;

fn test_app() -> App {
    App::new().0
}

fn keep_element(element: Element<'_, Message>) {
    std::hint::black_box(element);
}

fn settings_message(message: Message) -> message::SettingsMessage {
    match message {
        Message::Settings(settings) => settings,
        _ => panic!("expected settings message"),
    }
}

#[test]
fn field_row_accepts_reliability_control() {
    let element: Element<'_, Message> =
        field_row("Provider 重试", "请求失败时的最大重试次数。", text("2"));
    drop(element);
}

#[test]
fn reliability_view_uses_expected_bounded_ranges() {
    let source = include_str!("system_settings_reliability.rs");

    assert!(source.contains("0.0..=20.0"));
    assert!(source.contains("0.0..=60_000.0"));
    assert!(source.contains("1.0..=3600.0"));
    assert!(source.contains("s.channel_initial_backoff_secs as f32..=3600.0"));
    assert!(source.contains("provider_retries_changed"));
    assert!(source.contains("provider_backoff_ms_changed"));
    assert!(source.contains("channel_initial_backoff_secs_changed"));
    assert!(source.contains("channel_max_backoff_secs_changed"));
    assert!(source.contains("scheduler_poll_secs_changed"));
    assert!(source.contains("scheduler_retries_changed"));
}

#[test]
fn view_builds_default_extreme_and_error_states() {
    let app = test_app();
    keep_element(view(&app));

    let mut app = test_app();
    app.reliability_settings.provider_retries = 20;
    app.reliability_settings.provider_backoff_ms = 60_000;
    app.reliability_settings.channel_initial_backoff_secs = 3600;
    app.reliability_settings.channel_max_backoff_secs = 3600;
    app.reliability_settings.scheduler_poll_secs = 3600;
    app.reliability_settings.scheduler_retries = 20;
    app.reliability_settings.save_error = Some("save failed".to_string());
    keep_element(view(&app));
}

#[test]
fn overlays_return_dialog_or_help_modal() {
    let mut app = test_app();
    keep_element(view_overlays(&app, text("dialog").into()));

    app.reliability_settings.show_help_modal = true;
    keep_element(view_overlays(&app, text("dialog").into()));
}

#[test]
fn reliability_help_explains_affected_features_and_omits_removed_fields() {
    let help_text = RELIABILITY_HELP_TEXT;

    for expected in [
        "桌面聊天、Gateway、Channel、Task Plan 和 ACP",
        "首次模型请求失败后",
        "重试次数越大",
        "首次重试前",
        "等待越长",
        "聊天通道监听断开后",
        "后续重连等待",
        "cron 扫描",
        "扫描越频繁",
        "cron 任务执行失败后",
        "Provider 设置保存后对新发起的模型请求生效",
        "频道退避设置在聊天通道监听器下次启动或重启时生效",
        "调度设置在 cron 调度器下次启动或重启时生效",
        r#""provider_retries": 3"#,
        r#""provider_backoff_ms": 500"#,
        r#""channel_initial_backoff_secs": 1"#,
        r#""channel_max_backoff_secs": 60"#,
        r#""scheduler_poll_secs": 60"#,
        r#""scheduler_retries": 3"#,
    ] {
        assert!(help_text.contains(expected), "missing help text: {expected}");
    }

    for removed in [
        "fallback_providers",
        "fallback_api_keys",
        "api_keys",
        "model_fallbacks",
    ] {
        assert!(!help_text.contains(removed), "removed field remains in help: {removed}");
    }
}

#[test]
fn slider_callbacks_round_into_reliability_messages() {
    match settings_message(provider_retries_changed(2.4)) {
        message::SettingsMessage::ReliabilityProviderRetriesChanged(value) => assert_eq!(value, 2),
        _ => panic!("expected provider retries message"),
    }

    match settings_message(provider_backoff_ms_changed(499.6)) {
        message::SettingsMessage::ReliabilityProviderBackoffMsChanged(value) => {
            assert_eq!(value, 500);
        }
        _ => panic!("expected provider backoff message"),
    }

    match settings_message(channel_initial_backoff_secs_changed(1.5)) {
        message::SettingsMessage::ReliabilityChannelInitialBackoffSecsChanged(value) => {
            assert_eq!(value, 2);
        }
        _ => panic!("expected channel initial backoff message"),
    }

    match settings_message(channel_max_backoff_secs_changed(59.5)) {
        message::SettingsMessage::ReliabilityChannelMaxBackoffSecsChanged(value) => {
            assert_eq!(value, 60);
        }
        _ => panic!("expected channel max backoff message"),
    }

    match settings_message(scheduler_poll_secs_changed(14.5)) {
        message::SettingsMessage::ReliabilitySchedulerPollSecsChanged(value) => {
            assert_eq!(value, 15);
        }
        _ => panic!("expected scheduler poll message"),
    }

    match settings_message(scheduler_retries_changed(1.5)) {
        message::SettingsMessage::ReliabilitySchedulerRetriesChanged(value) => {
            assert_eq!(value, 2);
        }
        _ => panic!("expected scheduler retries message"),
    }
}
