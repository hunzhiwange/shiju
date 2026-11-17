//! 领域视图测试的内存数据，不初始化桌面或外部服务。
use super::ControlView;
use desktop_task_executor::executor::state::WorktreePoolSnapshot;
use desktop_task_model::{Task, agent_activity::AgentEntry};

#[derive(Default)]
pub struct TestApp {
    pub desktop_locale: config_ui::DesktopLocale,
    pub window_size: (f32, f32),
    pub project_path: Option<String>,
    pub task_board_tasks: Vec<Task>,
    pub task_board_executor_running: bool,
    pub task_board_worktree_manual_action_kind: Option<&'static str>,
    pub task_board_worktree_manual_confirm_kind: Option<&'static str>,
    pub task_board_worktree_panel_expanded: bool,
    pub task_board_worktree_pixel_office: bool,
    pub task_board_worktree_snapshot: Option<WorktreePoolSnapshot>,
    pub task_board_worktree_snapshot_loading: bool,
    pub task_board_worktree_action_logs: Vec<String>,
    pub agent_entries: Vec<AgentEntry<'static>>,
    pub project_is_git_repo: bool,
}

impl TestApp {
    pub fn new() -> (Self, ()) {
        (Self::default(), ())
    }

    pub fn view(&self) -> ControlView<'_> {
        ControlView {
            desktop_locale: self.desktop_locale,
            window_size: self.window_size,
            project_path: self.project_path.as_deref(),
            task_board_tasks: &self.task_board_tasks,
            task_board_executor_running: self.task_board_executor_running,
            task_board_worktree_manual_action_kind: self.task_board_worktree_manual_action_kind,
            task_board_worktree_manual_confirm_kind: self.task_board_worktree_manual_confirm_kind,
            task_board_worktree_panel_expanded: self.task_board_worktree_panel_expanded,
            task_board_worktree_pixel_office: self.task_board_worktree_pixel_office,
            task_board_worktree_snapshot: self.task_board_worktree_snapshot.as_ref(),
            task_board_worktree_snapshot_loading: self.task_board_worktree_snapshot_loading,
            task_board_worktree_action_logs: &self.task_board_worktree_action_logs,
            agent_entries: self.agent_entries.iter().map(|e| AgentEntry {
                key: e.key,
                label: e.label,
                enabled: e.enabled,
            }).collect(),
        }
    }
}

pub async fn assert_element_draws(
    mut element: iced::Element<'_, crate::message::TaskBoardMessage>,
    theme: &iced::Theme,
    width: f32,
) {
    use iced::advanced::renderer::Headless;
    use iced::Color;

    let mut renderer = <iced::Renderer as Headless>::new(
        iced::Font::DEFAULT,
        iced::Pixels(16.0),
        Some("tiny-skia"),
    )
    .await
    .expect("tiny-skia headless renderer should be available");
    let mut tree = iced::advanced::widget::Tree::new(element.as_widget());
    let viewport = iced::Rectangle::with_size(iced::Size::new(width, 800.0));
    let node = element.as_widget_mut().layout(
        &mut tree,
        &renderer,
        &iced::advanced::layout::Limits::new(iced::Size::ZERO, viewport.size()),
    );

    element.as_widget().draw(
        &tree,
        &mut renderer,
        theme,
        &iced::advanced::renderer::Style::default(),
        iced::advanced::Layout::new(&node),
        iced::mouse::Cursor::Unavailable,
        &viewport,
    );

    let pixels = renderer.screenshot(iced::Size::new(width as u32, 800), 1.0, Color::TRANSPARENT);
    assert!(pixels.chunks_exact(4).any(|pixel| pixel[3] > 0));
}
