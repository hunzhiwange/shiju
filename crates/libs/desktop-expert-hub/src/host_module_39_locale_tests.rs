use desktop_i18n::I18n;
use config_ui::DesktopLocale;

#[test]
fn desktop_gettext_area_18_knowledge_feedback_switches_after_creation() {
    let mut state = crate::state::KnowledgeUiState::default();
    let mut update = crate::KnowledgeUpdate {
        knowledge: &mut state,
        gateway: || Err("isolated".into()),
        dropped_paths: Vec::new(),
        feedback: None,
    };
    let _ = crate::message::knowledge_tool::update(
        &mut update,
        crate::message::knowledge_tool::KnowledgeToolMessage::CreateDataset,
    );
    let (_, _, message) = update.feedback.take().expect("validation feedback");
    drop(update);
    for (locale, expected) in [
        (DesktopLocale::SimplifiedChinese, "知识库名称不能为空"),
        (DesktopLocale::TraditionalChinese, "知識庫名稱不能為空"),
        (DesktopLocale::English, "Knowledge base name is required"),
    ] {
        assert_eq!(message.render(I18n::new(locale)).unwrap(), expected);
        assert_eq!(
            state.notification_feedback.as_ref().unwrap().render(I18n::new(locale)),
            expected
        );
    }
}
