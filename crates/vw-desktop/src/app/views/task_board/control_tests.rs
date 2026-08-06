use iced::advanced::renderer::Headless;

#[test]
fn test_module_is_wired() {
    let module = std::hint::black_box(module_path!());

    assert!(module.ends_with("control_tests"));
}

fn test_app() -> crate::app::App {
    crate::app::App::new().0
}

fn task(
    id: &str,
    status: crate::app::task::TaskStatus,
    deleted: bool,
    archived: bool,
) -> crate::app::task::Task {
    let mut task = crate::app::task::Task::new(10);
    task.id = id.to_string();
    task.status = status;
    task.deleted = deleted;
    task.archived = archived;
    task
}

fn slot(
    id: &str,
    state: crate::app::task::WorktreeState,
    leased_task_id: Option<&str>,
    taint_reason: Option<&str>,
) -> crate::app::task::WorktreeSlotSnapshot {
    crate::app::task::WorktreeSlotSnapshot {
        id: id.to_string(),
        path: format!("/tmp/{id}"),
        branch: format!("task/{id}"),
        state,
        leased_task_id: leased_task_id.map(str::to_string),
        taint_reason: taint_reason.map(str::to_string),
    }
}

fn snapshot() -> crate::app::task::WorktreePoolSnapshot {
    crate::app::task::WorktreePoolSnapshot {
        repo_root: "/repo".to_string(),
        pool_root: "/repo/.worktrees".to_string(),
        base_branch: "main".to_string(),
        idle_count: 1,
        busy_count: 1,
        tainted_count: 1,
        recycling_count: 1,
        dead_count: 1,
        merge_target_locks: vec![("main".to_string(), "task-1".to_string())],
        slots: vec![
            slot("idle", crate::app::task::WorktreeState::Idle, None, None),
            slot("busy", crate::app::task::WorktreeState::Busy, Some("task-1"), None),
            slot(
                "tainted",
                crate::app::task::WorktreeState::Tainted,
                Some("task-2"),
                Some("dirty tree"),
            ),
            slot("recycling", crate::app::task::WorktreeState::Recycling, None, None),
            slot("dead", crate::app::task::WorktreeState::Dead, None, Some("missing")),
        ],
    }
}

fn snapshot_without_locks() -> crate::app::task::WorktreePoolSnapshot {
    let mut snapshot = snapshot();
    snapshot.merge_target_locks.clear();
    snapshot.slots.clear();
    snapshot
}

fn assert_near(value: f32, expected: f32) {
    assert!((value - expected).abs() < 0.000_1);
}

async fn assert_element_draws(mut element: iced::Element<'_, crate::app::Message>) {
    let mut renderer = <iced::Renderer as Headless>::new(
        iced::Font::DEFAULT,
        iced::Pixels(16.0),
        Some("tiny-skia"),
    )
    .await
    .expect("tiny-skia headless renderer should be available");
    let mut tree = iced::advanced::widget::Tree::new(element.as_widget());
    let viewport = iced::Rectangle::with_size(iced::Size::new(640.0, 480.0));
    let node = element.as_widget_mut().layout(
        &mut tree,
        &renderer,
        &iced::advanced::layout::Limits::new(iced::Size::ZERO, viewport.size()),
    );

    element.as_widget().draw(
        &tree,
        &mut renderer,
        &iced::Theme::Dark,
        &iced::advanced::renderer::Style::default(),
        iced::advanced::Layout::new(&node),
        iced::mouse::Cursor::Unavailable,
        &viewport,
    );

    let pixels = renderer.screenshot(iced::Size::new(640, 480), 1.0, iced::Color::TRANSPARENT);
    assert!(pixels.chunks_exact(4).any(|pixel| pixel[3] > 0));
}

#[test]
fn build_control_buttons_covers_default_board_without_worktree_panel() {
    let app = test_app();

    let element = super::build_control_buttons(&app);

    std::hint::black_box(element);
}

#[test]
fn build_control_buttons_covers_active_scheduler_and_completed_tasks() {
    let mut app = test_app();
    app.task_board_executor_running = true;
    app.task_board_view_mode = crate::app::state::TaskBoardViewMode::Staff;
    app.task_board_worktree_manual_action_kind = Some("cleanup");
    app.task_board_tasks = vec![
        task("completed", crate::app::task::TaskStatus::Completed, false, false),
        task("deleted", crate::app::task::TaskStatus::Completed, true, false),
        task("archived", crate::app::task::TaskStatus::Completed, false, true),
        task("running", crate::app::task::TaskStatus::Running, false, false),
    ];

    let element = super::build_control_buttons(&app);

    std::hint::black_box(element);
}

#[test]
fn build_control_buttons_covers_loading_without_snapshot() {
    let mut app = test_app();
    app.project_path = Some("/repo".to_string());
    app.project_is_git_repo = true;
    app.task_board_worktree_snapshot_loading = true;

    let element = super::build_control_buttons(&app);

    std::hint::black_box(element);
}

#[test]
fn non_git_project_hides_worktree_controls() {
    fn tree_node_count(tree: &iced::advanced::widget::Tree) -> usize {
        1 + tree.children.iter().map(tree_node_count).sum::<usize>()
    }

    let mut non_git_app = test_app();
    non_git_app.project_path = Some("/repo".to_string());
    non_git_app.project_is_git_repo = false;
    non_git_app.task_board_worktree_snapshot = Some(snapshot());
    non_git_app.task_board_worktree_panel_expanded = true;
    non_git_app.task_board_worktree_snapshot_loading = true;
    non_git_app.task_board_worktree_pixel_office = true;
    assert!(!super::should_render_worktree_controls(&non_git_app));
    let non_git_nodes = {
        let element = super::build_control_buttons(&non_git_app);
        tree_node_count(&iced::advanced::widget::Tree::new(element.as_widget()))
    };

    let mut git_app = test_app();
    git_app.project_path = Some("/repo".to_string());
    git_app.project_is_git_repo = true;
    git_app.task_board_worktree_snapshot = Some(snapshot());
    git_app.task_board_worktree_panel_expanded = true;
    git_app.task_board_worktree_snapshot_loading = true;
    git_app.task_board_worktree_pixel_office = true;
    assert!(super::should_render_worktree_controls(&git_app));
    let git_nodes = {
        let element = super::build_control_buttons(&git_app);
        tree_node_count(&iced::advanced::widget::Tree::new(element.as_widget()))
    };

    assert!(
        git_nodes > non_git_nodes,
        "git project should render worktree panel widgets that non-git hides: git={git_nodes} non_git={non_git_nodes}"
    );
}

#[test]
fn build_control_buttons_covers_expanded_worktree_snapshot() {
    let mut app = test_app();
    app.project_is_git_repo = true;
    app.task_board_worktree_snapshot = Some(snapshot());
    app.task_board_worktree_panel_expanded = true;
    app.task_board_worktree_snapshot_loading = true;
    app.task_board_worktree_manual_confirm_kind = Some("cleanup");
    app.task_board_worktree_action_logs = vec!["completed one cleanup step".to_string()];

    let element = super::build_control_buttons(&app);

    std::hint::black_box(element);
}

#[test]
fn build_control_buttons_covers_collapsed_worktree_snapshot_without_locks_or_activity() {
    let mut app = test_app();
    app.project_is_git_repo = true;
    app.task_board_worktree_snapshot = Some(snapshot_without_locks());
    app.task_board_worktree_panel_expanded = false;
    app.task_board_worktree_snapshot_loading = false;
    app.task_board_worktree_pixel_office = false;

    let element = super::build_control_buttons(&app);

    std::hint::black_box(element);
}

#[tokio::test(flavor = "multi_thread")]
async fn build_control_buttons_draws_confirming_worktree_panel() {
    let mut app = test_app();
    app.project_is_git_repo = true;
    app.task_board_worktree_snapshot = Some(snapshot());
    app.task_board_worktree_panel_expanded = true;
    app.task_board_worktree_snapshot_loading = true;
    app.task_board_worktree_manual_confirm_kind = Some("merge");
    app.task_board_worktree_action_logs = vec!["merge awaiting confirmation".to_string()];

    assert_element_draws(super::build_control_buttons(&app)).await;
}

#[test]
fn build_control_buttons_covers_running_worktree_actions_and_pixel_office() {
    let mut cleanup_app = test_app();
    cleanup_app.project_is_git_repo = true;
    cleanup_app.task_board_worktree_snapshot = Some(snapshot_without_locks());
    cleanup_app.task_board_worktree_manual_action_kind = Some("cleanup");

    let cleanup_element = super::build_control_buttons(&cleanup_app);
    std::hint::black_box(cleanup_element);

    let mut delete_app = test_app();
    delete_app.project_is_git_repo = true;
    delete_app.task_board_worktree_snapshot = Some(snapshot_without_locks());
    delete_app.task_board_worktree_manual_action_kind = Some("delete");
    delete_app.task_board_worktree_manual_confirm_kind = Some("delete");

    let delete_element = super::build_control_buttons(&delete_app);
    std::hint::black_box(delete_element);

    let mut merge_app = test_app();
    merge_app.project_is_git_repo = true;
    merge_app.task_board_worktree_snapshot = Some(snapshot());
    merge_app.task_board_worktree_manual_action_kind = Some("merge");
    merge_app.task_board_worktree_manual_confirm_kind = Some("merge");
    merge_app.task_board_worktree_pixel_office = true;

    let merge_element = super::build_control_buttons(&merge_app);
    std::hint::black_box(merge_element);
}

#[test]
fn worktree_text_helpers_cover_confirm_and_action_variants() {
    assert!(super::worktree_confirm_hint("cleanup").contains("一键清理"));
    assert!(super::worktree_confirm_hint("delete").contains("删除所有 worktree"));
    assert!(super::worktree_confirm_hint("merge").contains("一键合并"));
    assert!(super::worktree_confirm_hint("other").contains("按 Esc"));

    assert!(super::worktree_action_status(Some("cleanup"), 0).contains("正在清理"));
    assert!(super::worktree_action_status(Some("delete"), 0).contains("正在删除"));
    assert!(super::worktree_action_status(Some("merge"), 0).contains("正在一键合并"));
    assert!(super::worktree_action_status(Some("slot_recycle"), 0).contains("正在回收"));
    assert_eq!(super::worktree_action_status(Some("other"), 0), "正在处理 worktree 操作");
    assert_eq!(super::worktree_action_status(None, 0), "正在处理 worktree 操作");
}

#[test]
fn style_helpers_build_expected_colors_and_borders() {
    let theme = iced::Theme::Dark;
    let tinted = super::status_chip_style(&theme, iced::Color::from_rgb8(0x2E, 0xB8, 0x72));
    let neutral = super::status_chip_style(&theme, iced::Color::from_rgba8(0, 0, 0, 0.0));

    assert!(matches!(tinted.background, Some(iced::Background::Color(_))));
    assert!(matches!(neutral.background, Some(iced::Background::Color(_))));
    assert_near(tinted.border.width, 1.0);
    assert_near(neutral.border.width, 1.0);
    assert!(tinted.text_color.expect("tinted chip text color").a > 0.99);
    assert!(neutral.text_color.expect("neutral chip text color").a < 1.0);

    let panel_styles = [
        super::top_controls_panel_style(&theme),
        super::control_outer_panel_style(&theme),
        super::confirm_panel_style(&theme),
        super::worktree_action_panel_style(&theme),
        super::worktree_slot_row_style(&theme),
        super::worktree_status_panel_style(&theme),
    ];
    for style in panel_styles {
        assert!(style.border.width.is_finite());
    }

    let filled_panel_styles = [
        super::confirm_panel_style(&theme),
        super::worktree_action_panel_style(&theme),
        super::worktree_slot_row_style(&theme),
        super::worktree_status_panel_style(&theme),
    ];
    for style in filled_panel_styles {
        assert!(matches!(style.background, Some(iced::Background::Color(_))));
    }

    let text_styles = [
        super::worktree_title_text_style(&theme),
        super::worktree_summary_text_style(&theme),
        super::worktree_refreshing_text_style(&theme),
        super::worktree_action_text_style(&theme),
        super::worktree_slot_title_text_style(&theme),
        super::worktree_slot_path_text_style(&theme),
        super::worktree_slot_taint_text_style(&theme),
        super::worktree_loading_text_style(&theme),
        super::confirm_hint_text_style(&theme),
    ];
    for style in text_styles {
        assert!(style.color.is_some());
    }
}
