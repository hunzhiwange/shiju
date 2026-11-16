use desktop_task_model::{Task, TaskStatus};
//! Owned panel data only; no desktop lifecycle, gateway, or App clone.
pub(super) struct PanelFixture {
    pub(super) app_theme: iced::Theme,
    pub(super) desktop_locale: config_ui::DesktopLocale,
    pub(super) agents_settings: desktop_settings_agents::agents_state::AgentsSettingsState,
    pub(super) project_path: Option<String>,
    pub(super) task_board_bulk_agent: String,
    pub(super) task_board_bulk_task_split_mode: desktop_task_model::TaskSplitMode,
    pub(super) task_board_detail_tab: crate::message::TaskBoardDetailTab,
    pub(super) task_board_draft: desktop_task_model::TaskDraft,
    pub(super) task_board_import_prompt_format: desktop_task_model::TaskImportPromptFormat,
    pub(super) task_board_settings: desktop_task_model::TaskBoardSettings,
    pub(super) task_board_viewing_logs: Option<desktop_task_model::Task>,
    pub(super) task_board_worktree_snapshot:
        Option<desktop_task_executor::executor::state::WorktreePoolSnapshot>,
    pub(super) task_board_logs_editor_id: iced::widget::Id,
    pub(super) task_board_logs_context_menu_pos: Option<(f32, f32)>,
    pub(super) current_line_height: f32,
    pub(super) task_board_logs_scroll_top_line: f32,
    pub(super) task_board_new_subtask_content: String,
    pub(super) task_board_import_editor: iced::widget::text_editor::Content,
    pub(super) task_board_prompt_editor: iced::widget::text_editor::Content,
    pub(super) task_board_logs_editor: iced::widget::text_editor::Content,
    pub(super) task_board_clear_prompt_after_create: bool,
    pub(super) task_board_close_after_create: bool,
    pub(super) task_board_close_after_edit: bool,
    pub(super) task_board_create_submit_success: bool,
    pub(super) task_board_edit_submit_success: bool,
    pub(super) task_board_import_prompt_collapsed: bool,
    pub(super) task_board_is_import_mode: bool,
    pub(super) task_board_logs_context_menu_open: bool,
    pub(super) acp_agents: Vec<String>,
    pub(super) task_board_executor_popover: bool,
    pub(super) task_board_bulk_executor_popover: bool,
    pub(super) task_board_bulk_acp_agent: Option<String>,
    pub(super) task_board_worktree_pixel_office: bool,
    pub(super) model_settings: desktop_settings_providers::state::ModelSettingsState,
    pub(super) task_board_tasks: Vec<Task>,
    pub(super) task_board_selected_tasks: std::collections::HashSet<String>,
    pub(super) task_board_bulk_active_status: Option<TaskStatus>,
    pub(super) task_board_bulk_priority_input: String,
    pub(super) task_board_bulk_reasoning_effort: Option<vw_api_chat_identity::ReasoningEffortDto>,
    pub(super) task_board_dragging: Option<(String, TaskStatus)>,
    pub(super) cursor_position: iced::Point,
    pub(super) task_board_context_menu: Option<(String, f32, f32)>,
    pub(super) task_board_column_has_vertical_scrollbar:
        std::collections::HashMap<TaskStatus, bool>,
    pub(super) window_size: (f32, f32),
    pub(super) task_board_executor_running: bool,
    pub(super) task_board_worktree_manual_action_kind: Option<&'static str>,
    pub(super) task_board_worktree_manual_confirm_kind: Option<&'static str>,
    pub(super) task_board_worktree_panel_expanded: bool,
    pub(super) task_board_worktree_snapshot_loading: bool,
    pub(super) task_board_worktree_action_logs: Vec<String>,
    pub(super) task_board_create_modal_open: bool,
    pub(super) task_board_settings_modal_open: bool,
    pub(super) task_board_settings_modal_tab: crate::message::TaskBoardSettingsModalTab,
}
impl Default for PanelFixture {
    fn default() -> Self {
        Self {
            task_board_tasks: Default::default(),
            task_board_selected_tasks: Default::default(),
            task_board_bulk_active_status: Default::default(),
            task_board_bulk_priority_input: Default::default(),
            task_board_bulk_reasoning_effort: Default::default(),
            task_board_dragging: Default::default(),
            cursor_position: iced::Point::ORIGIN,
            task_board_context_menu: Default::default(),
            task_board_column_has_vertical_scrollbar: Default::default(),
            window_size: (1200.0, 800.0),
            task_board_executor_running: Default::default(),
            task_board_worktree_manual_action_kind: Default::default(),
            task_board_worktree_manual_confirm_kind: Default::default(),
            task_board_worktree_panel_expanded: Default::default(),
            task_board_worktree_snapshot_loading: Default::default(),
            task_board_worktree_action_logs: Default::default(),
            task_board_create_modal_open: Default::default(),
            task_board_settings_modal_open: Default::default(),
            task_board_settings_modal_tab: Default::default(),
            app_theme: iced::Theme::Dark,
            desktop_locale: config_ui::DesktopLocale::SimplifiedChinese,
            agents_settings: Default::default(),
            project_path: Default::default(),
            task_board_bulk_agent: Default::default(),
            task_board_bulk_task_split_mode: Default::default(),
            task_board_detail_tab: Default::default(),
            task_board_draft: Default::default(),
            task_board_import_prompt_format: Default::default(),
            task_board_settings: Default::default(),
            task_board_viewing_logs: Default::default(),
            task_board_worktree_snapshot: Default::default(),
            task_board_logs_editor_id: iced::widget::Id::unique(),
            task_board_logs_context_menu_pos: Default::default(),
            current_line_height: 20.0,
            task_board_logs_scroll_top_line: Default::default(),
            task_board_new_subtask_content: Default::default(),
            task_board_import_editor: Default::default(),
            task_board_prompt_editor: Default::default(),
            task_board_logs_editor: Default::default(),
            task_board_clear_prompt_after_create: Default::default(),
            task_board_close_after_create: Default::default(),
            task_board_close_after_edit: Default::default(),
            task_board_create_submit_success: Default::default(),
            task_board_edit_submit_success: Default::default(),
            task_board_import_prompt_collapsed: Default::default(),
            task_board_is_import_mode: Default::default(),
            task_board_logs_context_menu_open: Default::default(),
            acp_agents: Default::default(),
            task_board_executor_popover: Default::default(),
            task_board_bulk_executor_popover: Default::default(),
            task_board_bulk_acp_agent: Default::default(),
            task_board_worktree_pixel_office: Default::default(),
            model_settings: Default::default(),
        }
    }
}
impl PanelFixture {
    fn input(&self) -> crate::panel::PanelView<'_> {
        crate::panel::PanelView {
            app_theme: &self.app_theme,
            desktop_locale: self.desktop_locale,
            agents_settings: &self.agents_settings,
            project_path: &self.project_path,
            task_board_bulk_agent: &self.task_board_bulk_agent,
            task_board_bulk_task_split_mode: self.task_board_bulk_task_split_mode,
            task_board_detail_tab: self.task_board_detail_tab,
            task_board_draft: &self.task_board_draft,
            task_board_import_prompt_format: self.task_board_import_prompt_format,
            task_board_settings: &self.task_board_settings,
            task_board_viewing_logs: &self.task_board_viewing_logs,
            task_board_worktree_snapshot: &self.task_board_worktree_snapshot,
            task_board_logs_editor_id: &self.task_board_logs_editor_id,
            task_board_logs_context_menu_pos: self.task_board_logs_context_menu_pos,
            current_line_height: self.current_line_height,
            task_board_logs_scroll_top_line: self.task_board_logs_scroll_top_line,
            task_board_new_subtask_content: &self.task_board_new_subtask_content,
            task_board_import_editor: &self.task_board_import_editor,
            task_board_prompt_editor: &self.task_board_prompt_editor,
            task_board_logs_editor: &self.task_board_logs_editor,
            task_board_clear_prompt_after_create: self.task_board_clear_prompt_after_create,
            task_board_close_after_create: self.task_board_close_after_create,
            task_board_close_after_edit: self.task_board_close_after_edit,
            task_board_create_submit_success: self.task_board_create_submit_success,
            task_board_edit_submit_success: self.task_board_edit_submit_success,
            task_board_import_prompt_collapsed: self.task_board_import_prompt_collapsed,
            task_board_is_import_mode: self.task_board_is_import_mode,
            task_board_logs_context_menu_open: self.task_board_logs_context_menu_open,
        }
    }
    fn executor(&self) -> crate::panel::executor_selector::ExecutorView<'_> {
        crate::panel::executor_selector::ExecutorView {
            task_board_draft: &self.task_board_draft,
            task_board_bulk_acp_agent: &self.task_board_bulk_acp_agent,
            task_board_executor_popover: self.task_board_executor_popover,
            task_board_bulk_executor_popover: self.task_board_bulk_executor_popover,
            desktop_locale: self.desktop_locale,
            acp_agents: &self.acp_agents,
        }
    }
}
pub(super) fn build_task_panel(
    app: &PanelFixture,
    edit: bool,
) -> iced::Element<'_, crate::panel::PanelMessage> {
    crate::panel::build_with_controls(
        app.input(),
        edit,
        crate::model_selector::SelectorView {
            draft_model: &app.task_board_draft.model,
            bulk_model: "auto",
            task_board_model_popover: false,
            task_board_bulk_model_popover: false,
            desktop_locale: app.desktop_locale,
            model_settings: &app.model_settings,
        },
        app.executor(),
        || None,
    )
}

pub(super) fn page(app: &PanelFixture) -> iced::Element<'_, crate::panel::PanelMessage> {
    use crate::{message::TaskBoardMessage as B, page::*, panel::PanelMessage as M};
    let input = PageView {
        task_board_create_modal_open: app.task_board_create_modal_open,
        task_board_worktree_pixel_office: app.task_board_worktree_pixel_office,
        kanban: true,
        viewing_logs: app.task_board_viewing_logs.is_some(),
        task_board_settings_modal_open: app.task_board_settings_modal_open,
    };
    assemble(
        input,
        PageBuilders {
            header: Box::new(|| {
                crate::control::build_control_buttons(&crate::control::ControlView {
                    desktop_locale: app.desktop_locale,
                    window_size: app.window_size,
                    project_path: app.project_path.as_deref(),
                    task_board_tasks: &app.task_board_tasks,
                    task_board_executor_running: app.task_board_executor_running,
                    task_board_worktree_manual_action_kind: app
                        .task_board_worktree_manual_action_kind,
                    task_board_worktree_manual_confirm_kind: app
                        .task_board_worktree_manual_confirm_kind,
                    task_board_worktree_panel_expanded: app.task_board_worktree_panel_expanded,
                    task_board_worktree_pixel_office: app.task_board_worktree_pixel_office,
                    task_board_worktree_snapshot: app.task_board_worktree_snapshot.as_ref(),
                    task_board_worktree_snapshot_loading: app.task_board_worktree_snapshot_loading,
                    task_board_worktree_action_logs: &app.task_board_worktree_action_logs,
                    agent_entries: vec![],
                })
                .map(M::TaskBoard)
            }),
            board: Box::new(|| board(app)),
            drag: Box::new(|| {
                crate::drag::build_drag_preview_layer(crate::drag::DragView {
                    task_board_dragging: &app.task_board_dragging,
                    task_board_tasks: &app.task_board_tasks,
                    desktop_locale: app.desktop_locale,
                    cursor_position: app.cursor_position,
                    window_size: app.window_size,
                })
                .map(M::TaskBoard)
            }),
            panel: Box::new(|edit| build_task_panel(app, edit)),
            settings: Box::new(|| {
                crate::modals::build_task_settings_modal(
                    crate::modals::SettingsView {
                        task_board_settings: &app.task_board_settings,
                        task_board_settings_modal_tab: app.task_board_settings_modal_tab,
                        desktop_locale: app.desktop_locale,
                        app_theme: &app.app_theme,
                    },
                    M::TaskBoard,
                    |i18n, theme, title, description| {
                        desktop_ui_controls::system_settings_common::settings_group_heading_with_help(i18n,theme,title,description)
                    },
                )
            }),
            context_closed: M::TaskBoard(B::ContextMenuClosed),
            close_logs: M::TaskBoard(B::CloseTaskLogs),
            close_settings: M::TaskBoard(B::CloseSettingsModal),
        },
    )
}
fn board(app: &PanelFixture) -> iced::Element<'_, crate::panel::PanelMessage> {
    use crate::panel::PanelMessage as M;
    use std::rc::Rc;
    crate::board::build_kanban_board(&crate::board::BoardView {
        task_board_settings: &app.task_board_settings,
        task_board_tasks: &app.task_board_tasks,
        task_board_selected_tasks: &app.task_board_selected_tasks,
        task_board_bulk_active_status: app.task_board_bulk_active_status,
        task_board_bulk_priority_input: &app.task_board_bulk_priority_input,
        task_board_bulk_acp_agent: &app.task_board_bulk_acp_agent,
        task_board_bulk_reasoning_effort: app.task_board_bulk_reasoning_effort,
        task_board_bulk_task_split_mode: app.task_board_bulk_task_split_mode,
        task_board_dragging: &app.task_board_dragging,
        cursor_position: app.cursor_position,
        task_board_context_menu: &app.task_board_context_menu,
        task_board_column_has_vertical_scrollbar: &app.task_board_column_has_vertical_scrollbar,
        desktop_locale: app.desktop_locale,
        slots: crate::board::BoardSlots {
            model: Rc::new(|| {
                crate::model_selector::build_bulk_model_selector(
                    crate::model_selector::SelectorView {
                        draft_model: &app.task_board_draft.model,
                        bulk_model: "auto",
                        task_board_model_popover: false,
                        task_board_bulk_model_popover: false,
                        desktop_locale: app.desktop_locale,
                        model_settings: &app.model_settings,
                    },
                )
                .map(M::Model)
            }),
            agent: Rc::new(|| crate::panel::build_bulk_agent_selector(app.input())),
            executor: Rc::new(|| {
                crate::panel::executor_selector::build_bulk_executor_selector(app.executor())
                    .map(M::TaskBoard)
            }),
            reasoning_effort: Rc::new(|| None),
            task_split: Rc::new(|| crate::panel::build_bulk_task_split_selector(app.input())),
            context_menu: Rc::new(move |task| {
                crate::modals::build_context_menu(desktop_i18n::I18n::new(app.desktop_locale), task)
                    .map(M::TaskBoard)
            }),
        },
    })
}

impl From<crate::message::TaskBoardMessage> for crate::panel::PanelMessage {
    fn from(message: crate::message::TaskBoardMessage) -> Self {
        Self::TaskBoard(message)
    }
}

impl PanelFixture {
    pub(super) fn update(&mut self, message: crate::panel::PanelMessage) {
        let crate::panel::PanelMessage::TaskBoard(message) = message else {
            panic!("expected board message")
        };
        let _ = crate::update::local::update_local(
            crate::update::local::LocalState {
                worktree_panel_expanded: &mut self.task_board_worktree_panel_expanded,
                worktree_pixel_office: &mut self.task_board_worktree_pixel_office,
                project_path: self.project_path.as_deref(),
                clear_prompt_after_create: &mut self.task_board_clear_prompt_after_create,
                close_after_create: &mut self.task_board_close_after_create,
                close_after_edit: &mut self.task_board_close_after_edit,
                settings: &self.task_board_settings,
                logs_auto_scroll: &mut Default::default(),
                editing_task_id: &mut Default::default(),
                detail_tab: &mut self.task_board_detail_tab,
                last_model: &mut Default::default(),
                create_modal_open: &mut self.task_board_create_modal_open,
                tasks: &mut self.task_board_tasks,
                bulk_acp_agent: &mut self.task_board_bulk_acp_agent,
                bulk_agent: &mut self.task_board_bulk_agent,
                bulk_executor_popover: &mut self.task_board_bulk_executor_popover,
                bulk_priority_input: &mut self.task_board_bulk_priority_input,
                bulk_reasoning_effort: &mut self.task_board_bulk_reasoning_effort,
                bulk_task_split_mode: &mut self.task_board_bulk_task_split_mode,
                column_has_vertical_scrollbar: &mut self.task_board_column_has_vertical_scrollbar,
                context_menu: &mut self.task_board_context_menu,
                create_submit_success: &mut self.task_board_create_submit_success,
                desc_editor: &mut Default::default(),
                draft: &mut self.task_board_draft,
                drag_pending: &mut Default::default(),
                dragging: &mut self.task_board_dragging,
                edit_submit_success: &mut self.task_board_edit_submit_success,
                executor_popover: &mut self.task_board_executor_popover,
                filter_priority: &mut Default::default(),
                filter_status: &mut Default::default(),
                import_editor: &mut self.task_board_import_editor,
                import_prompt_collapsed: &mut self.task_board_import_prompt_collapsed,
                import_prompt_format: &mut self.task_board_import_prompt_format,
                is_import_mode: &mut self.task_board_is_import_mode,
                last_acp_agent: &mut Default::default(),
                logs_context_menu_open: &mut self.task_board_logs_context_menu_open,
                logs_context_menu_pos: &mut self.task_board_logs_context_menu_pos,
                new_subtask_content: &mut self.task_board_new_subtask_content,
                prompt_editor: &mut self.task_board_prompt_editor,
                selected_task: &mut Default::default(),
                selected_tasks: &mut self.task_board_selected_tasks,
                settings_modal_open: &mut self.task_board_settings_modal_open,
                settings_modal_tab: &mut self.task_board_settings_modal_tab,
                sort_ascending: &mut Default::default(),
                sort_by_priority: &mut Default::default(),
                worktree_manual_confirm_kind: &mut self.task_board_worktree_manual_confirm_kind,
            },
            message,
            "main",
        );
    }
}
