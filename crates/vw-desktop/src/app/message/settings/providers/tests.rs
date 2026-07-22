#[test]
fn test_module_is_wired() {
    let module = module_path!();

    assert!(module.ends_with("tests"));
}

#[test]
fn update_forwards_close_and_clears_connect_state() {
    let (mut app, _) = crate::app::App::new();
    app.provider_settings.connect_modal = Some(crate::app::state::ProviderConnectState::default());
    app.provider_settings.connect_error = Some("invalid key".to_string());

    let _ = super::update(&mut app, super::SettingsMessage::ProviderConnectClose);

    assert!(app.provider_settings.connect_modal.is_none());
    assert!(app.provider_settings.connect_error.is_none());
}

#[test]
fn update_forwards_disconnect_request_payload_unchanged() {
    let (mut app, _) = crate::app::App::new();
    let provider_id = " custom-provider/测试 ".to_string();

    let _ = super::update(
        &mut app,
        super::SettingsMessage::ProviderDisconnectRequested(provider_id.clone()),
    );

    assert_eq!(
        app.provider_settings.disconnect_confirm_provider_id.as_deref(),
        Some(provider_id.as_str())
    );
}

#[test]
fn update_forwards_disconnect_error_and_clears_confirmation() {
    let (mut app, _) = crate::app::App::new();
    app.provider_settings.disconnect_confirm_provider_id = Some("openai".to_string());

    let _ = super::update(
        &mut app,
        super::SettingsMessage::ProviderDisconnectDone(Err("disconnect denied".to_string())),
    );

    assert_eq!(app.provider_settings.save_error.as_deref(), Some("disconnect denied"));
    assert!(app.provider_settings.disconnect_confirm_provider_id.is_none());
}
