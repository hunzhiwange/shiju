
mod behavior_0 {
use crate::state::{QueryClassificationRuleInput, QueryClassificationSettingsState};
#[test]
fn route_classification_and_heartbeat_defaults_are_ready_to_edit_2() {
    let rule = QueryClassificationRuleInput::default();
    assert!(rule.pattern.is_empty());
    assert!(rule.category.is_empty());
    assert_eq!(rule.priority_input, "0");
    let classification = QueryClassificationSettingsState::default();
    assert!(!classification.enabled);
    assert!(classification.rules.is_empty());
    assert!(classification.save_error.is_none());


}
}
