#[test]
fn test_module_is_wired() {
    let module = module_path!();

    assert!(module.ends_with("search_tests"));
}

#[test]
fn update_ignores_all_routed_search_messages_without_scheduling_work() {
    let messages = [
        super::PreviewMessage::SearchChanged("needle".to_string()),
        super::PreviewMessage::SearchNext,
        super::PreviewMessage::SearchPrev,
        super::PreviewMessage::GotoLineChanged("42".to_string()),
        super::PreviewMessage::GotoLineSubmit,
        super::PreviewMessage::SearchToggle,
        super::PreviewMessage::SearchEditorAction(iced::widget::text_editor::Action::SelectAll),
        super::PreviewMessage::CopySearchMatches,
    ];

    for message in messages {
        let mut app = crate::app::App::new().0;
        app.file_url_input = "kept".to_string();
        app.active_preview_path = Some("/tmp/a.txt".to_string());

        let task = super::search::update(&mut app, message);

        assert_eq!(task.units(), 0);
        assert_eq!(app.file_url_input, "kept");
        assert_eq!(app.active_preview_path.as_deref(), Some("/tmp/a.txt"));
    }
}
