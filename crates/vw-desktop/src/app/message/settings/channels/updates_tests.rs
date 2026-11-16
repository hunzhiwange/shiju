// 验证频道设置消息的状态同步、数值钳制与派生输入刷新。
// 测试夹具围绕目标分支构造可控状态；跨模块行为由对应集成测试负责。

use super::*;
use crate::app::App;
use std::sync::Arc;
use gateway_channel_qr::{
    ChannelQrFetchResponseDto, ChannelQrStatusDto, ChannelQrStatusResponseDto,
};

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
fn wecom_delivery_modal_close_ignores_late_qr_and_preserves_cron_draft() {
    let mut app = app();
    app.cron_settings.add_draft.prompt = "保持任务内容".into();
    drop(update(
        &mut app,
        SettingsMessage::Channels(ChannelsMessage::WeComDeliveryOpened(
            crate::app::state::WeComDeliveryMode::Qr,
        )),
    ));
    let generation = app.channels_settings.qr_flow.generation;
    drop(update(&mut app, SettingsMessage::Channels(ChannelsMessage::WeComDeliveryClosed)));
    drop(update(
        &mut app,
        SettingsMessage::Channels(ChannelsMessage::QrPolled {
            channel: "wecom".into(),
            generation,
            result: Ok(ChannelQrStatusResponseDto {
                status: ChannelQrStatusDto::Authorized,
                credentials: Some(
                    serde_json::from_value(serde_json::json!({"bot_id":"late", "secret":"late"}))
                        .unwrap(),
                ),
            }),
        }),
    ));
    assert!(app.channels_settings.wecom.is_none());
    assert_eq!(app.cron_settings.add_draft.prompt, "保持任务内容");
}

#[test]
fn wecom_delivery_modal_qr_credentials_stay_in_draft_until_saved() {
    let mut app = app();
    drop(update(
        &mut app,
        SettingsMessage::Channels(ChannelsMessage::WeComDeliveryOpened(
            crate::app::state::WeComDeliveryMode::Qr,
        )),
    ));
    let generation = app.channels_settings.qr_flow.generation;
    drop(update(
        &mut app,
        SettingsMessage::Channels(ChannelsMessage::QrPolled {
            channel: "wecom".into(),
            generation,
            result: Ok(ChannelQrStatusResponseDto {
                status: ChannelQrStatusDto::Authorized,
                credentials: Some(
                    serde_json::from_value(serde_json::json!({"bot_id":"bot", "secret":"fixture"}))
                        .unwrap(),
                ),
            }),
        }),
    ));
    assert!(app.channels_settings.wecom.is_none());
    assert_eq!(app.channels_settings.wecom_delivery.as_ref().unwrap().bot_id, "bot");
    assert!(app.channels_settings.qr_flow.credentials_ready);
    drop(update(&mut app, SettingsMessage::Channels(ChannelsMessage::WeComDeliverySaved(Ok(())))));
    assert_eq!(app.channels_settings.wecom.as_ref().unwrap().bot_id, "bot");
    assert!(app.channels_settings.wecom_delivery.is_none());
}

#[test]
fn wecom_delivery_modal_save_uses_real_config_api_and_retains_draft_on_failure() {
    const CHILD: &str = "SHIJU_WECOM_MODAL_CONFIG_CHILD";
    if std::env::var_os(CHILD).is_none() {
        let home = tempfile::tempdir().unwrap();
        let cfg = config_ui::GatewayClientSystemSettingsConfig {
            host: "[".into(),
            servers: Vec::new(),
            ..Default::default()
        };
        std::fs::write(
            home.path().join("gateway-client-bootstrap.json"),
            serde_json::to_vec(&cfg).unwrap(),
        )
        .unwrap();
        let output = std::process::Command::new(std::env::current_exe().unwrap())
            .args(["--exact", &format!("{}::wecom_delivery_modal_save_uses_real_config_api_and_retains_draft_on_failure", module_path!().split_once("::").unwrap().1)])
            .env(CHILD, "1").env("HOME", home.path()).env("VIBEWINDOW_CONFIG_DIR", home.path()).env("VIBEWINDOW_WORKSPACE", home.path())
            .current_dir(home.path()).output().unwrap();
        assert!(
            output.status.success()
                && String::from_utf8_lossy(&output.stdout).contains("1 passed; 0 failed"),
            "{}\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        return;
    }
    use crate::app::config::desktop_test_gateway::{TestServer, set_gateway_port, temp_home};
    use iced_runtime::futures::futures::StreamExt;
    let mut app = app();
    let (_lock, _home, _guard) = temp_home();
    for status in [500, 200] {
        let observed = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let observed_requests = observed.clone();
        let server = TestServer::start_matching(
            2,
            |request| matches!(request.path.as_str(), "/v1/config" | "/v1/global/config"),
            move |request| {
                observed_requests.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                if request.method == "GET" {
                    (
                        200,
                        r#"{"channels_config":{"cli":true,"wecom":{"bot_id":"existing","secret":"***MASKED***"}}}"#,
                    )
                } else {
                    assert_eq!(request.method, "PATCH");
                    let value: serde_json::Value = serde_json::from_str(&request.body).unwrap();
                    assert_eq!(value["channels_config"]["wecom"]["bot_id"], "existing");
                    assert_eq!(
                        value["channels_config"]["wecom_webhook"]["webhook_url"],
                        "https://qyapi.weixin.qq.com/cgi-bin/webhook/send?key=fixture"
                    );
                    (
                        status,
                        if status == 200 {
                            "{}"
                        } else {
                            r#"{"error":"https://example.invalid/secret"}"#
                        },
                    )
                }
            },
        );
        set_gateway_port(server.port);
        drop(update(
            &mut app,
            SettingsMessage::Channels(ChannelsMessage::WeComDeliveryOpened(
                crate::app::state::WeComDeliveryMode::Webhook,
            )),
        ));
        drop(update(
            &mut app,
            SettingsMessage::Channels(ChannelsMessage::WeComDeliveryChanged(
                "webhook_url".into(),
                "https://qyapi.weixin.qq.com/cgi-bin/webhook/send?key=fixture".into(),
            )),
        ));
        let task = update(&mut app, SettingsMessage::Channels(ChannelsMessage::WeComDeliverySave));
        assert!(app.channels_settings.wecom_delivery.as_ref().unwrap().saving);
        let output = tokio::runtime::Runtime::new()
            .unwrap()
            .block_on(async {
                let mut stream =
                    iced_runtime::task::into_stream(task).ok_or("missing save task")?;
                match stream.next().await {
                    Some(iced_runtime::Action::Output(Message::Settings(
                        SettingsMessage::Channels(message),
                    ))) => Ok::<_, String>(message),
                    _ => Err("missing save result".into()),
                }
            })
            .unwrap();
        drop(update(&mut app, SettingsMessage::Channels(output)));
        if status == 500 {
            assert!(app.channels_settings.wecom_webhook.is_none());
            let draft = app.channels_settings.wecom_delivery.as_ref().unwrap();
            assert!(!draft.saving);
            assert!(!draft.error.as_ref().unwrap().contains("example.invalid"));
            assert!(draft.webhook_url.ends_with("key=fixture"));
        } else {
            assert!(app.channels_settings.wecom_webhook.is_some());
            assert!(app.channels_settings.wecom_delivery.is_none());
        }
        assert_eq!(observed.load(std::sync::atomic::Ordering::SeqCst), 2);
        assert_eq!(server.take_requests().len(), 2);
    }
}

#[test]
fn wecom_delivery_modal_expired_retry_preserves_job_draft() {
    let mut app = app();
    app.cron_settings.edit_draft.prompt = "保留编辑中的任务".into();
    drop(update(
        &mut app,
        SettingsMessage::Channels(ChannelsMessage::WeComDeliveryOpened(
            crate::app::state::WeComDeliveryMode::Qr,
        )),
    ));
    let generation = app.channels_settings.qr_flow.generation;
    drop(update(
        &mut app,
        SettingsMessage::Channels(ChannelsMessage::QrPolled {
            channel: "wecom".into(),
            generation,
            result: Ok(ChannelQrStatusResponseDto {
                status: ChannelQrStatusDto::Expired,
                credentials: None,
            }),
        }),
    ));
    assert!(app.channels_settings.wecom_delivery.is_some());
    assert_eq!(app.channels_settings.qr_flow.status, Some(ChannelQrStatusDto::Expired));
    drop(update(&mut app, SettingsMessage::Channels(ChannelsMessage::QrRequested("wecom".into()))));
    assert!(app.channels_settings.qr_flow.generation > generation);
    assert_eq!(app.cron_settings.edit_draft.prompt, "保留编辑中的任务");
}
