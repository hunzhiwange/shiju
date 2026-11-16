use desktop_i18n::I18n;
use config_ui::DesktopLocale;

#[test]
fn desktop_gettext_area_13_status_keeps_parameters_and_rejects_stale_metadata() {
    use desktop_i18n::format::Message;
    use crate::state::WorkflowState;
    let mut state = WorkflowState::default();
    state.set_status(Message::new("已加载 %s", vec!["用户/%s/%n/中文".into()]));
    assert_eq!(
        state.status_text(I18n::new(DesktopLocale::English)).unwrap(),
        "Loaded 用户/%s/%n/中文"
    );
    assert_eq!(
        state.status_text(I18n::new(DesktopLocale::TraditionalChinese)).unwrap(),
        "已載入 用户/%s/%n/中文"
    );
    state.status_message = Some("raw 已加载 %s".into());
    assert_eq!(state.status_text(I18n::new(DesktopLocale::English)).unwrap(), "raw 已加载 %s");
}
