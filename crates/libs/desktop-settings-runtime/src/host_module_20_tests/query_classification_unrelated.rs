use crate::messages::SettingsMessage;
use crate::query_classification::update;
use crate::state::QueryClassificationRuleInput;
#[test]
fn update_ignores_messages_for_other_settings_sections() {
    let mut app = crate::state::QueryClassificationSettingsState::default();
    app.enabled = true;
    app.rules = vec![QueryClassificationRuleInput::default()];
    app.save_error = Some("keep".to_string());

    let _ = update(&mut app, SettingsMessage::ReliabilityHelpClose);

    assert!(app.enabled);
    assert_eq!(app.rules.len(), 1);
    assert_eq!(app.save_error.as_deref(), Some("keep"));
}
