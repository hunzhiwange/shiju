use desktop_task_model::*;
use crate::message::TaskBoardSettingsModalTab;

pub struct Fixture {
 pub task_board_draft: TaskDraft,
 pub task_board_bulk_acp_agent: Option<String>,
 pub task_board_bulk_reasoning_effort: Option<vw_api_chat_identity::ReasoningEffortDto>,
 pub task_board_executor_popover: bool,
 pub task_board_bulk_executor_popover: bool,
 pub desktop_locale: config_ui::DesktopLocale,
 pub acp_agents: Vec<String>,
 pub task_board_dragging: Option<(String, TaskStatus)>,
 pub task_board_tasks: Vec<Task>,
 pub cursor_position: iced::Point,
 pub window_size: (f32, f32),
 pub task_board_new_subtask_content: String,
 pub task_board_settings: TaskBoardSettings,
 pub task_board_settings_modal_tab: TaskBoardSettingsModalTab,
 pub app_theme: iced::Theme,
}
impl Fixture {
 pub fn new() -> (Self, ()) { (Self::default(), ()) }
 pub fn executor(&self) -> crate::panel::executor_selector::ExecutorView<'_> { crate::panel::executor_selector::ExecutorView { task_board_draft: &self.task_board_draft, task_board_bulk_acp_agent: &self.task_board_bulk_acp_agent, task_board_executor_popover: self.task_board_executor_popover, task_board_bulk_executor_popover: self.task_board_bulk_executor_popover, desktop_locale: self.desktop_locale, acp_agents: &self.acp_agents } }
 pub fn reasoning(&self) -> crate::panel::reasoning_selector::ReasoningView<'_> { crate::panel::reasoning_selector::ReasoningView { draft: &self.task_board_draft, bulk_agent: self.task_board_bulk_acp_agent.as_deref(), bulk_effort: self.task_board_bulk_reasoning_effort, model_options: &[None], desktop_locale: self.desktop_locale } }
 pub fn drag(&self) -> crate::drag::DragView<'_> { crate::drag::DragView { task_board_dragging: &self.task_board_dragging, task_board_tasks: &self.task_board_tasks, desktop_locale: self.desktop_locale, cursor_position: self.cursor_position, window_size: self.window_size } }
 pub fn settings(&self) -> crate::modals::SettingsView<'_> { crate::modals::SettingsView { task_board_settings: &self.task_board_settings, task_board_settings_modal_tab: self.task_board_settings_modal_tab, desktop_locale: self.desktop_locale, app_theme: &self.app_theme } }
}

impl Default for Fixture {
 fn default() -> Self { Self { task_board_draft: Default::default(), task_board_bulk_acp_agent: None, task_board_bulk_reasoning_effort: None, task_board_executor_popover: false, task_board_bulk_executor_popover: false, desktop_locale: config_ui::DesktopLocale::SimplifiedChinese, acp_agents: vec![], task_board_dragging: None, task_board_tasks: vec![], cursor_position: iced::Point::ORIGIN, window_size: (1200.,800.), task_board_new_subtask_content: String::new(), task_board_settings: Default::default(), task_board_settings_modal_tab: Default::default(), app_theme: iced::Theme::Dark } }
}
