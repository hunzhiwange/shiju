// 宿主保留依据：find::handle 同步查找标签、顶层标签及编辑器定位，保留宿主消息映射契约。
// 验证项目内查找替换标签的输入同步与选项更新。
// 测试夹具围绕目标分支构造可控状态；跨模块行为由对应集成测试负责。

// 编译接线：引用父模块符号，确保测试片段仍由真实模块包含。
// 该冒烟用例只承担接线职责，具体行为由同领域测试覆盖。
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
        localized_error: None,
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
fn find_in_project_creates_active_find_tab_and_open_tab() {
    let mut app = app();
    app.navigation_tabs.project_path = Some("/tmp/project".to_string());

    let _ = super::handle(
        &mut app,
        crate::app::message::project::ProjectMessage::FileTreeFindInProject,
    );

    assert_eq!(app.project_find.find_results_tabs.len(), 1);
    assert_eq!(app.project_find.find_results_tabs[0].scope_path, "/tmp/project");
    assert_eq!(app.project_find.active_find_results_tab_id, Some(app.project_find.find_results_tabs[0].id.clone()));
    assert!(app.file_manager.show_file_manager);
    assert!(app.navigation_tabs.active_tab_id.as_deref().is_some_and(|id| id.starts_with("find:")));
    assert!(matches!(app.navigation_tabs.screen, crate::app::Screen::Project));
}









#[test]
fn find_completed_reuses_existing_open_tab_without_duplication() {
    let mut app = app();
    app.project_find.find_results_tabs.push(find_tab("a"));
    app.navigation_tabs.open_tabs.push(crate::app::AppTab {
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
            localized_error: None,
            limit_reached: false,
        },
    );

    assert_eq!(app.navigation_tabs.open_tabs.iter().filter(|tab| tab.id == "find:a").count(), 1);
    assert_eq!(
        app.navigation_tabs.open_tabs.iter().find(|tab| tab.id == "find:a").unwrap().title,
        "Existing results"
    );
    assert_eq!(app.navigation_tabs.active_tab_id.as_deref(), Some("find:a"));
    assert_eq!(app.project_find.active_find_results_tab_id.as_deref(), Some("a"));
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

    assert_eq!(app.preview_state.pending_preview_goto, Some(("/tmp/project/src/main.rs".to_string(), 7, 3)));
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

    assert_eq!(app.preview_state.pending_preview_goto, Some(("/tmp/project/src/main.rs".to_string(), 0, 0)));
}

// 状态转换：覆盖交互前后的字段同步、选择关系与清理规则。
// 连续操作仍需保持模块不变量，并清除已经失效的瞬态反馈。
#[test]
fn tab_selected_existing_open_tab_keeps_the_active_workspace() {
    let mut app = app();
    let active_tab_id = app.navigation_tabs.active_tab_id.clone();
    app.navigation_tabs.open_tabs.push(crate::app::AppTab {
        id: "find:a".to_string(),
        title: "A".to_string(),
        screen: crate::app::Screen::Project,
        project_path: Some("/tmp/project".to_string()),
    });

    let _ = super::handle(
        &mut app,
        crate::app::message::project::ProjectMessage::FileTreeFindTabSelected("a".to_string()),
    );

    assert_eq!(app.project_find.active_find_results_tab_id.as_deref(), Some("a"));
    assert_eq!(app.navigation_tabs.active_tab_id, active_tab_id);
    assert!(app.file_manager.show_file_manager);
}

#[test]
fn tab_selected_without_open_tab_preserves_active_open_tab() {
    let mut app = app();
    let active_tab_id = app.navigation_tabs.active_tab_id.clone();

    let _ = super::handle(
        &mut app,
        crate::app::message::project::ProjectMessage::FileTreeFindTabSelected(
            "missing".to_string(),
        ),
    );

    assert_eq!(app.project_find.active_find_results_tab_id.as_deref(), Some("missing"));
    assert_eq!(app.navigation_tabs.active_tab_id, active_tab_id);
    assert!(app.file_manager.show_file_manager);
}



#[test]
fn tab_closed_removes_find_and_open_tabs_and_selects_last() {
    let mut app = app();
    app.project_find.find_results_tabs.push(find_tab("a"));
    app.project_find.find_results_tabs.push(find_tab("b"));
    app.navigation_tabs.open_tabs.push(crate::app::AppTab {
        id: "find:a".to_string(),
        title: "A".to_string(),
        screen: crate::app::Screen::Project,
        project_path: Some("/tmp/project".to_string()),
    });
    app.navigation_tabs.open_tabs.push(crate::app::AppTab {
        id: "find:b".to_string(),
        title: "B".to_string(),
        screen: crate::app::Screen::Project,
        project_path: Some("/tmp/project".to_string()),
    });
    app.project_find.active_find_results_tab_id = Some("a".to_string());
    app.navigation_tabs.active_tab_id = Some("find:a".to_string());

    let _ = super::handle(
        &mut app,
        crate::app::message::project::ProjectMessage::FileTreeFindTabClosed("a".to_string()),
    );

    assert_eq!(
        app.project_find.find_results_tabs.iter().map(|tab| tab.id.as_str()).collect::<Vec<_>>(),
        vec!["b"]
    );
    assert_eq!(
        app.navigation_tabs.open_tabs.iter().map(|tab| tab.id.as_str()).collect::<Vec<_>>(),
        vec!["home", "find:b"]
    );
    assert_eq!(app.project_find.active_find_results_tab_id.as_deref(), Some("b"));
    assert_eq!(app.navigation_tabs.active_tab_id.as_deref(), Some("find:b"));
}

#[test]
fn tab_closed_nonactive_tab_preserves_active_selections() {
    let mut app = app();
    app.project_find.find_results_tabs.push(find_tab("a"));
    app.project_find.find_results_tabs.push(find_tab("b"));
    app.navigation_tabs.open_tabs.push(crate::app::AppTab {
        id: "find:a".to_string(),
        title: "A".to_string(),
        screen: crate::app::Screen::Project,
        project_path: Some("/tmp/project".to_string()),
    });
    app.navigation_tabs.open_tabs.push(crate::app::AppTab {
        id: "find:b".to_string(),
        title: "B".to_string(),
        screen: crate::app::Screen::Project,
        project_path: Some("/tmp/project".to_string()),
    });
    app.project_find.active_find_results_tab_id = Some("b".to_string());
    app.navigation_tabs.active_tab_id = Some("find:b".to_string());

    let _ = super::handle(
        &mut app,
        crate::app::message::project::ProjectMessage::FileTreeFindTabClosed("a".to_string()),
    );

    assert_eq!(app.project_find.active_find_results_tab_id.as_deref(), Some("b"));
    assert_eq!(app.navigation_tabs.active_tab_id.as_deref(), Some("find:b"));
    assert!(app.project_find.find_results_tabs.iter().all(|tab| tab.id != "a"));
    assert!(app.navigation_tabs.open_tabs.iter().all(|tab| tab.id != "find:a"));
}

#[test]
fn tab_closed_without_open_tab_selects_last_remaining_find_tab() {
    let mut app = app();
    app.project_find.find_results_tabs.push(find_tab("a"));
    app.project_find.find_results_tabs.push(find_tab("b"));
    app.project_find.active_find_results_tab_id = Some("a".to_string());
    let active_tab_id = app.navigation_tabs.active_tab_id.clone();

    let _ = super::handle(
        &mut app,
        crate::app::message::project::ProjectMessage::FileTreeFindTabClosed("a".to_string()),
    );

    assert_eq!(app.project_find.active_find_results_tab_id.as_deref(), Some("b"));
    assert_eq!(app.navigation_tabs.active_tab_id, active_tab_id);
    assert_eq!(app.project_find.find_results_tabs.len(), 1);
}

#[test]
fn tab_closed_last_active_find_tab_clears_active_find_and_falls_back_to_last_open_tab() {
    let mut app = app();
    app.project_find.find_results_tabs.push(find_tab("a"));
    app.navigation_tabs.open_tabs.push(crate::app::AppTab {
        id: "find:a".to_string(),
        title: "A".to_string(),
        screen: crate::app::Screen::Project,
        project_path: Some("/tmp/project".to_string()),
    });
    app.project_find.active_find_results_tab_id = Some("a".to_string());
    app.navigation_tabs.active_tab_id = Some("find:a".to_string());

    let _ = super::handle(
        &mut app,
        crate::app::message::project::ProjectMessage::FileTreeFindTabClosed("a".to_string()),
    );

    assert!(app.project_find.find_results_tabs.is_empty());
    assert!(app.project_find.active_find_results_tab_id.is_none());
    assert_eq!(app.navigation_tabs.active_tab_id.as_deref(), Some("home"));
}

fn add_find_open_tab(app: &mut crate::app::App, id: &str) {
    app.navigation_tabs.open_tabs.push(crate::app::AppTab {
        id: format!("find:{id}"),
        title: id.to_string(),
        screen: crate::app::Screen::Project,
        project_path: Some("/tmp/project".to_string()),
    });
}

fn set_find_tabs(app: &mut crate::app::App) {
    for id in ["a", "b", "c"] {
        app.project_find.find_results_tabs.push(find_tab(id));
        add_find_open_tab(app, id);
    }
}



#[test]
fn find_tab_menu_close_current_syncs_active_find_and_top_level_tabs() {
    let mut app = app();
    set_find_tabs(&mut app);
    app.project_find.active_find_results_tab_id = Some("b".to_string());
    app.navigation_tabs.active_tab_id = Some("find:b".to_string());
    app.project_find.find_results_tab_menu_id = Some("b".to_string());

    let _ = super::handle(
        &mut app,
        crate::app::message::project::ProjectMessage::FileTreeFindTabClosed("b".to_string()),
    );

    assert_eq!(
        app.project_find.find_results_tabs.iter().map(|tab| tab.id.as_str()).collect::<Vec<_>>(),
        vec!["a", "c"]
    );
    assert_eq!(app.project_find.active_find_results_tab_id.as_deref(), Some("c"));
    assert_eq!(app.navigation_tabs.active_tab_id.as_deref(), Some("find:c"));
    assert!(app.navigation_tabs.open_tabs.iter().all(|tab| tab.id != "find:b"));
    assert!(app.project_find.find_results_tab_menu_id.is_none());
}

#[test]
fn find_tab_menu_close_left_syncs_active_find_and_top_level_tabs() {
    let mut app = app();
    set_find_tabs(&mut app);
    app.project_find.active_find_results_tab_id = Some("a".to_string());
    app.navigation_tabs.active_tab_id = Some("find:a".to_string());
    app.project_find.find_results_tab_menu_id = Some("b".to_string());

    let _ = super::handle(
        &mut app,
        crate::app::message::project::ProjectMessage::FileTreeFindTabMenuCloseLeft("b".to_string()),
    );

    assert_eq!(
        app.project_find.find_results_tabs.iter().map(|tab| tab.id.as_str()).collect::<Vec<_>>(),
        vec!["b", "c"]
    );
    assert_eq!(app.project_find.active_find_results_tab_id.as_deref(), Some("c"));
    assert_eq!(app.navigation_tabs.active_tab_id.as_deref(), Some("find:c"));
    assert!(app.navigation_tabs.open_tabs.iter().all(|tab| tab.id != "find:a"));
    assert!(app.project_find.find_results_tab_menu_id.is_none());
}

#[test]
fn find_tab_menu_close_right_syncs_active_find_and_top_level_tabs() {
    let mut app = app();
    set_find_tabs(&mut app);
    app.project_find.active_find_results_tab_id = Some("c".to_string());
    app.navigation_tabs.active_tab_id = Some("find:c".to_string());
    app.project_find.find_results_tab_menu_id = Some("b".to_string());

    let _ = super::handle(
        &mut app,
        crate::app::message::project::ProjectMessage::FileTreeFindTabMenuCloseRight(
            "b".to_string(),
        ),
    );

    assert_eq!(
        app.project_find.find_results_tabs.iter().map(|tab| tab.id.as_str()).collect::<Vec<_>>(),
        vec!["a", "b"]
    );
    assert_eq!(app.project_find.active_find_results_tab_id.as_deref(), Some("b"));
    assert_eq!(app.navigation_tabs.active_tab_id.as_deref(), Some("find:b"));
    assert!(app.navigation_tabs.open_tabs.iter().all(|tab| tab.id != "find:c"));
    assert!(app.project_find.find_results_tab_menu_id.is_none());
}

#[test]
fn find_tab_menu_close_all_clears_all_find_state_and_top_level_tabs() {
    let mut app = app();
    set_find_tabs(&mut app);
    app.project_find.active_find_results_tab_id = Some("b".to_string());
    app.navigation_tabs.active_tab_id = Some("find:b".to_string());
    app.project_find.find_results_tab_menu_id = Some("b".to_string());

    let _ = super::handle(
        &mut app,
        crate::app::message::project::ProjectMessage::FileTreeFindTabMenuCloseAll,
    );

    assert!(app.project_find.find_results_tabs.is_empty());
    assert!(app.project_find.active_find_results_tab_id.is_none());
    assert_eq!(app.navigation_tabs.active_tab_id.as_deref(), Some("home"));
    assert!(app.navigation_tabs.open_tabs.iter().all(|tab| !tab.id.starts_with("find:")));
    assert!(app.project_find.find_results_tab_menu_id.is_none());
    assert!(app.project_find.find_results_tab_menu_pos.is_none());
}

// 失败与边界：覆盖非法、缺失、过期或冲突场景的既定处理规则。
// 重点确认现有状态与错误反馈不会被意外的部分更新污染。
#[test]
fn unrelated_project_message_is_not_handled() {
    let mut app = app();

    let task =
        super::handle(&mut app, crate::app::message::project::ProjectMessage::AddFilePressed);

    assert!(task.is_none());
}
