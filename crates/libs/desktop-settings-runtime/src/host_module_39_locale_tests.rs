use desktop_i18n::I18n;
use config_ui::DesktopLocale;

#[test]
fn desktop_gettext_area_7_local_validation_feedback_switches_language() {
    let mut state = crate::state::QueryClassificationSettingsState::default();
    state.rules = vec![crate::state::QueryClassificationRuleInput {
        pattern: String::new(),
        category: "raw/%s".into(),
        priority_input: "1".into(),
    }];
    let _ = crate::query_classification::update(
        &mut state,
        crate::messages::SettingsMessage::QueryClassification(
            crate::messages::QueryClassificationMessage::PatternChanged(0, String::new()),
        ),
    );
    assert_eq!(
        state
            .save_error_translation
            .as_ref()
            .unwrap()
            .render(I18n::new(DesktopLocale::English))
            .unwrap(),
        "Classification rule 1 is missing a pattern"
    );
    assert_eq!(state.rules[0].category, "raw/%s");
}
