use super::*;

#[test]
fn wecom_config_requires_bot_id_and_secret() {
    let valid = WeComConfig { bot_id: "bot".into(), secret: "secret".into() };
    assert!(valid.has_required_credentials());
    assert!(
        !WeComConfig { bot_id: " ".into(), secret: "secret".into() }.has_required_credentials()
    );
    assert!(!WeComConfig { bot_id: "bot".into(), secret: "\t".into() }.has_required_credentials());
}

#[test]
fn wecom_config_round_trips_json() {
    let config: WeComConfig =
        serde_json::from_str(r#"{"bot_id":"bot","secret":"secret"}"#).unwrap();
    assert_eq!(config.bot_id, "bot");
    assert_eq!(config.secret, "secret");
}
