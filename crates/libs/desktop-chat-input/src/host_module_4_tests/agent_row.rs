use crate::input_panel::agent_children::{
    AgentChildSession, AgentChildSessionKind, AgentChildSessionStatus,
};
use crate::input_panel::agent_sessions::Action;
use iced::Theme;
fn i18n() -> desktop_i18n::I18n {
    desktop_i18n::I18n::new(config_ui::DesktopLocale::SimplifiedChinese)
}
fn child(session_id: &str, agent: &str, title: &str, task: &str) -> AgentChildSession {
    AgentChildSession {
        session_id: session_id.to_string(),
        agent: agent.to_string(),
        task: task.to_string(),
        title: title.to_string(),
        directory: "/workspace/project".to_string(),
        kind: AgentChildSessionKind::Agent,
        status: AgentChildSessionStatus::Running,
        depth: 0,
        plan_run_task_id: None,
        task_path: None,
    }
}

#[tokio::test]
async fn child_session_navigation_input_row_keeps_stop_separate_from_status() {
    use iced::advanced::{clipboard, renderer::Headless};
    use iced::{Event, Font, Pixels, Point, Size, mouse};
    use iced_runtime::user_interface::{Cache, UserInterface};

    let mut child = child("nested-child", "审查", "检查中文状态", "工作流执行");
    child.depth = 1;
    child.kind = AgentChildSessionKind::WorkflowExecution;
    child.plan_run_task_id = Some("owning-plan".into());
    let mut renderer =
        iced::Renderer::new(Font::DEFAULT, Pixels(16.0), Some("tiny-skia")).await.unwrap();
    let viewport = Size::new(340.0, 100.0);

    for theme in [Theme::Light, Theme::Dark] {
        let mut ui = UserInterface::build(
            crate::input_panel::agent_sessions::agent_session_row(
                i18n(),
                0,
                &child,
                std::convert::identity::<Action>,
            ),
            viewport,
            Cache::new(),
            &mut renderer,
        );
        ui.draw(&mut renderer, &theme, &Default::default(), mouse::Cursor::Unavailable);
        let mut click = |position| {
            let mut messages = Vec::new();
            ui.update(
                &[
                    Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)),
                    Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)),
                ],
                mouse::Cursor::Available(position),
                &mut renderer,
                &mut clipboard::Null,
                &mut messages,
            );
            messages
        };
        assert!(click(Point::new(50.0, 18.0)).is_empty());
        assert!(matches!(
            click(Point::new(325.0, 22.0)).as_slice(),
            [Action::StopPlanRun(id)] if id == "owning-plan"
        ));
    }
}
