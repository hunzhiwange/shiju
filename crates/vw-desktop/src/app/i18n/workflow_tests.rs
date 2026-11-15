use super::{I18n, workflow::Feedback};
use config_ui::DesktopLocale;

#[test]
fn desktop_gettext_area_13_state_error_is_localized_without_changing_identifiers() {
    let mut state = crate::apps::workflow::state::WorkflowState::default();
    let error = state.open_edit_environment_variable_editor("用户/%n").unwrap_err();
    state.set_error(error);
    assert_eq!(
        state.error_text(I18n::new(DesktopLocale::English)).unwrap(),
        "Environment variable does not exist"
    );
    state.set_error("环境变量不存在".to_string());
    assert_eq!(state.error_text(I18n::new(DesktopLocale::English)).unwrap(), "环境变量不存在");
}
