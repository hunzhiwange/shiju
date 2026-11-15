fn find_tab(id: &str) -> desktop_file_tree::state::FindInFolderTab {
    desktop_file_tree::state::FindInFolderTab {
        id: id.to_string(),
        title: "查找".to_string(),
        scope_path: "/tmp/project".to_string(),
        query_input: String::new(),
        replace_input: String::new(),
        query_editor: iced::widget::text_editor::Content::new(),
        replace_editor: iced::widget::text_editor::Content::new(),
        query: String::new(),
        replace_text: String::new(),
        case_sensitive: false,
        whole_word: false,
        use_regex: false,
        running: false,
        error: None,
        limit_reached: false,
        matches: Vec::new(),
        localized_error: None,
        results_page: 0,
    }
}

#[test]
fn query_editor_action_syncs_input_and_clears_error() {
    let mut app = super::FindState::default();
    let mut tab = find_tab("a");
    tab.error = Some("old error".to_string());
    app.find_results_tabs.push(tab);

    let _ = super::handle(
        &mut app, None, Vec::new(),
        super::FindMessage::FileTreeFindQueryEditorAction(
            "a".to_string(),
            iced::widget::text_editor::Action::Edit(iced::widget::text_editor::Edit::Paste(
                std::sync::Arc::new("needle".to_string()),
            )),
        ),
    );

    let tab = &app.find_results_tabs[0];
    assert_eq!(tab.query_input, "needle");
    assert_eq!(tab.query_editor.text(), "needle");
    assert!(tab.error.is_none());
}

#[test]
fn replace_editor_action_syncs_input_for_matching_tab_only() {
    let mut app = super::FindState::default();
    app.find_results_tabs.push(find_tab("a"));
    app.find_results_tabs.push(find_tab("b"));

    let _ = super::handle(
        &mut app, None, Vec::new(),
        super::FindMessage::FileTreeFindReplaceEditorAction(
            "a".to_string(),
            iced::widget::text_editor::Action::Edit(iced::widget::text_editor::Edit::Paste(
                std::sync::Arc::new("replacement".to_string()),
            )),
        ),
    );

    assert_eq!(app.find_results_tabs[0].replace_input, "replacement");
    assert_eq!(app.find_results_tabs[0].replace_editor.text(), "replacement");
    assert!(app.find_results_tabs[1].replace_input.is_empty());
}

#[test]
fn editor_actions_for_missing_tab_leave_existing_tab_unchanged() {
    let mut app = super::FindState::default();
    let mut tab = find_tab("a");
    tab.query_input = "query".to_string();
    tab.replace_input = "replace".to_string();
    tab.error = Some("keep".to_string());
    app.find_results_tabs.push(tab);

    let _ = super::handle(
        &mut app, None, Vec::new(),
        super::FindMessage::FileTreeFindQueryEditorAction(
            "missing".to_string(),
            iced::widget::text_editor::Action::SelectAll,
        ),
    );
    let _ = super::handle(
        &mut app, None, Vec::new(),
        super::FindMessage::FileTreeFindReplaceEditorAction(
            "missing".to_string(),
            iced::widget::text_editor::Action::SelectAll,
        ),
    );

    let tab = &app.find_results_tabs[0];
    assert_eq!(tab.query_input, "query");
    assert_eq!(tab.replace_input, "replace");
    assert_eq!(tab.error.as_deref(), Some("keep"));
}

#[test]
fn option_toggles_update_matching_tab_only() {
    let mut app = super::FindState::default();
    app.find_results_tabs.push(find_tab("a"));
    app.find_results_tabs.push(find_tab("b"));

    let _ = super::handle(
        &mut app, None, Vec::new(),
        super::FindMessage::FileTreeFindCaseSensitiveToggled(
            "a".to_string(),
            true,
        ),
    );
    let _ = super::handle(
        &mut app, None, Vec::new(),
        super::FindMessage::FileTreeFindWholeWordToggled(
            "a".to_string(),
            true,
        ),
    );
    let _ = super::handle(
        &mut app, None, Vec::new(),
        super::FindMessage::FileTreeFindRegexToggled(
            "a".to_string(),
            true,
        ),
    );

    assert!(app.find_results_tabs[0].case_sensitive);
    assert!(app.find_results_tabs[0].whole_word);
    assert!(app.find_results_tabs[0].use_regex);
    assert!(!app.find_results_tabs[1].case_sensitive);
    assert!(!app.find_results_tabs[1].whole_word);
    assert!(!app.find_results_tabs[1].use_regex);
}

#[test]
fn option_toggles_for_missing_tab_leave_existing_options_unchanged() {
    let mut app = super::FindState::default();
    app.find_results_tabs.push(find_tab("a"));

    let _ = super::handle(
        &mut app, None, Vec::new(),
        super::FindMessage::FileTreeFindCaseSensitiveToggled(
            "missing".to_string(),
            true,
        ),
    );
    let _ = super::handle(
        &mut app, None, Vec::new(),
        super::FindMessage::FileTreeFindWholeWordToggled(
            "missing".to_string(),
            true,
        ),
    );
    let _ = super::handle(
        &mut app, None, Vec::new(),
        super::FindMessage::FileTreeFindRegexToggled(
            "missing".to_string(),
            true,
        ),
    );

    let tab = &app.find_results_tabs[0];
    assert!(!tab.case_sensitive);
    assert!(!tab.whole_word);
    assert!(!tab.use_regex);
}

#[test]
fn option_shortcuts_toggle_active_tab_only() {
    let mut app = super::FindState::default();
    app.find_results_tabs.push(find_tab("a"));
    app.find_results_tabs.push(find_tab("b"));
    app.active_find_results_tab_id = Some("a".to_string());

    for message in [
        super::FindMessage::FileTreeFindCaseSensitiveShortcut,
        super::FindMessage::FileTreeFindWholeWordShortcut,
        super::FindMessage::FileTreeFindRegexShortcut,
    ] {
        let _ = super::handle(&mut app, None, Vec::new(), message);
    }

    assert!(app.find_results_tabs[0].case_sensitive);
    assert!(app.find_results_tabs[0].whole_word);
    assert!(app.find_results_tabs[0].use_regex);
    assert!(!app.find_results_tabs[1].case_sensitive);
    assert!(!app.find_results_tabs[1].whole_word);
    assert!(!app.find_results_tabs[1].use_regex);
}

#[test]
fn option_shortcuts_without_active_tab_leave_options_unchanged() {
    let mut app = super::FindState::default();
    app.find_results_tabs.push(find_tab("a"));

    for message in [
        super::FindMessage::FileTreeFindCaseSensitiveShortcut,
        super::FindMessage::FileTreeFindWholeWordShortcut,
        super::FindMessage::FileTreeFindRegexShortcut,
    ] {
        let _ = super::handle(&mut app, None, Vec::new(), message);
    }

    let tab = &app.find_results_tabs[0];
    assert!(!tab.case_sensitive);
    assert!(!tab.whole_word);
    assert!(!tab.use_regex);
}

#[test]
fn results_page_change_clamps_to_available_matches() {
    let mut app = super::FindState::default();
    let mut tab = find_tab("a");
    tab.matches = (0..205).map(|index| find_match(&format!("match {index}"))).collect();
    app.find_results_tabs.push(tab);

    let _ = super::handle(
        &mut app, None, Vec::new(),
        super::FindMessage::FileTreeFindResultsPageChanged(
            "a".to_string(),
            usize::MAX,
        ),
    );

    assert_eq!(app.find_results_tabs[0].results_page, 8);
}

#[test]
fn find_run_with_empty_query_sets_error_without_running() {
    let mut app = super::FindState::default();
    let mut tab = find_tab("a");
    tab.query_input = "  \n\t ".to_string();
    app.find_results_tabs.push(tab);

    let _ = super::handle(
        &mut app, None, Vec::new(),
        super::FindMessage::FileTreeFindRun("a".to_string()),
    );

    let tab = &app.find_results_tabs[0];
    assert_eq!(tab.error.as_deref(), Some("请输入查找关键字"));
    assert!(!tab.running);
}

#[test]
fn find_run_for_missing_tab_is_a_noop() {
    let mut app = super::FindState::default();
    app.find_results_tabs.push(find_tab("a"));

    let _ = super::handle(
        &mut app, None, Vec::new(),
        super::FindMessage::FileTreeFindRun("missing".to_string()),
    );

    assert!(!app.find_results_tabs[0].running);
    assert!(app.find_results_tabs[0].error.is_none());
}

#[test]
fn find_run_with_nonempty_query_marks_tab_running_and_clears_stale_state() {
    let mut app = super::FindState::default();
    let mut tab = find_tab("a");
    tab.query_input = "  needle  ".to_string();
    tab.error = Some("old error".to_string());
    tab.limit_reached = true;
    app.find_results_tabs.push(tab);

    let _ = super::handle(
        &mut app, None, Vec::new(),
        super::FindMessage::FileTreeFindRun("a".to_string()),
    );

    let tab = &app.find_results_tabs[0];
    assert!(tab.running);
    assert!(tab.error.is_none());
    assert!(!tab.limit_reached);
    assert_eq!(tab.query_input, "  needle  ");
}

#[test]
fn refresh_active_with_empty_query_sets_error() {
    let mut app = super::FindState::default();
    app.find_results_tabs.push(find_tab("a"));
    app.active_find_results_tab_id = Some("a".to_string());

    let _ = super::handle(
        &mut app, None, Vec::new(),
        super::FindMessage::FileTreeFindRefreshActive,
    );

    assert_eq!(app.find_results_tabs[0].error.as_deref(), Some("请输入查找关键字"));
}

// 失败与边界：覆盖非法、缺失、过期或冲突场景的既定处理规则。
// 重点确认现有状态与错误反馈不会被意外的部分更新污染。
#[test]
fn refresh_without_active_tab_is_a_noop() {
    let mut app = super::FindState::default();
    app.find_results_tabs.push(find_tab("a"));

    let _ = super::handle(
        &mut app, None, Vec::new(),
        super::FindMessage::FileTreeFindRefreshActive,
    );

    assert!(!app.find_results_tabs[0].running);
    assert!(app.find_results_tabs[0].error.is_none());
}

#[test]
fn refresh_with_missing_active_tab_is_a_noop() {
    let mut app = super::FindState::default();
    app.find_results_tabs.push(find_tab("a"));
    app.active_find_results_tab_id = Some("missing".to_string());

    let _ = super::handle(
        &mut app, None, Vec::new(),
        super::FindMessage::FileTreeFindRefreshActive,
    );

    assert!(!app.find_results_tabs[0].running);
    assert!(app.find_results_tabs[0].error.is_none());
}

#[test]
fn refresh_with_nonempty_query_marks_active_tab_running() {
    let mut app = super::FindState::default();
    let mut tab = find_tab("a");
    tab.query_input = "needle".to_string();
    tab.error = Some("old error".to_string());
    tab.limit_reached = true;
    app.find_results_tabs.push(tab);
    app.active_find_results_tab_id = Some("a".to_string());

    let _ = super::handle(
        &mut app, None, Vec::new(),
        super::FindMessage::FileTreeFindRefreshActive,
    );

    let tab = &app.find_results_tabs[0];
    assert!(tab.running);
    assert!(tab.error.is_none());
    assert!(!tab.limit_reached);
}
