#[test]
fn test_module_is_wired() {
    let module = module_path!();

    assert!(module.ends_with("find_tests"));
}

fn app() -> crate::app::App {
    crate::app::App::new().0
}

fn find_tab(id: &str) -> crate::app::state::FindInFolderTab {
    crate::app::state::FindInFolderTab {
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
        results_page: 0,
    }
}

fn find_match(preview: &str) -> crate::app::state::FindInFolderMatch {
    crate::app::state::FindInFolderMatch {
        path: "/tmp/project/a.rs".to_string(),
        line: 1,
        column: 2,
        preview: preview.to_string(),
        match_len: preview.len(),
    }
}

#[test]
fn query_editor_action_syncs_input_and_clears_error() {
    let mut app = app();
    let mut tab = find_tab("a");
    tab.error = Some("old error".to_string());
    app.find_results_tabs.push(tab);

    let _ = super::handle(
        &mut app,
        crate::app::message::project::ProjectMessage::FileTreeFindQueryEditorAction(
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
    let mut app = app();
    app.find_results_tabs.push(find_tab("a"));
    app.find_results_tabs.push(find_tab("b"));

    let _ = super::handle(
        &mut app,
        crate::app::message::project::ProjectMessage::FileTreeFindReplaceEditorAction(
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
    let mut app = app();
    let mut tab = find_tab("a");
    tab.query_input = "query".to_string();
    tab.replace_input = "replace".to_string();
    tab.error = Some("keep".to_string());
    app.find_results_tabs.push(tab);

    let _ = super::handle(
        &mut app,
        crate::app::message::project::ProjectMessage::FileTreeFindQueryEditorAction(
            "missing".to_string(),
            iced::widget::text_editor::Action::SelectAll,
        ),
    );
    let _ = super::handle(
        &mut app,
        crate::app::message::project::ProjectMessage::FileTreeFindReplaceEditorAction(
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
    let mut app = app();
    app.find_results_tabs.push(find_tab("a"));
    app.find_results_tabs.push(find_tab("b"));

    let _ = super::handle(
        &mut app,
        crate::app::message::project::ProjectMessage::FileTreeFindCaseSensitiveToggled(
            "a".to_string(),
            true,
        ),
    );
    let _ = super::handle(
        &mut app,
        crate::app::message::project::ProjectMessage::FileTreeFindWholeWordToggled(
            "a".to_string(),
            true,
        ),
    );
    let _ = super::handle(
        &mut app,
        crate::app::message::project::ProjectMessage::FileTreeFindRegexToggled(
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
    let mut app = app();
    app.find_results_tabs.push(find_tab("a"));

    let _ = super::handle(
        &mut app,
        crate::app::message::project::ProjectMessage::FileTreeFindCaseSensitiveToggled(
            "missing".to_string(),
            true,
        ),
    );
    let _ = super::handle(
        &mut app,
        crate::app::message::project::ProjectMessage::FileTreeFindWholeWordToggled(
            "missing".to_string(),
            true,
        ),
    );
    let _ = super::handle(
        &mut app,
        crate::app::message::project::ProjectMessage::FileTreeFindRegexToggled(
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
    let mut app = app();
    app.find_results_tabs.push(find_tab("a"));
    app.find_results_tabs.push(find_tab("b"));
    app.active_find_results_tab_id = Some("a".to_string());

    for message in [
        crate::app::message::project::ProjectMessage::FileTreeFindCaseSensitiveShortcut,
        crate::app::message::project::ProjectMessage::FileTreeFindWholeWordShortcut,
        crate::app::message::project::ProjectMessage::FileTreeFindRegexShortcut,
    ] {
        let _ = super::handle(&mut app, message);
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
    let mut app = app();
    app.find_results_tabs.push(find_tab("a"));

    for message in [
        crate::app::message::project::ProjectMessage::FileTreeFindCaseSensitiveShortcut,
        crate::app::message::project::ProjectMessage::FileTreeFindWholeWordShortcut,
        crate::app::message::project::ProjectMessage::FileTreeFindRegexShortcut,
    ] {
        let _ = super::handle(&mut app, message);
    }

    let tab = &app.find_results_tabs[0];
    assert!(!tab.case_sensitive);
    assert!(!tab.whole_word);
    assert!(!tab.use_regex);
}

#[test]
fn results_page_change_clamps_to_available_matches() {
    let mut app = app();
    let mut tab = find_tab("a");
    tab.matches = (0..205).map(|index| find_match(&format!("match {index}"))).collect();
    app.find_results_tabs.push(tab);

    let _ = super::handle(
        &mut app,
        crate::app::message::project::ProjectMessage::FileTreeFindResultsPageChanged(
            "a".to_string(),
            usize::MAX,
        ),
    );

    assert_eq!(app.find_results_tabs[0].results_page, 8);
}

#[test]
fn find_run_with_empty_query_sets_error_without_running() {
    let mut app = app();
    let mut tab = find_tab("a");
    tab.query_input = "  \n\t ".to_string();
    app.find_results_tabs.push(tab);

    let _ = super::handle(
        &mut app,
        crate::app::message::project::ProjectMessage::FileTreeFindRun("a".to_string()),
    );

    let tab = &app.find_results_tabs[0];
    assert_eq!(tab.error.as_deref(), Some("请输入查找关键字"));
    assert!(!tab.running);
}

#[test]
fn find_run_for_missing_tab_is_a_noop() {
    let mut app = app();
    app.find_results_tabs.push(find_tab("a"));

    let _ = super::handle(
        &mut app,
        crate::app::message::project::ProjectMessage::FileTreeFindRun("missing".to_string()),
    );

    assert!(!app.find_results_tabs[0].running);
    assert!(app.find_results_tabs[0].error.is_none());
}

#[test]
fn find_run_with_nonempty_query_marks_tab_running_and_clears_stale_state() {
    let mut app = app();
    let mut tab = find_tab("a");
    tab.query_input = "  needle  ".to_string();
    tab.error = Some("old error".to_string());
    tab.limit_reached = true;
    app.find_results_tabs.push(tab);

    let _ = super::handle(
        &mut app,
        crate::app::message::project::ProjectMessage::FileTreeFindRun("a".to_string()),
    );

    let tab = &app.find_results_tabs[0];
    assert!(tab.running);
    assert!(tab.error.is_none());
    assert!(!tab.limit_reached);
    assert_eq!(tab.query_input, "  needle  ");
}

#[test]
fn refresh_active_with_empty_query_sets_error() {
    let mut app = app();
    app.find_results_tabs.push(find_tab("a"));
    app.active_find_results_tab_id = Some("a".to_string());

    let _ = super::handle(
        &mut app,
        crate::app::message::project::ProjectMessage::FileTreeFindRefreshActive,
    );

    assert_eq!(app.find_results_tabs[0].error.as_deref(), Some("请输入查找关键字"));
}

#[test]
fn refresh_without_active_tab_is_a_noop() {
    let mut app = app();
    app.find_results_tabs.push(find_tab("a"));

    let _ = super::handle(
        &mut app,
        crate::app::message::project::ProjectMessage::FileTreeFindRefreshActive,
    );

    assert!(!app.find_results_tabs[0].running);
    assert!(app.find_results_tabs[0].error.is_none());
}

#[test]
fn refresh_with_missing_active_tab_is_a_noop() {
    let mut app = app();
    app.find_results_tabs.push(find_tab("a"));
    app.active_find_results_tab_id = Some("missing".to_string());

    let _ = super::handle(
        &mut app,
        crate::app::message::project::ProjectMessage::FileTreeFindRefreshActive,
    );

    assert!(!app.find_results_tabs[0].running);
    assert!(app.find_results_tabs[0].error.is_none());
}

#[test]
fn refresh_with_nonempty_query_marks_active_tab_running() {
    let mut app = app();
    let mut tab = find_tab("a");
    tab.query_input = "needle".to_string();
    tab.error = Some("old error".to_string());
    tab.limit_reached = true;
    app.find_results_tabs.push(tab);
    app.active_find_results_tab_id = Some("a".to_string());

    let _ = super::handle(
        &mut app,
        crate::app::message::project::ProjectMessage::FileTreeFindRefreshActive,
    );

    let tab = &app.find_results_tabs[0];
    assert!(tab.running);
    assert!(tab.error.is_none());
    assert!(!tab.limit_reached);
}

#[test]
fn find_in_project_without_project_path_does_not_create_tabs() {
    let mut app = app();
    app.project_path = None;

    let _ = super::handle(
        &mut app,
        crate::app::message::project::ProjectMessage::FileTreeFindInProject,
    );

    assert!(app.find_results_tabs.is_empty());
    assert!(app.active_find_results_tab_id.is_none());
}

#[test]
fn find_in_project_creates_active_find_tab_and_open_tab() {
    let mut app = app();
    app.project_path = Some("/tmp/project".to_string());

    let _ = super::handle(
        &mut app,
        crate::app::message::project::ProjectMessage::FileTreeFindInProject,
    );

    assert_eq!(app.find_results_tabs.len(), 1);
    assert_eq!(app.find_results_tabs[0].scope_path, "/tmp/project");
    assert_eq!(app.active_find_results_tab_id, Some(app.find_results_tabs[0].id.clone()));
    assert!(app.show_file_manager);
    assert!(app.active_tab_id.as_deref().is_some_and(|id| id.starts_with("find:")));
    assert!(matches!(app.screen, crate::app::Screen::Project));
}

#[test]
fn find_scope_changed_resolves_project_relative_input_and_rejects_escape() {
    let mut app = app();
    app.project_path = Some("/tmp/project".to_string());
    app.find_results_tabs.push(find_tab("a"));

    let _ = super::handle(
        &mut app,
        crate::app::message::project::ProjectMessage::FileTreeFindScopeChanged(
            "a".to_string(),
            "src/nested".to_string(),
        ),
    );
    assert_eq!(app.find_results_tabs[0].scope_path, "/tmp/project/src/nested");
    assert!(app.find_results_tabs[0].error.is_none());

    let _ = super::handle(
        &mut app,
        crate::app::message::project::ProjectMessage::FileTreeFindScopeChanged(
            "a".to_string(),
            String::new(),
        ),
    );
    assert_eq!(app.find_results_tabs[0].scope_path, "/tmp/project");

    for invalid in ["/tmp/outside", "../outside", "src/../../outside"] {
        let _ = super::handle(
            &mut app,
            crate::app::message::project::ProjectMessage::FileTreeFindScopeChanged(
                "a".to_string(),
                invalid.to_string(),
            ),
        );
        assert_eq!(app.find_results_tabs[0].scope_path, "/tmp/project");
        assert_eq!(app.find_results_tabs[0].error.as_deref(), Some("作用域必须是项目内的相对路径"));
    }
}

#[test]
fn find_completed_updates_existing_tab_without_replacing_matches_on_error() {
    let mut app = app();
    let mut tab = find_tab("a");
    tab.matches.push(crate::app::state::FindInFolderMatch {
        path: "/tmp/project/a.rs".to_string(),
        line: 1,
        column: 2,
        preview: "old".to_string(),
        match_len: 3,
    });
    app.find_results_tabs.push(tab);

    let _ = super::handle(
        &mut app,
        crate::app::message::project::ProjectMessage::FileTreeFindCompleted {
            tab_id: "a".to_string(),
            title: "查找: main".to_string(),
            scope_path: "/tmp/project".to_string(),
            query: "main".to_string(),
            replace_text: "lib".to_string(),
            case_sensitive: true,
            whole_word: true,
            use_regex: false,
            matches: Vec::new(),
            error: Some("bad regex".to_string()),
            limit_reached: true,
        },
    );

    let tab = &app.find_results_tabs[0];
    assert_eq!(tab.title, "查找: main");
    assert_eq!(tab.query, "main");
    assert_eq!(tab.replace_text, "lib");
    assert!(tab.case_sensitive);
    assert!(tab.whole_word);
    assert!(!tab.running);
    assert_eq!(tab.error.as_deref(), Some("bad regex"));
    assert!(tab.limit_reached);
    assert_eq!(tab.matches.len(), 1);
}

#[test]
fn find_completed_creates_missing_tab_with_matches() {
    let mut app = app();
    let matches = vec![crate::app::state::FindInFolderMatch {
        path: "/tmp/project/a.rs".to_string(),
        line: 4,
        column: 5,
        preview: "fn main()".to_string(),
        match_len: 4,
    }];

    let _ = super::handle(
        &mut app,
        crate::app::message::project::ProjectMessage::FileTreeFindCompleted {
            tab_id: "new".to_string(),
            title: "查找: main".to_string(),
            scope_path: "/tmp/project".to_string(),
            query: "main".to_string(),
            replace_text: String::new(),
            case_sensitive: false,
            whole_word: false,
            use_regex: false,
            matches,
            error: None,
            limit_reached: false,
        },
    );

    assert_eq!(app.find_results_tabs.len(), 1);
    assert_eq!(app.find_results_tabs[0].query_input, "main");
    assert_eq!(app.find_results_tabs[0].matches.len(), 1);
    assert_eq!(app.active_find_results_tab_id.as_deref(), Some("new"));
}

#[test]
fn find_completed_success_replaces_existing_matches_and_clears_running() {
    let mut app = app();
    let mut tab = find_tab("a");
    tab.running = true;
    tab.error = Some("old error".to_string());
    tab.matches.push(find_match("old"));
    app.find_results_tabs.push(tab);

    let _ = super::handle(
        &mut app,
        crate::app::message::project::ProjectMessage::FileTreeFindCompleted {
            tab_id: "a".to_string(),
            title: "查找: new".to_string(),
            scope_path: "/tmp/project/src".to_string(),
            query: "new".to_string(),
            replace_text: "updated".to_string(),
            case_sensitive: false,
            whole_word: true,
            use_regex: true,
            matches: vec![find_match("new")],
            error: None,
            limit_reached: false,
        },
    );

    let tab = &app.find_results_tabs[0];
    assert_eq!(tab.scope_path, "/tmp/project/src");
    assert_eq!(tab.matches.len(), 1);
    assert_eq!(tab.matches[0].preview, "new");
    assert!(!tab.running);
    assert!(tab.error.is_none());
    assert!(tab.whole_word);
    assert!(tab.use_regex);
}

#[test]
fn find_completed_reuses_existing_open_tab_without_duplication() {
    let mut app = app();
    app.find_results_tabs.push(find_tab("a"));
    app.open_tabs.push(crate::app::AppTab {
        id: "find:a".to_string(),
        title: "Existing results".to_string(),
        screen: crate::app::Screen::Project,
        project_path: Some("/tmp/project".to_string()),
    });

    let _ = super::handle(
        &mut app,
        crate::app::message::project::ProjectMessage::FileTreeFindCompleted {
            tab_id: "a".to_string(),
            title: "查找: main".to_string(),
            scope_path: "/tmp/project".to_string(),
            query: "main".to_string(),
            replace_text: String::new(),
            case_sensitive: false,
            whole_word: false,
            use_regex: false,
            matches: vec![find_match("main")],
            error: None,
            limit_reached: false,
        },
    );

    assert_eq!(app.open_tabs.iter().filter(|tab| tab.id == "find:a").count(), 1);
    assert_eq!(
        app.open_tabs.iter().find(|tab| tab.id == "find:a").unwrap().title,
        "Existing results"
    );
    assert_eq!(app.active_tab_id.as_deref(), Some("find:a"));
    assert_eq!(app.active_find_results_tab_id.as_deref(), Some("a"));
}

#[test]
fn tab_selected_empty_clears_active_find_tab() {
    let mut app = app();
    app.active_find_results_tab_id = Some("a".to_string());

    let _ = super::handle(
        &mut app,
        crate::app::message::project::ProjectMessage::FileTreeFindTabSelected(String::new()),
    );

    assert!(app.active_find_results_tab_id.is_none());
}

#[test]
fn match_selected_prepares_zero_based_editor_target_and_opens_file() {
    let mut app = app();

    let task = super::handle(
        &mut app,
        crate::app::message::project::ProjectMessage::FileTreeFindMatchSelected {
            path: "/tmp/project/src/main.rs".to_string(),
            line: 8,
            column: 4,
        },
    );

    assert_eq!(app.pending_preview_goto, Some(("/tmp/project/src/main.rs".to_string(), 7, 3)));
    assert!(task.is_some_and(|task| task.units() > 0));
}

#[test]
fn match_selected_clamps_invalid_one_based_position_to_editor_origin() {
    let mut app = app();

    let _ = super::handle(
        &mut app,
        crate::app::message::project::ProjectMessage::FileTreeFindMatchSelected {
            path: "/tmp/project/src/main.rs".to_string(),
            line: 0,
            column: 0,
        },
    );

    assert_eq!(app.pending_preview_goto, Some(("/tmp/project/src/main.rs".to_string(), 0, 0)));
}

#[test]
fn tab_selected_existing_open_tab_keeps_the_active_workspace() {
    let mut app = app();
    let active_tab_id = app.active_tab_id.clone();
    app.open_tabs.push(crate::app::AppTab {
        id: "find:a".to_string(),
        title: "A".to_string(),
        screen: crate::app::Screen::Project,
        project_path: Some("/tmp/project".to_string()),
    });

    let _ = super::handle(
        &mut app,
        crate::app::message::project::ProjectMessage::FileTreeFindTabSelected("a".to_string()),
    );

    assert_eq!(app.active_find_results_tab_id.as_deref(), Some("a"));
    assert_eq!(app.active_tab_id, active_tab_id);
    assert!(app.show_file_manager);
}

#[test]
fn tab_selected_without_open_tab_preserves_active_open_tab() {
    let mut app = app();
    let active_tab_id = app.active_tab_id.clone();

    let _ = super::handle(
        &mut app,
        crate::app::message::project::ProjectMessage::FileTreeFindTabSelected(
            "missing".to_string(),
        ),
    );

    assert_eq!(app.active_find_results_tab_id.as_deref(), Some("missing"));
    assert_eq!(app.active_tab_id, active_tab_id);
    assert!(app.show_file_manager);
}

#[test]
fn tab_selected_while_search_is_running_keeps_the_active_find_tab() {
    let mut app = app();
    let mut running = find_tab("running");
    running.running = true;
    app.find_results_tabs.push(running);
    app.find_results_tabs.push(find_tab("other"));
    app.active_find_results_tab_id = Some("running".to_string());

    let _ = super::handle(
        &mut app,
        crate::app::message::project::ProjectMessage::FileTreeFindTabSelected("other".to_string()),
    );

    assert_eq!(app.active_find_results_tab_id.as_deref(), Some("running"));
}

#[test]
fn tab_closed_removes_find_and_open_tabs_and_selects_last() {
    let mut app = app();
    app.find_results_tabs.push(find_tab("a"));
    app.find_results_tabs.push(find_tab("b"));
    app.open_tabs.push(crate::app::AppTab {
        id: "find:a".to_string(),
        title: "A".to_string(),
        screen: crate::app::Screen::Project,
        project_path: Some("/tmp/project".to_string()),
    });
    app.open_tabs.push(crate::app::AppTab {
        id: "find:b".to_string(),
        title: "B".to_string(),
        screen: crate::app::Screen::Project,
        project_path: Some("/tmp/project".to_string()),
    });
    app.active_find_results_tab_id = Some("a".to_string());
    app.active_tab_id = Some("find:a".to_string());

    let _ = super::handle(
        &mut app,
        crate::app::message::project::ProjectMessage::FileTreeFindTabClosed("a".to_string()),
    );

    assert_eq!(
        app.find_results_tabs.iter().map(|tab| tab.id.as_str()).collect::<Vec<_>>(),
        vec!["b"]
    );
    assert_eq!(
        app.open_tabs.iter().map(|tab| tab.id.as_str()).collect::<Vec<_>>(),
        vec!["home", "find:b"]
    );
    assert_eq!(app.active_find_results_tab_id.as_deref(), Some("b"));
    assert_eq!(app.active_tab_id.as_deref(), Some("find:b"));
}

#[test]
fn tab_closed_nonactive_tab_preserves_active_selections() {
    let mut app = app();
    app.find_results_tabs.push(find_tab("a"));
    app.find_results_tabs.push(find_tab("b"));
    app.open_tabs.push(crate::app::AppTab {
        id: "find:a".to_string(),
        title: "A".to_string(),
        screen: crate::app::Screen::Project,
        project_path: Some("/tmp/project".to_string()),
    });
    app.open_tabs.push(crate::app::AppTab {
        id: "find:b".to_string(),
        title: "B".to_string(),
        screen: crate::app::Screen::Project,
        project_path: Some("/tmp/project".to_string()),
    });
    app.active_find_results_tab_id = Some("b".to_string());
    app.active_tab_id = Some("find:b".to_string());

    let _ = super::handle(
        &mut app,
        crate::app::message::project::ProjectMessage::FileTreeFindTabClosed("a".to_string()),
    );

    assert_eq!(app.active_find_results_tab_id.as_deref(), Some("b"));
    assert_eq!(app.active_tab_id.as_deref(), Some("find:b"));
    assert!(app.find_results_tabs.iter().all(|tab| tab.id != "a"));
    assert!(app.open_tabs.iter().all(|tab| tab.id != "find:a"));
}

#[test]
fn tab_closed_without_open_tab_selects_last_remaining_find_tab() {
    let mut app = app();
    app.find_results_tabs.push(find_tab("a"));
    app.find_results_tabs.push(find_tab("b"));
    app.active_find_results_tab_id = Some("a".to_string());
    let active_tab_id = app.active_tab_id.clone();

    let _ = super::handle(
        &mut app,
        crate::app::message::project::ProjectMessage::FileTreeFindTabClosed("a".to_string()),
    );

    assert_eq!(app.active_find_results_tab_id.as_deref(), Some("b"));
    assert_eq!(app.active_tab_id, active_tab_id);
    assert_eq!(app.find_results_tabs.len(), 1);
}

#[test]
fn tab_closed_last_active_find_tab_clears_active_find_and_falls_back_to_last_open_tab() {
    let mut app = app();
    app.find_results_tabs.push(find_tab("a"));
    app.open_tabs.push(crate::app::AppTab {
        id: "find:a".to_string(),
        title: "A".to_string(),
        screen: crate::app::Screen::Project,
        project_path: Some("/tmp/project".to_string()),
    });
    app.active_find_results_tab_id = Some("a".to_string());
    app.active_tab_id = Some("find:a".to_string());

    let _ = super::handle(
        &mut app,
        crate::app::message::project::ProjectMessage::FileTreeFindTabClosed("a".to_string()),
    );

    assert!(app.find_results_tabs.is_empty());
    assert!(app.active_find_results_tab_id.is_none());
    assert_eq!(app.active_tab_id.as_deref(), Some("home"));
}

fn add_find_open_tab(app: &mut crate::app::App, id: &str) {
    app.open_tabs.push(crate::app::AppTab {
        id: format!("find:{id}"),
        title: id.to_string(),
        screen: crate::app::Screen::Project,
        project_path: Some("/tmp/project".to_string()),
    });
}

fn set_find_tabs(app: &mut crate::app::App) {
    for id in ["a", "b", "c"] {
        app.find_results_tabs.push(find_tab(id));
        add_find_open_tab(app, id);
    }
}

#[test]
fn find_tab_right_click_records_menu_target_and_close_dismisses_it() {
    let mut app = app();
    app.find_results_tabs.push(find_tab("a"));

    let _ = super::handle(
        &mut app,
        crate::app::message::project::ProjectMessage::FileTreeFindTabRightClicked(
            "a".to_string(),
            12.0,
            24.0,
        ),
    );

    assert_eq!(app.find_results_tab_menu_id.as_deref(), Some("a"));
    assert_eq!(app.find_results_tab_menu_pos, Some(iced::Point::new(12.0, 24.0)));

    let _ = super::handle(
        &mut app,
        crate::app::message::project::ProjectMessage::FileTreeFindTabMenuClose,
    );

    assert!(app.find_results_tab_menu_id.is_none());
    assert!(app.find_results_tab_menu_pos.is_none());
}

#[test]
fn find_tab_menu_close_current_syncs_active_find_and_top_level_tabs() {
    let mut app = app();
    set_find_tabs(&mut app);
    app.active_find_results_tab_id = Some("b".to_string());
    app.active_tab_id = Some("find:b".to_string());
    app.find_results_tab_menu_id = Some("b".to_string());

    let _ = super::handle(
        &mut app,
        crate::app::message::project::ProjectMessage::FileTreeFindTabClosed("b".to_string()),
    );

    assert_eq!(
        app.find_results_tabs.iter().map(|tab| tab.id.as_str()).collect::<Vec<_>>(),
        vec!["a", "c"]
    );
    assert_eq!(app.active_find_results_tab_id.as_deref(), Some("c"));
    assert_eq!(app.active_tab_id.as_deref(), Some("find:c"));
    assert!(app.open_tabs.iter().all(|tab| tab.id != "find:b"));
    assert!(app.find_results_tab_menu_id.is_none());
}

#[test]
fn find_tab_menu_close_left_syncs_active_find_and_top_level_tabs() {
    let mut app = app();
    set_find_tabs(&mut app);
    app.active_find_results_tab_id = Some("a".to_string());
    app.active_tab_id = Some("find:a".to_string());
    app.find_results_tab_menu_id = Some("b".to_string());

    let _ = super::handle(
        &mut app,
        crate::app::message::project::ProjectMessage::FileTreeFindTabMenuCloseLeft("b".to_string()),
    );

    assert_eq!(
        app.find_results_tabs.iter().map(|tab| tab.id.as_str()).collect::<Vec<_>>(),
        vec!["b", "c"]
    );
    assert_eq!(app.active_find_results_tab_id.as_deref(), Some("c"));
    assert_eq!(app.active_tab_id.as_deref(), Some("find:c"));
    assert!(app.open_tabs.iter().all(|tab| tab.id != "find:a"));
    assert!(app.find_results_tab_menu_id.is_none());
}

#[test]
fn find_tab_menu_close_right_syncs_active_find_and_top_level_tabs() {
    let mut app = app();
    set_find_tabs(&mut app);
    app.active_find_results_tab_id = Some("c".to_string());
    app.active_tab_id = Some("find:c".to_string());
    app.find_results_tab_menu_id = Some("b".to_string());

    let _ = super::handle(
        &mut app,
        crate::app::message::project::ProjectMessage::FileTreeFindTabMenuCloseRight(
            "b".to_string(),
        ),
    );

    assert_eq!(
        app.find_results_tabs.iter().map(|tab| tab.id.as_str()).collect::<Vec<_>>(),
        vec!["a", "b"]
    );
    assert_eq!(app.active_find_results_tab_id.as_deref(), Some("b"));
    assert_eq!(app.active_tab_id.as_deref(), Some("find:b"));
    assert!(app.open_tabs.iter().all(|tab| tab.id != "find:c"));
    assert!(app.find_results_tab_menu_id.is_none());
}

#[test]
fn find_tab_menu_close_all_clears_all_find_state_and_top_level_tabs() {
    let mut app = app();
    set_find_tabs(&mut app);
    app.active_find_results_tab_id = Some("b".to_string());
    app.active_tab_id = Some("find:b".to_string());
    app.find_results_tab_menu_id = Some("b".to_string());

    let _ = super::handle(
        &mut app,
        crate::app::message::project::ProjectMessage::FileTreeFindTabMenuCloseAll,
    );

    assert!(app.find_results_tabs.is_empty());
    assert!(app.active_find_results_tab_id.is_none());
    assert_eq!(app.active_tab_id.as_deref(), Some("home"));
    assert!(app.open_tabs.iter().all(|tab| !tab.id.starts_with("find:")));
    assert!(app.find_results_tab_menu_id.is_none());
    assert!(app.find_results_tab_menu_pos.is_none());
}

#[test]
fn unrelated_project_message_is_not_handled() {
    let mut app = app();

    let task =
        super::handle(&mut app, crate::app::message::project::ProjectMessage::AddFilePressed);

    assert!(task.is_none());
}
