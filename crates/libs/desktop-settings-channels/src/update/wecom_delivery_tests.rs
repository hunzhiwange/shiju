use super::wecom_delivery::{credentials, update};
use super::Services;
use crate::state::ChannelsSettingsState;
use crate::messages::ChannelsMessage;
use config_channel_services::WeComConfig;
fn services() -> Services {
    Services { config: desktop_config::ConfigAccess::new(gateway_endpoint::GatewayEndpoint::new("127.0.0.1", 1), None), client: Err("fixture".into()), project_choices: vec![] }
}
#[test]
fn wecom_delivery_modal_three_entries_keep_secrets_and_cancel_isolated() {
    use crate::state::WeComDeliveryMode;
    let mut app = ChannelsSettingsState::default();
    app.wecom =
        Some(WeComConfig { bot_id: "bot".into(), secret: "***MASKED***".into() });
    for mode in [WeComDeliveryMode::Qr, WeComDeliveryMode::Manual, WeComDeliveryMode::Webhook] {
        drop(update(
            &mut app,
            &services(),
            ChannelsMessage::WeComDeliveryOpened(mode),
        ));
        let draft = app.wecom_delivery.as_ref().expect("modal opens");
        assert_eq!(draft.mode, mode);
        assert!(draft.secret.is_empty());
        assert!(draft.webhook_url.is_empty());
        drop(update(
            &mut app,
            &services(),
            ChannelsMessage::WeComDeliveryChanged(
                "secret".into(),
                "replacement".into(),
            ),
        ));
        assert_eq!(app.wecom.as_ref().unwrap().secret, "***MASKED***");
        drop(update(&mut app, &services(), ChannelsMessage::WeComDeliveryClosed));
        assert!(app.wecom_delivery.is_none());
        assert_eq!(app.wecom.as_ref().unwrap().secret, "***MASKED***");
    }
}

#[test]
fn wecom_delivery_modal_save_failure_preserves_draft() {
    let mut app = ChannelsSettingsState::default();
    drop(update(
        &mut app,
        &services(),
        ChannelsMessage::WeComDeliveryOpened(
            crate::state::WeComDeliveryMode::Manual,
        ),
    ));
    drop(update(
        &mut app,
        &services(),
        ChannelsMessage::WeComDeliverySaved(Err("保存失败".into())),
    ));
    assert_eq!(
        app.wecom_delivery.as_ref().unwrap().error.as_deref(),
        Some("保存失败，请检查网关连接与配置后重试")
    );
}

#[test]
fn wecom_delivery_modal_retains_replaces_and_clears_masked_credentials() {
    use crate::state::WeComDeliveryMode;
    let mut app = ChannelsSettingsState::default();
    app.wecom =
        Some(WeComConfig { bot_id: "bot".into(), secret: "***MASKED***".into() });
    app.wecom_webhook =
        Some(config_channel_services::WeComWebhookConfig { webhook_url: "***MASKED***".into() });
    for mode in [WeComDeliveryMode::Manual, WeComDeliveryMode::Webhook] {
        drop(update(
            &mut app,
            &services(),
            ChannelsMessage::WeComDeliveryOpened(mode),
        ));
        let (wecom, webhook) = credentials(&app).unwrap();
        assert_eq!(wecom.unwrap().secret, "***MASKED***");
        assert_eq!(webhook.unwrap().webhook_url, "***MASKED***");
        let (field, replacement) = if mode == WeComDeliveryMode::Manual {
            ("secret", "replacement")
        } else {
            ("webhook_url", "https://qyapi.weixin.qq.com/cgi-bin/webhook/send?key=fixture")
        };
        drop(update(
            &mut app,
            &services(),
            ChannelsMessage::WeComDeliveryChanged(
                field.into(),
                replacement.into(),
            ),
        ));
        let (wecom, webhook) = credentials(&app).unwrap();
        if mode == WeComDeliveryMode::Manual {
            assert_eq!(wecom.unwrap().secret, replacement);
        } else {
            assert_eq!(webhook.unwrap().webhook_url, replacement);
        }
        drop(update(
            &mut app,
            &services(),
            ChannelsMessage::WeComDeliveryClearSecret,
        ));
        let (wecom, webhook) = credentials(&app).unwrap();
        if mode == WeComDeliveryMode::Manual {
            assert!(wecom.is_none());
            assert!(webhook.is_some());
        } else {
            assert!(webhook.is_none());
            assert!(wecom.is_some());
        }
    }
}

#[test]
fn wecom_delivery_modal_webhook_only_validates_without_bot_credentials() {
    let mut app = ChannelsSettingsState::default();
    drop(update(
        &mut app,
        &services(),
        ChannelsMessage::WeComDeliveryOpened(
            crate::state::WeComDeliveryMode::Webhook,
        ),
    ));
    for value in [
        "",
        "http://qyapi.weixin.qq.com/cgi-bin/webhook/send?key=fixture",
        "https://example.invalid/?key=fixture",
        "https://qyapi.weixin.qq.com/cgi-bin/webhook/send?key=fixture#fragment",
    ] {
        app.wecom_delivery.as_mut().unwrap().webhook_url = value.into();
        assert!(credentials(&app).is_err());
    }
    app.wecom_delivery.as_mut().unwrap().webhook_url =
        "https://qyapi.weixin.qq.com/cgi-bin/webhook/send?key=fixture".into();
    let (wecom, webhook) = credentials(&app).unwrap();
    assert!(wecom.is_none());
    assert!(webhook.is_some());
}
