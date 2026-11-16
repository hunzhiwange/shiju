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
        app.task_board.task_board_draft.subtasks = vec!["draft fixture subtask".into()];
        app.task_board.task_board_new_subtask_content = "new fixture subtask".into();
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
