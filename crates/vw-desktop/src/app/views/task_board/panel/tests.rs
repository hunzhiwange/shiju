//! 任务详情面板组合测试，覆盖选中任务、空状态和各折叠区块。
//! fixture 串联执行器、模型、日志与子任务控件，验证消息路由和布局。
use super::*;
use crate::app::state::DelegateAgentSettingsEntry;
use crate::app::task::{TaskStatus, WorktreePoolSnapshot};
use iced::widget::button;

fn test_app() -> App {
    App::new().0
}

fn keep_element(element: Element<'_, Message>) {
    std::hint::black_box(element);
}

fn snapshot_with_lock(holder: Option<&str>) -> WorktreePoolSnapshot {
    WorktreePoolSnapshot {
        repo_root: "/repo".to_string(),
        pool_root: "/repo/.worktrees".to_string(),
        base_branch: "main".to_string(),
        idle_count: 0,
        busy_count: 0,
        tainted_count: 0,
        recycling_count: 0,
        dead_count: 0,
        merge_target_locks: holder
            .map(|task_id| vec![("main".to_string(), task_id.to_string())])
            .unwrap_or_default(),
        slots: Vec::new(),
    }
}

#[test]
fn module_is_wired_and_scrollbar_contract_is_narrow() {
    assert!(module_path!().ends_with("panel::tests"));
    assert_eq!(TASK_PANEL_SCROLLBAR_WIDTH, 4.0);
    assert!(TASK_PANEL_SCROLLBAR_GUTTER >= TASK_PANEL_SCROLLBAR_WIDTH);
}

#[test]
fn agent_option_displays_its_user_facing_label() {
    let option =
        TaskBoardAgentOption { key: "reviewer".to_string(), label: "Reviewer".to_string() };

    assert_eq!(option.to_string(), "Reviewer");
}

#[test]
fn task_status_colors_cover_builtin_and_custom_statuses() {
    let cases = [
        (TaskStatus::Pool, (Color::from_rgb8(107, 114, 128), Color::from_rgb8(243, 244, 246))),
        (TaskStatus::Pending, (Color::from_rgb8(37, 99, 235), Color::from_rgb8(219, 234, 254))),
        (TaskStatus::Planning, (Color::from_rgb8(79, 70, 229), Color::from_rgb8(224, 231, 255))),
        (TaskStatus::Running, (Color::from_rgb8(147, 51, 234), Color::from_rgb8(243, 232, 255))),
        (TaskStatus::Failed, (Color::from_rgb8(220, 38, 38), Color::from_rgb8(254, 226, 226))),
        (TaskStatus::Paused, (Color::from_rgb8(202, 138, 4), Color::from_rgb8(254, 249, 195))),
        (
            TaskStatus::CodeComplete,
            (Color::from_rgb8(5, 150, 105), Color::from_rgb8(209, 250, 229)),
        ),
        (TaskStatus::CodeReview, (Color::from_rgb8(217, 119, 6), Color::from_rgb8(254, 243, 199))),
        (TaskStatus::PrSubmitted, (Color::from_rgb8(8, 145, 178), Color::from_rgb8(207, 250, 254))),
        (TaskStatus::Completed, (Color::from_rgb8(22, 163, 74), Color::from_rgb8(220, 252, 231))),
        (TaskStatus::Archived, (Color::from_rgb8(100, 116, 139), Color::from_rgb8(241, 245, 249))),
        (
            TaskStatus::Custom("review-ready"),
            (Color::from_rgb8(107, 114, 128), Color::from_rgb8(243, 244, 246)),
        ),
    ];

    for (status, expected) in cases {
        assert_eq!(task_status_tag_colors(status), expected, "status: {status:?}");
    }
}

#[test]
fn duration_and_running_dots_cover_zero_boundaries_and_cycle() {
    assert_eq!(format_duration_ms(0), "0s");
    assert_eq!(format_duration_ms(999), "0s");
    assert_eq!(format_duration_ms(61_000), "1m 1s");

    assert_eq!(running_dots(0), "·");
    assert_eq!(running_dots(999), "·");
    assert_eq!(running_dots(1_000), "··");
    assert_eq!(running_dots(2_000), "···");
    assert_eq!(running_dots(3_000), "·");
}

#[test]
fn segmented_button_style_distinguishes_theme_selection_and_interaction() {
    for theme in [Theme::Light, Theme::Dark] {
        let active = segmented_button_style(&theme, true, button::Status::Active);
        let inactive = segmented_button_style(&theme, false, button::Status::Disabled);
        let hovered = segmented_button_style(&theme, false, button::Status::Hovered);
        let pressed = segmented_button_style(&theme, true, button::Status::Pressed);

        assert_eq!(active.text_color, theme.palette().text);
        assert_eq!(inactive.text_color, theme.palette().text);
        assert_eq!(active.border.width, 1.0);
        assert_ne!(active.background, inactive.background);
        assert!(hovered.background.is_some());
        assert!(pressed.background.is_some());
    }
}

#[test]
fn agent_options_filter_disabled_entries_but_preserve_selection_and_main_fallback() {
    let mut app = test_app();
    app.agents_settings.entries.clear();

    let mut enabled = DelegateAgentSettingsEntry::from_config("enabled", None);
    enabled.label = "Enabled agent".to_string();
    enabled.enabled = true;
    let mut disabled = DelegateAgentSettingsEntry::from_config("disabled", None);
    disabled.label = "Disabled agent".to_string();
    disabled.enabled = false;
    app.agents_settings.entries = vec![enabled, disabled];

    let default_options = build_agent_options(&app, MAIN_AGENT_KEY);
    assert_eq!(default_options[0].key, MAIN_AGENT_KEY);
    assert!(default_options.iter().any(|option| option.key == "enabled"));
    assert!(!default_options.iter().any(|option| option.key == "disabled"));

    let selected_options = build_agent_options(&app, "disabled");
    assert!(
        selected_options
            .iter()
            .any(|option| { option.key == "disabled" && option.label == "Disabled agent" })
    );
}

#[test]
fn agent_options_exclude_retired_builtin_agents() {
    let mut app = test_app();
    app.agents_settings.entries = vec![
        DelegateAgentSettingsEntry::from_config(MAIN_AGENT_KEY, None),
        DelegateAgentSettingsEntry::from_config("build", None),
    ];

    let options = build_agent_options(&app, "build");

    assert!(options.iter().any(|option| option.key == MAIN_AGENT_KEY));
    assert!(!options.iter().any(|option| option.key == "build"));
}

#[test]
fn agent_options_insert_main_when_agent_registry_is_empty() {
    let mut app = test_app();
    app.agents_settings.entries.clear();

    let options = build_agent_options(&app, "missing-agent");

    assert_eq!(options.len(), 1);
    assert_eq!(options[0].key, MAIN_AGENT_KEY);
    assert_eq!(options[0].label, "Main");
}

#[test]
fn contextual_rows_reject_irrelevant_or_blank_task_metadata() {
    let app = test_app();
    let mut task = Task::default();

    assert!(
        build_failed_reason_row(
            crate::app::i18n::I18n::new(vw_config_types::ui::DesktopLocale::SimplifiedChinese),
            &task
        )
        .is_none()
    );
    task.status = TaskStatus::Failed;
    task.last_error = Some("   ".to_string());
    assert!(
        build_failed_reason_row(
            crate::app::i18n::I18n::new(vw_config_types::ui::DesktopLocale::SimplifiedChinese),
            &task
        )
        .is_none()
    );
    task.status = TaskStatus::Paused;
    task.pause_reason = Some("   ".to_string());
    assert!(
        build_failed_reason_row(
            crate::app::i18n::I18n::new(vw_config_types::ui::DesktopLocale::SimplifiedChinese),
            &task
        )
        .is_none()
    );

    assert!(
        build_selected_worktree_row(
            crate::app::i18n::I18n::new(vw_config_types::ui::DesktopLocale::SimplifiedChinese),
            &task
        )
        .is_none()
    );
    task.selected_worktree_path = Some("   ".to_string());
    assert!(
        build_selected_worktree_row(
            crate::app::i18n::I18n::new(vw_config_types::ui::DesktopLocale::SimplifiedChinese),
            &task
        )
        .is_none()
    );

    task.merge_target_branch = Some("main".to_string());
    assert!(build_merge_lock_row(&app, &task).is_none());
}

#[test]
fn contextual_rows_build_failure_pause_worktree_and_merge_lock_variants() {
    let mut app = test_app();
    app.project_path = Some("/repo".to_string());

    let mut task = Task { id: "task-1".to_string(), ..Task::default() };
    task.status = TaskStatus::Failed;
    assert!(
        build_failed_reason_row(
            crate::app::i18n::I18n::new(vw_config_types::ui::DesktopLocale::SimplifiedChinese),
            &task
        )
        .is_some()
    );
    task.last_error = Some("worker exited".to_string());
    assert!(
        build_failed_reason_row(
            crate::app::i18n::I18n::new(vw_config_types::ui::DesktopLocale::SimplifiedChinese),
            &task
        )
        .is_some()
    );

    task.status = TaskStatus::Paused;
    assert!(
        build_failed_reason_row(
            crate::app::i18n::I18n::new(vw_config_types::ui::DesktopLocale::SimplifiedChinese),
            &task
        )
        .is_some()
    );
    task.pause_reason = Some("waiting for review".to_string());
    assert!(
        build_failed_reason_row(
            crate::app::i18n::I18n::new(vw_config_types::ui::DesktopLocale::SimplifiedChinese),
            &task
        )
        .is_some()
    );

    task.selected_worktree_path = Some(" /repo/.worktrees/task-1 ".to_string());
    assert!(
        build_selected_worktree_row(
            crate::app::i18n::I18n::new(vw_config_types::ui::DesktopLocale::SimplifiedChinese),
            &task
        )
        .is_some()
    );
    task.merge_target_branch = Some("main".to_string());

    app.task_board_worktree_snapshot = Some(snapshot_with_lock(None));
    assert!(build_merge_lock_row(&app, &task).is_some());
    app.task_board_worktree_snapshot = Some(snapshot_with_lock(Some("task-1")));
    assert!(build_merge_lock_row(&app, &task).is_some());
    app.task_board_worktree_snapshot = Some(snapshot_with_lock(Some("task-2")));
    assert!(build_merge_lock_row(&app, &task).is_some());
}

#[test]
fn field_and_action_builders_cover_draft_edit_and_bulk_message_paths() {
    let mut app = test_app();

    keep_element(build_priority_field(&app, false));
    keep_element(build_priority_field(&app, true));
    keep_element(build_task_split_mode_selector(&app, false));
    keep_element(build_task_split_mode_selector(&app, true));
    keep_element(build_bulk_task_split_selector(&app));
    keep_element(build_agent_field(&app, false));
    keep_element(build_agent_field(&app, true));
    keep_element(build_bulk_agent_selector(&app));
    keep_element(build_action_buttons(&app, false));
    keep_element(build_action_buttons(&app, true));

    app.task_board_create_submit_success = true;
    app.task_board_edit_submit_success = true;
    keep_element(build_action_buttons(&app, false));
    keep_element(build_action_buttons(&app, true));

    app.task_board_is_import_mode = true;
    keep_element(build_action_buttons(&app, false));
}

#[test]
fn task_panel_builds_create_import_edit_and_logs_layouts() {
    let mut app = test_app();
    keep_element(build_task_panel(&app, false));

    app.task_board_is_import_mode = true;
    app.task_board_import_prompt_collapsed = false;
    keep_element(build_task_panel(&app, false));
    app.task_board_import_prompt_collapsed = true;
    keep_element(build_task_panel(&app, false));

    let mut task = Task { id: "task-panel".to_string(), ..Task::default() };
    task.status = TaskStatus::Failed;
    task.last_error = Some("executor unavailable".to_string());
    task.selected_worktree_path = Some("/repo/.worktrees/task-panel".to_string());
    task.merge_target_branch = Some("main".to_string());
    app.project_path = Some("/repo".to_string());
    app.task_board_worktree_snapshot = Some(snapshot_with_lock(Some("task-panel")));
    app.task_board_viewing_logs = Some(task);
    app.task_board_detail_tab = TaskBoardDetailTab::Basic;
    keep_element(build_task_panel(&app, true));

    app.task_board_detail_tab = TaskBoardDetailTab::Logs;
    keep_element(build_task_panel(&app, true));

    app.task_board_viewing_logs = None;
    app.task_board_detail_tab = TaskBoardDetailTab::Basic;
    keep_element(build_task_panel(&app, true));
}

#[test]
fn supporting_panel_builders_construct_tabs_headers_and_completion_options() {
    let mut app = test_app();

    keep_element(section_block(&app, "标题", "说明", text("内容")));
    keep_element(build_detail_tabs(&app));
    std::hint::black_box(build_detail_tab_button(
        "日志",
        TaskBoardDetailTab::Logs,
        TaskBoardDetailTab::Basic,
    ));

    let create_header = build_header_row(
        crate::app::i18n::I18n::new(vw_config_types::ui::DesktopLocale::SimplifiedChinese),
        text("新建任务"),
        None,
        None,
        build_close_button(TaskBoardMessage::CreateTaskCancelled),
    );
    std::hint::black_box(create_header);

    let mut completed_task =
        Task { id: "completed-task".to_string(), status: TaskStatus::Completed, ..Task::default() };
    completed_task.last_execution_duration_ms = Some(1_500);
    let completed_header = build_header_row(
        crate::app::i18n::I18n::new(vw_config_types::ui::DesktopLocale::SimplifiedChinese),
        text("任务详情"),
        Some(completed_task.id.as_str()),
        Some(&completed_task),
        build_close_button(TaskBoardMessage::CloseTaskLogs),
    );
    std::hint::black_box(completed_header);

    let running_task =
        Task { id: "running-task".to_string(), status: TaskStatus::Running, ..Task::default() };
    let running_header = build_header_row(
        crate::app::i18n::I18n::new(vw_config_types::ui::DesktopLocale::SimplifiedChinese),
        text("任务详情"),
        Some(running_task.id.as_str()),
        Some(&running_task),
        build_close_button(TaskBoardMessage::CloseTaskLogs),
    );
    std::hint::black_box(running_header);

    keep_element(build_prompt_field(&app));
    keep_element(build_mode_toggle(&app));
    keep_element(build_clear_prompt_checkbox(&app));
    keep_element(build_close_after_create_checkbox(&app));
    keep_element(build_close_after_edit_checkbox(&app));
    keep_element(build_create_options_row(&app));
    keep_element(build_edit_options_row(&app));

    app.task_board_is_import_mode = true;
    keep_element(build_mode_toggle(&app));
}

#[test]
fn import_field_builds_each_template_format_and_priority_fallback() {
    let mut app = test_app();
    app.task_board_is_import_mode = true;
    app.task_board_draft.priority = "not-a-number".to_string();
    app.task_board_import_prompt_collapsed = false;

    for format in [
        crate::app::task::TaskImportPromptFormat::Json,
        crate::app::task::TaskImportPromptFormat::Csv,
        crate::app::task::TaskImportPromptFormat::Tsv,
    ] {
        app.task_board_import_prompt_format = format;
        keep_element(build_import_field(&app));
    }

    app.task_board_import_prompt_collapsed = true;
    keep_element(build_import_field(&app));
}

#[cfg(not(target_arch = "wasm32"))]
mod help_rollout {
    use super::super::super::modals::modals_tests::help_rollout::*;
    use super::*;
    use crate::app::i18n::I18n;
    use crate::app::message::ChatMessage;
    use crate::app::task::SubTask;
    use iced::Size;
    use iced::keyboard::Key;

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn help_rollout_24_task_forms_preserve_actions() {
        let Some(mut app) =
            isolated_fixture(module_path!(), "help_rollout_24_task_forms_preserve_actions")
        else {
            return;
        };
        let mut renderer = renderer().await;
        app.task_board_draft.priority = "173".into();
        app.task_board_draft.subtasks = vec!["draft fixture subtask".into()];
        app.task_board_new_subtask_content = "new fixture subtask".into();
        for (locale, theme, width) in LAYOUTS {
            app.desktop_locale = locale;
            app.app_theme = theme;
            let i18n = I18n::new(locale);
            let size = Size::new(width, 1040);
            app.task_board_create_modal_open = true;
            app.task_board_is_import_mode = false;
            let mut ui = scene(crate::app::views::task_board::view(&app), &mut renderer, size);
            for (title, description, artifact) in [
                ("新建任务", "集中填写提示词、模型、执行器与导入内容。", "create"),
                (
                    "任务配置",
                    "填写提示词并选择模型、代理、执行器、任务拆分与优先级。",
                    "create-config",
                ),
                ("创建行为", "控制创建完成后的默认收尾动作。", "create-options"),
            ] {
                check_help(&mut ui, &mut renderer, &app, size, title, description, artifact);
            }
            let bounds = geometry(&mut ui, &renderer);
            locate(&bounds.inputs, "173");
            locate(&bounds.inputs, "draft fixture subtask");
            for arrow in ["↑", "↓"] {
                assert!(
                    click(&mut ui, &mut renderer, locate(&bounds.labels, arrow).center())
                        .is_empty(),
                    "a single subtask cannot be reordered"
                );
            }
            assert!(matches!(
                click(
                    &mut ui,
                    &mut renderer,
                    locate(&bounds.labels, i18n.__("新增子任务")).center(),
                )
                .as_slice(),
                [Message::TaskBoard(TaskBoardMessage::AddDraftSubtask)]
            ));
            assert!(matches!(
                click(&mut ui, &mut renderer, locate(&bounds.labels, i18n.__("创建任务")).center())
                    .as_slice(),
                [Message::TaskBoard(TaskBoardMessage::CreateTaskSubmitted)]
            ));
            assert!(matches!(
                click(&mut ui, &mut renderer, locate(&bounds.labels, i18n.__("取消")).center())
                    .as_slice(),
                [Message::TaskBoard(TaskBoardMessage::CreateTaskCancelled)]
            ));
            drop(ui);

            app.task_board_is_import_mode = true;
            let mut ui = scene(crate::app::views::task_board::view(&app), &mut renderer, size);
            check_help(
                &mut ui,
                &mut renderer,
                &app,
                size,
                "批量导入",
                "支持 JSON、CSV、TSV，适合一次性导入多条任务。",
                "import",
            );
            let bounds = geometry(&mut ui, &renderer);
            assert!(bounds.labels.iter().any(|(label, _)| {
                label.contains("priority, prompt") && label.contains("CSV/TSV")
            }));
            assert!(matches!(
                click(&mut ui, &mut renderer, locate(&bounds.labels, i18n.__("导入任务")).center())
                    .as_slice(),
                [Message::TaskBoard(TaskBoardMessage::ImportTasksSubmitted)]
            ));
            drop(ui);

            app.task_board_create_modal_open = false;
            app.task_board_detail_tab = TaskBoardDetailTab::Basic;
            app.task_board_viewing_logs = Some(Task {
                id: "fixture-task".into(),
                status: TaskStatus::Failed,
                last_error: Some("fixture execution error".into()),
                subtasks: vec![SubTask::new("现有子任务 / Existing subtask".into())],
                ..Task::default()
            });
            let mut ui = scene(crate::app::views::task_board::view(&app), &mut renderer, size);
            for (title, description, artifact) in [
                (
                    "任务详情",
                    "查看运行状态、调整执行参数，并保持日志与子任务在同一侧面板内。",
                    "detail",
                ),
                ("任务配置", "编辑提示词、模型、代理、执行器、任务拆分与优先级。", "edit-config"),
                ("编辑行为", "保存成功后的面板处理策略。", "edit-options"),
            ] {
                check_help(&mut ui, &mut renderer, &app, size, title, description, artifact);
            }
            let bounds = geometry(&mut ui, &renderer);
            locate(&bounds.labels, "fixture-task");
            locate(
                &bounds.labels,
                &crate::app::i18n::tasks::status_label(i18n, TaskStatus::Failed),
            );
            assert!(
                bounds.labels.iter().any(|(label, _)| label.contains("fixture execution error"))
            );
            locate(&bounds.inputs, "new fixture subtask");
            let click_label = |ui: &mut Ui<'_>, renderer: &mut iced::Renderer, label| {
                click(ui, renderer, locate(&bounds.labels, i18n.__(label)).center())
            };
            assert!(matches!(
                click_label(&mut ui, &mut renderer, "添加").as_slice(),
                [Message::TaskBoard(TaskBoardMessage::AddSubTask { task_id, content })]
                    if task_id == "fixture-task" && content == "new fixture subtask"
            ));
            assert!(matches!(
                click_label(&mut ui, &mut renderer, "保存修改").as_slice(),
                [Message::TaskBoard(TaskBoardMessage::SaveEditingTask)]
            ));
            assert!(matches!(
                click_label(&mut ui, &mut renderer, "取消").as_slice(),
                [Message::TaskBoard(TaskBoardMessage::CloseTaskLogs)]
            ));
        }
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn help_rollout_24_chat_priority_preserves_editing() {
        let Some(mut app) =
            isolated_fixture(module_path!(), "help_rollout_24_chat_priority_preserves_editing")
        else {
            return;
        };
        let runtime = app.current_session_runtime_mut();
        runtime.task_mode_enabled = true;
        runtime.task_mode_priority = "173".into();

        let mut renderer = renderer().await;
        for (locale, theme, width) in LAYOUTS {
            app.desktop_locale = locale;
            app.app_theme = theme;
            let size = Size::new(width, 640);
            use iced::advanced::{Layout, layout::Limits, widget::Tree};

            fn priority_row(layout: Layout<'_>) -> Option<Layout<'_>> {
                // The priority row is the first of the three task mode rows
                // with a label, a spacer, and a fixed 180 px control.
                let mut children = layout.children();
                if children.len() == 3
                    && children.nth(2).is_some_and(|control| control.bounds().width == 180.0)
                {
                    return Some(layout);
                }
                layout.children().find_map(priority_row)
            }

            let mut view = crate::app::components::input_panel::view(&app);
            let mut tree = Tree::new(view.as_widget());
            let node = view.as_widget_mut().layout(
                &mut tree,
                &renderer,
                &Limits::new(Size::ZERO, Size::new(width as f32, size.height as f32)),
            );
            let row = priority_row(Layout::new(&node)).expect("task mode priority row");
            let mut children = row.children();
            let title = children.next().unwrap();
            let input = children.nth(1).unwrap();
            let icon = title.children().nth(1).expect("priority label has a help anchor");
            let mut ui = scene(view, &mut renderer, size);
            let click_messages = check_help_at(
                &mut ui,
                &mut renderer,
                &app,
                size,
                I18n::new(locale).__("优先级"),
                icon.bounds().center(),
                "chat-priority",
            );
            // DropArea emits its normal release message for both clicks.
            assert!(matches!(
                click_messages.as_slice(),
                [Message::Chat(ChatMessage::InputAreaDragDrop)]
            ));
            assert!(matches!(
                click(&mut ui, &mut renderer, input.bounds().center()).as_slice(),
                [Message::Chat(ChatMessage::InputAreaDragDrop)]
            ));
            assert!(matches!(
                key(&mut ui, &mut renderer, Key::Character("9".into()), Some("9")).as_slice(),
                [Message::Chat(ChatMessage::TaskModePriorityChanged(value))]
                    if value.len() == 4 && value.contains('9')
            ));
        }
    }
}
