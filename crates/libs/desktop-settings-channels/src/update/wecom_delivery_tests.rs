use super::wecom_delivery::{credentials, update};
use super::Services;
use crate::state::ChannelsSettingsState;
use crate::messages::ChannelsMessage;
use config_channel_services::WeComConfig;
fn services() -> Services {
    Services { config: desktop_config::ConfigAccess::new(gateway_endpoint::GatewayEndpoint::new("127.0.0.1", 1), None), client: Err("fixture".into()), project_choices: vec![] }
}
#[test]
fn wecom_delivery_modes_keep_secrets_and_cancel_isolated() {
    use crate::state::WeComDeliveryMode;
    let mut app = ChannelsSettingsState::default();
    app.wecom =
        Some(WeComConfig { bot_id: "bot".into(), secret: "***MASKED***".into() });
    for mode in [WeComDeliveryMode::Qr, WeComDeliveryMode::Manual] {
        drop(update(
            &mut app,
            &services(),
            ChannelsMessage::WeComDeliveryOpened(mode),
        ));
        let draft = app.wecom_delivery.as_ref().expect("modal opens");
        assert_eq!(draft.mode, mode);
        assert!(draft.secret.is_empty());
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
    drop(update(
        &mut app, &services(),
        ChannelsMessage::WeComDeliveryOpened(WeComDeliveryMode::Manual),
    ));
    assert_eq!(credentials(&app).unwrap().unwrap().secret, "***MASKED***");
    drop(update(
        &mut app, &services(),
        ChannelsMessage::WeComDeliveryChanged("secret".into(), "replacement".into()),
    ));
    assert_eq!(credentials(&app).unwrap().unwrap().secret, "replacement");
    drop(update(&mut app, &services(), ChannelsMessage::WeComDeliveryClearSecret));
    assert!(credentials(&app).unwrap().is_none());
    assert_eq!(app.wecom_webhook.as_ref().unwrap().webhook_url, "***MASKED***");
}

#[test]
fn named_wecom_qr_credentials_only_update_the_selected_instance_after_save() {
    use gateway_channel_qr::{ChannelQrStatusDto, ChannelQrStatusResponseDto};
    let mut app = ChannelsSettingsState::default();
    let services = services();
    app.wecom = Some(WeComConfig { bot_id: "default".into(), secret: "default-secret".into() });
    let handle = super::updates::update;
    for _ in 0..2 {
        drop(handle(&mut app, &services, ChannelsMessage::InstanceAdded("wecom".into())));
    }
    drop(handle(&mut app, &services, ChannelsMessage::QrRequested("wecom:2".into())));
    let generation = app.qr_flow.generation;
    let result = serde_json::json!({"bot_id": "named", "secret": "named-secret"});
    drop(handle(
        &mut app,
        &services,
        ChannelsMessage::QrPolled {
            channel: "wecom".into(),
            generation,
            result: Ok(ChannelQrStatusResponseDto {
                status: ChannelQrStatusDto::Authorized,
                credentials: result.as_object().cloned(),
            }),
        },
    ));

    assert!(app.qr_flow.credentials_ready);
    assert!(app.wecom_instances[1].config.secret.is_empty());
    drop(handle(&mut app, &services, ChannelsMessage::QrSaveConfirmed));
    assert_eq!(app.wecom.as_ref().unwrap().secret, "default-secret");
    assert!(app.wecom_instances[0].config.secret.is_empty());
    assert_eq!(app.wecom_instances[1].config.secret, "named-secret");
}

#[test]
fn named_wecom_modal_keeps_scanned_credentials_in_the_selected_draft() {
    use crate::state::WeComDeliveryMode;
    use gateway_channel_qr::{ChannelQrStatusDto, ChannelQrStatusResponseDto};

    let services = services();
    let mut app = ChannelsSettingsState::default();
    app.wecom = Some(WeComConfig {
        bot_id: "default".into(),
        secret: "default-secret".into(),
    });
    for _ in 0..2 {
        drop(super::updates::update(
            &mut app,
            &services,
            ChannelsMessage::InstanceAdded("wecom".into()),
        ));
    }
    drop(update(
        &mut app,
        &services,
        ChannelsMessage::WeComInstanceOpened("2".into(), WeComDeliveryMode::Qr),
    ));
    let generation = app.qr_flow.generation;
    let scanned = serde_json::json!({"bot_id": "named", "secret": "named-secret"});
    drop(super::updates::update(
        &mut app,
        &services,
        ChannelsMessage::QrPolled {
            channel: "wecom".into(),
            generation,
            result: Ok(ChannelQrStatusResponseDto {
                status: ChannelQrStatusDto::Authorized,
                credentials: scanned.as_object().cloned(),
            }),
        },
    ));

    assert_eq!(app.wecom_delivery.as_ref().unwrap().instance_id.as_deref(), Some("2"));
    assert_eq!(app.wecom_delivery.as_ref().unwrap().secret, "named-secret");
    assert!(app.wecom_instances[1].config.secret.is_empty());

    drop(update(&mut app, &services, ChannelsMessage::WeComDeliverySave));
    drop(update(&mut app, &services, ChannelsMessage::WeComDeliverySaved(Ok(()))));
    assert_eq!(app.wecom.as_ref().unwrap().secret, "default-secret");
    assert!(app.wecom_instances[0].config.secret.is_empty());
    assert_eq!(app.wecom_instances[1].config.secret, "named-secret");
}
