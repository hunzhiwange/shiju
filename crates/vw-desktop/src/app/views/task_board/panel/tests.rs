// Task 20 ownership: Host contracts: task forms compose host context-help and injected editors; chat priority exercises input_panel drag/drop and ChatMessage routing outside the task-board domain.
use crate::app::Message;
use crate::app::message::TaskBoardMessage;
use crate::app::state::TaskBoardDetailTab;
use crate::app::task::{Task, TaskStatus};
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
        app.task_board.task_board_draft.priority = "173".into();
        for (locale, theme, width) in LAYOUTS {
            app.general_settings.desktop_locale = locale;
            app.appearance.app_theme = theme;
            let i18n = I18n::new(locale);
            let size = Size::new(width, 1040);
            app.task_board.task_board_create_modal_open = true;
            app.task_board.task_board_is_import_mode = false;
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

            app.task_board.task_board_is_import_mode = true;
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

            app.task_board.task_board_create_modal_open = false;
            app.task_board.task_board_detail_tab = TaskBoardDetailTab::Basic;
            app.task_board.task_board_viewing_logs = Some(Task {
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
            let history = locate(&bounds.labels, "现有子任务 / Existing subtask");
            assert!(click(&mut ui, &mut renderer, history.center()).is_empty());
            let click_label = |ui: &mut Ui<'_>, renderer: &mut iced::Renderer, label| {
                click(ui, renderer, locate(&bounds.labels, i18n.__(label)).center())
            };
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
            app.general_settings.desktop_locale = locale;
            app.appearance.app_theme = theme;
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

#[test]
fn work_plan_detail_entry_clears_selection_on_project_switch() {
    use crate::app::message::task_board::update;

    let (mut app, _) = crate::app::App::new();
    let old_project = "/repo/work-old";
    let new_project = "/repo/work-new";
    let task_id = "shared-id";
    let open_detail = |path: &str| TaskBoardMessage::OpenTaskDetail {
        project_path: path.into(),
        task_id: task_id.into(),
    };

    let _ = app.open_project(old_project.into());
    app.task_board.task_board_tasks.push(Task {
        id: task_id.into(),
        status: TaskStatus::Running,
        ..Task::default()
    });
    let _ = update(&mut app, open_detail(old_project));
    assert_eq!(app.task_board.task_board_editing_task_id.as_deref(), Some(task_id));
    let _ = update(&mut app, TaskBoardMessage::SelectDetailTab(TaskBoardDetailTab::Logs));

    let _ = app.open_project(new_project.into());
    assert!(app.task_board.task_board_editing_task_id.is_none());
    assert!(app.task_board.task_board_viewing_logs.is_none());
    assert_eq!(app.task_board.task_board_detail_tab, TaskBoardDetailTab::Basic);

    let updated = Task {
        id: task_id.into(),
        status: TaskStatus::Failed,
        ..Task::default()
    };
    app.task_board.task_board_tasks.push(updated);
    let _ = update(&mut app, open_detail(old_project));
    let _ = update(&mut app, TaskBoardMessage::ViewTaskLogs {
        project_path: old_project.into(),
        task_id: task_id.into(),
    });
    assert!(app.task_board.task_board_viewing_logs.is_none());

    let _ = update(&mut app, open_detail(new_project));
    assert_eq!(app.task_board.task_board_viewing_logs.as_ref().unwrap().status, TaskStatus::Failed);

    let _ = update(&mut app, TaskBoardMessage::TasksLoaded {
        project_path: new_project.into(),
        tasks: vec![Task { id: task_id.into(), status: TaskStatus::Completed, ..Task::default() }],
    });
    assert_eq!(app.task_board.task_board_viewing_logs.as_ref().unwrap().status, TaskStatus::Completed);

    let _ = update(&mut app, TaskBoardMessage::TasksLoaded {
        project_path: new_project.into(),
        tasks: vec![],
    });
    assert!(app.task_board.task_board_viewing_logs.is_none());
    assert!(app.task_board.task_board_editing_task_id.is_none());
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn work_plan_detail_panel_creates_and_opens_independent_child() {
    use desktop_task_board::work_views::ChildTaskDraft;
    use vw_api_agent_scheduler::AgentSchedulerTaskPlanPlacementDto::Child;

    let mut parent = Task::new(17);
    parent.model = "parent-model".into();
    parent.prompt = "parent prompt".into();
    parent.label_ids.push("parent label".into());
    parent.add_log("parent log".to_owned());
    let mut draft = ChildTaskDraft::new("/project".into(), &parent, TaskStatus::Pool);
    draft.prompt = iced::widget::text_editor::Content::with_text("child prompt");
    let (child, placement) = draft.build().unwrap();
    assert_ne!(child.id, parent.id);
    assert_eq!(child.model, parent.model);
    assert_eq!(child.prompt, "child prompt");
    assert!(child.label_ids.is_empty());
    assert!(child.logs.iter().all(|log| log.message != "parent log"));
    assert_eq!(placement, Child { parent_task_id: parent.id.clone(), batch: 1 });

    let Some(mut app) = super::super::modals::modals_tests::help_rollout::isolated_fixture(
        module_path!(), "work_plan_detail_panel_creates_and_opens_independent_child",
    ) else {
        return;
    };
    app.navigation_tabs.project_path = Some("/project".into());
    app.task_board.task_board_tasks = vec![parent.clone(), child.clone()];
    for task in [&parent, &child] {
        let message = super::super::work_plan::open_detail("/project", &task.id);
        let Message::TaskBoard(message) = message else { panic!("expected task navigation") };
        let _ = crate::app::message::task_board::update(&mut app, message);
        let opened = app.task_board.task_board_viewing_logs.as_ref().unwrap();
        assert_eq!(opened.id, task.id);
        assert_eq!(opened.prompt, task.prompt);
        assert_eq!(
            opened.logs.iter().any(|log| log.message == "parent log"),
            task.id == parent.id,
        );
    }
}

#[cfg(not(target_arch = "wasm32"))]
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn work_plan_e2e_desktop_open_plan_and_child_details() {
    use super::super::{modals::modals_tests::help_rollout::*, work_plan, work_sidebar};
    use crate::app::i18n::I18n;
    use crate::app::message::task_board::update;
    use crate::app::task::SubTask;
    use desktop_task_board::work_views::{BuiltinWorkView, WorkPlanGroupKey};
    use iced::keyboard::Key;
    use iced::{Element, Renderer, Size, Theme};
    use serde_json::json;
    use vw_api_agent_scheduler::AgentSchedulerTaskBoardStatusResponse;

    let Some(mut app) =
        isolated_fixture(module_path!(), "work_plan_e2e_desktop_open_plan_and_child_details")
    else {
        return;
    };
    let projects = tempfile::tempdir().unwrap();
    for name in ["plan-alpha", "plan-beta"] {
        std::fs::create_dir(projects.path().join(name)).unwrap();
    }
    let alpha = projects.path().join("plan-alpha").to_string_lossy().into_owned();
    let beta = projects.path().join("plan-beta").to_string_lossy().into_owned();
    let _ = app.open_project(alpha.clone());
    app.navigation_tabs.screen = crate::app::Screen::TaskBoard;
    app.task_board.show_task_board = true;
    app.workspace_state.recent_projects = vec![alpha.clone(), beta.clone()];
    app.workspace_state.recent_projects_meta.clear();
    app.task_board.switch_work_view(BuiltinWorkView::MyTasks);

    let snapshot = |project: &str, id: &str, title: &str, placement: serde_json::Value| {
        json!({
            "project_id": project, "task_id": id, "placement": placement,
            "status": "queued", "status_key": "pending", "priority": 1, "order_no": 0,
            "model": "fixture", "prompt": title, "retry_count": 0, "updated_at_ms": 1
        })
    };
    let mut child = snapshot(
        "alpha",
        "child",
        "Child details",
        json!({
            "kind": "child", "parent_task_id": "parent", "batch": 2
        }),
    );
    child["read_model"]["dispatch_blocker"] =
        json!({"kind": "waiting_for_earlier_batch", "task_ids": ["child-first"]});
    let mut legacy = snapshot("alpha", "legacy", "Legacy plan", json!(null));
    legacy["status"] = json!("completed");
    legacy["status_key"] = json!("completed");
    legacy["read_model"]["legacy_subtasks"] =
        json!([SubTask::new("Saved legacy checklist".into())]);
    let alpha_snapshot: AgentSchedulerTaskBoardStatusResponse = serde_json::from_value(json!({
        "project_id": "alpha",
        "tasks": [
            snapshot("alpha", "parent", "Parent details", json!({"kind":"root","outer_batch":1,"inner_batch":2})),
            snapshot("alpha", "later", "Later group", json!({"kind":"root","outer_batch":2,"inner_batch":1})),
            snapshot("alpha", "child-first", "First child", json!({"kind":"child","parent_task_id":"parent","batch":1})),
            child, legacy
        ]
    })).unwrap();
    let load = |app: &mut crate::app::App, path: &str, response| {
        let generation = app.task_board.task_board_workflow_generation;
        let _ = update(
            app,
            TaskBoardMessage::AgentSchedulerStatusLoaded {
                project_path: path.into(),
                generation,
                result: Ok(response),
            },
        );
    };
    load(&mut app, &alpha, alpha_snapshot.clone());

    let mut renderer = renderer().await;
    let size = Size::new(1000, 1600);
    let click_label = |view: Element<'_, Message>, renderer: &mut Renderer, label: &str| {
        let mut ui = scene(view, renderer, size);
        let position = locate(&geometry(&mut ui, renderer).labels, label).center();
        click(&mut ui, renderer, position)
    };
    let apply = |app: &mut crate::app::App, messages: Vec<Message>| {
        let [message]: [Message; 1] = messages.try_into().ok().expect("one board message");
        let Message::TaskBoard(message) = message else {
            panic!("expected task board message");
        };
        let _ = update(app, message);
    };

    let messages = click_label(work_sidebar::view(&app), &mut renderer, "工作计划");
    assert!(matches!(
        messages.as_slice(),
        [Message::TaskBoard(TaskBoardMessage::OpenWorkPlanView)]
    ));
    apply(&mut app, messages);
    assert!(app.task_board.work_plan.open);
    for theme in [Theme::Dark, Theme::Light] {
        app.appearance.app_theme = theme;
        let mut ui = scene(work_plan::project_view(&app), &mut renderer, size);
        ui.draw(
            &mut renderer,
            &app.appearance.app_theme,
            &iced::advanced::renderer::Style { text_color: app.appearance.app_theme.palette().text },
            iced::mouse::Cursor::Unavailable,
        );
        let bounds = geometry(&mut ui, &renderer);
        locate(&bounds.labels, "▾ 外层批次 1 · 完成 0/1 · 进行中 0 · 等待 0");
        locate(&bounds.labels, "▾ 1.2 · 完成 0/1 · 进行中 0 · 等待 0");
        locate(&bounds.labels, "▾ 2.1 · 完成 0/1 · 进行中 0 · 等待 0");
        assert!(!bounds.labels.iter().any(|(label, _)| label == "Child details"));
    }
    let messages = click_label(
        work_plan::project_view(&app),
        &mut renderer,
        "▾ 外层批次 1 · 完成 0/1 · 进行中 0 · 等待 0",
    );
    apply(&mut app, messages);
    assert!(app.task_board.work_plan.collapsed.contains(&WorkPlanGroupKey::Outer(1)));
    let messages = click_label(
        work_plan::project_view(&app),
        &mut renderer,
        "▸ 外层批次 1 · 完成 0/1 · 进行中 0 · 等待 0",
    );
    apply(&mut app, messages);
    let messages = click_label(work_plan::project_view(&app), &mut renderer, "Parent details");
    apply(&mut app, messages);
    assert_eq!(app.task_board.task_board_editing_task_id.as_deref(), Some("parent"));

    {
        let mut ui = scene(super::build_task_panel(&app, true), &mut renderer, size);
        let bounds = geometry(&mut ui, &renderer);
        let i18n = I18n::new(app.general_settings.desktop_locale);
        locate(&bounds.labels, i18n.__("任务快捷操作"));
        locate(&bounds.labels, i18n.__("属性"));
        locate(&bounds.labels, "已保存批次 1.2");
        locate(&bounds.labels, "安排批次");
        locate(&bounds.labels, "新增子任务");
    }
    let parent = app.task_board.task_board_viewing_logs.as_ref().unwrap();
    let messages =
        click_label(work_plan::detail(&app, parent).unwrap(), &mut renderer, "新增子任务");
    apply(&mut app, messages);
    assert!(app.task_board.work_plan.child_draft.is_some());
    {
        let background = iced::widget::button("Background control")
            .on_press(Message::TaskBoard(TaskBoardMessage::CloseTaskLogs));
        let mut ui =
            scene(work_plan::with_child_modal(&app, background.into()), &mut renderer, size);
        assert!(click(&mut ui, &mut renderer, iced::Point::new(20.0, 12.0)).is_empty());
        let messages =
            key(&mut ui, &mut renderer, Key::Named(iced::keyboard::key::Named::Escape), None);
        assert!(matches!(
            messages.as_slice(),
            [Message::TaskBoard(TaskBoardMessage::CloseWorkPlanChild)]
        ));
        drop(ui);
        apply(&mut app, messages);
    }
    let parent = app.task_board.task_board_viewing_logs.as_ref().unwrap();
    let messages =
        click_label(work_plan::detail(&app, parent).unwrap(), &mut renderer, "Child details");
    let stale_child_click = match messages.as_slice() {
        [
            Message::TaskBoard(
                message @ TaskBoardMessage::OpenTaskDetail { project_path, task_id },
            ),
        ] if project_path == &alpha && task_id == "child" => message.clone(),
        _ => panic!("expected a child detail click scoped to alpha"),
    };
    apply(&mut app, messages);
    let child = app.task_board.task_board_viewing_logs.as_ref().unwrap();
    assert_eq!(child.id, "child");
    let mut ui = scene(work_plan::detail(&app, child).unwrap(), &mut renderer, size);
    let bounds = geometry(&mut ui, &renderer);
    locate(&bounds.labels, "子任务批次 2");
    locate(&bounds.labels, "等待前序批次 · child-first");
    let back = click(&mut ui, &mut renderer, locate(&bounds.labels, "返回父任务 parent").center());
    drop(ui);
    apply(&mut app, back);
    assert_eq!(app.task_board.task_board_editing_task_id.as_deref(), Some("parent"));

    let _ = update(&mut app, TaskBoardMessage::SelectDetailTab(TaskBoardDetailTab::Logs));
    let parent = app.task_board.task_board_viewing_logs.as_ref().unwrap();
    let messages =
        click_label(work_plan::history(&app, parent).unwrap(), &mut renderer, "包含子任务");
    apply(&mut app, messages);
    let request = app.task_board.work_plan.history.as_ref().unwrap().request;
    let _ =
        update(
            &mut app,
            TaskBoardMessage::WorkPlanHistoryLoaded {
                project_path: alpha.clone(),
                request,
                result: Ok(vec![serde_json::from_value(json!({
            "cursor": 2, "project_id": "alpha", "task_id": "child", "run_id": "child-run",
            "kind": "run_completed", "message": "Child execution result", "created_at_ms": 1
        })).unwrap()]),
            },
        );
    {
        let mut ui = scene(super::build_task_panel(&app, true), &mut renderer, size);
        let bounds = geometry(&mut ui, &renderer);
        locate(&bounds.labels, "子任务 child · 运行 child-run");
        assert!(bounds.labels.iter().any(|(label, _)| label.ends_with("Child execution result")));
    }

    let _ = update(&mut app, TaskBoardMessage::SelectDetailTab(TaskBoardDetailTab::Basic));
    let messages = click_label(work_plan::project_view(&app), &mut renderer, "Legacy plan");
    apply(&mut app, messages);
    {
        let mut ui = scene(super::build_task_panel(&app, true), &mut renderer, size);
        let bounds = geometry(&mut ui, &renderer);
        let checklist = locate(&bounds.labels, "Saved legacy checklist");
        assert!(click(&mut ui, &mut renderer, checklist.center()).is_empty());
        assert!(!bounds.labels.iter().any(|(label, _)| label == "开始执行计划"));
    }

    let messages = click_label(work_sidebar::view(&app), &mut renderer, "plan-beta");
    apply(&mut app, messages);
    assert_eq!(app.navigation_tabs.project_path.as_deref(), Some(beta.as_str()));
    assert!(app.task_board.task_board_viewing_logs.is_none());
    let beta_snapshot = serde_json::from_value(json!({
        "project_id": "beta",
        "tasks": [snapshot("beta", "child", "Beta task with the same ID", json!(null))]
    }))
    .unwrap();
    load(&mut app, &beta, beta_snapshot);
    load(&mut app, &alpha, alpha_snapshot);
    let _ = update(&mut app, stale_child_click);
    assert!(app.task_board.task_board_viewing_logs.is_none());
    let messages = click_label(work_sidebar::view(&app), &mut renderer, "工作计划");
    apply(&mut app, messages);
    let messages =
        click_label(work_plan::project_view(&app), &mut renderer, "Beta task with the same ID");
    apply(&mut app, messages);
    assert_eq!(
        app.task_board.task_board_viewing_logs.as_ref().unwrap().prompt,
        "Beta task with the same ID"
    );
    assert!(app.task_board.work_plan.history.is_none());
}
