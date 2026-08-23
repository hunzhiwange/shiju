// 验证预览搜索消息在当前路由层保持无副作用。
// 测试夹具围绕目标分支构造可控状态；跨模块行为由对应集成测试负责。

// 编译接线：引用父模块符号，确保测试片段仍由真实模块包含。
// 该冒烟用例只承担接线职责，具体行为由同领域测试覆盖。
#[test]
fn test_module_is_wired() {
    let module = module_path!();

    assert!(module.ends_with("search_tests"));
}

// 失败与边界：覆盖非法、缺失、过期或冲突场景的既定处理规则。
// 重点确认现有状态与错误反馈不会被意外的部分更新污染。
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
