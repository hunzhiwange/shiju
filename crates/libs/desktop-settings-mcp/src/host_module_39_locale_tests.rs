use desktop_i18n::I18n;
use config_ui::DesktopLocale;

#[test]
fn desktop_gettext_area_6_mcp_validation_keeps_semantic_error() {
    let mut state = crate::state::State::default();
    state.editor = iced::widget::text_editor::Content::with_text("{");
    let _ = crate::configuration_update::update(
        &mut state,
        crate::configuration_update::Message::Format,
    );
    let message = state.save_error_translation.as_ref().unwrap();
    assert!(
        message
            .render(I18n::new(DesktopLocale::English))
            .unwrap()
            .starts_with("JSON parse failed: ")
    );
    assert!(
        message
            .render(I18n::new(DesktopLocale::TraditionalChinese))
            .unwrap()
            .starts_with("JSON 解析失敗：")
    );
}
