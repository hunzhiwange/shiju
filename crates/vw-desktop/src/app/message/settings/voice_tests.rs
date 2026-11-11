use super::*;
#[test]
fn voice_task_13_late_load_cannot_reopen_cancelled_form() {
    let (mut app, _) = App::new();
    app.show_system_settings = true;
    app.system_settings_tab = crate::app::components::system_settings::SystemTab::Voice;
    app.voice.settings.endpoint = Some(crate::app::gateway_client_endpoint());
    app.voice.settings.generation = 4;
    app.voice.settings.edit = Some(Edit::new(VoiceConfig::default(), Preferences::default()));
    let _ = update(&mut app, Event::Cancel);
    let _ = update(
        &mut app,
        Event::Loaded(
            4,
            Ok(Loaded {
                config: VoiceConfig::default(),
                local: Preferences::default(),
                devices: vec![],
                warning: None,
            }),
        ),
    );
    assert!(app.voice.settings.edit.is_none());
}
#[test]
fn voice_task_13_connection_identity_includes_tenant_and_auth() {
    let endpoint = GatewayEndpoint::new("localhost", 1234);
    let changed =
        endpoint.clone().with_auth(vw_gateway_client::GatewayAuth { skey: Some("fixture".into()) });
    assert!(!same_endpoint(&endpoint, &changed));
    assert!(same_endpoint(&endpoint, &endpoint.clone()));
}
