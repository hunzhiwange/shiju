use super::*;

#[test]
fn cc_route_089_ui_config_expected_behavior() {
    let original = RoutingConfig {
        enabled: true,
        listen_address: "127.0.0.1".parse().unwrap(),
        listen_port: 9000,
        logging_enabled: true,
    };
    let mut state = State::default();
    state.load(Value::Listener(original));
    state.fields[1] = "9100".into();
    let Value::Listener(draft) = state.draft().unwrap() else { panic!("listener draft expected") };
    assert_eq!(draft.listen_port, 9100);
    assert!(draft.enabled && draft.logging_enabled);
    let Some(Value::Listener(loaded)) = state.value else { panic!("loaded value expected") };
    assert_eq!(loaded, original);
}

#[test]
fn cc_route_089_timeout_zero_only_disables_idle() {
    let mut state = State::default();
    state.load(Value::App(AppRoutingConfig {
        first_byte_timeout_secs: 30,
        idle_timeout_secs: 0,
        non_streaming_timeout_secs: 60,
    }));
    assert!(state.draft().is_ok());
    state.fields[0] = "0".into();
    assert!(state.draft().is_err());
    state.fields[0] = "30".into();
    state.fields[2] = "0".into();
    assert!(state.draft().is_err());
}
