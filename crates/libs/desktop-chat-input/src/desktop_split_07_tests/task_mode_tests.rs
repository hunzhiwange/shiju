//! 验证任务模式表单的执行器选项和布局。

use crate::input_panel::task_mode::{Input, Message, task_mode_form};

use iced::Length;
use vw_api_chat_identity::ReasoningEffortDto;

struct Fixture {
    acp_agents: Vec<String>,
    show_executor_popover: bool,
    theme: iced::Theme,
}
fn test_app() -> Fixture {
    Fixture {
        acp_agents: vec![],
        show_executor_popover: false,
        theme: iced::Theme::Dark,
    }
}
fn input(fixture: &Fixture) -> Input<'_> {
    Input {
        i18n: desktop_i18n::I18n::new(config_ui::DesktopLocale::SimplifiedChinese),
        theme: &fixture.theme,
        providers: &[],
        agents: &fixture.acp_agents,
        expanded: fixture.show_executor_popover,
    }
}

fn keep(element: iced::Element<'_, Message>) {
    std::hint::black_box(element);
}

#[test]
fn task_739_test_module_is_wired() {
    let path = std::path::Path::new(file!());
    assert!(path.ends_with("task_mode_tests.rs"));
}

#[test]
fn disabled_task_mode_returns_placeholder_element() {
    let app = test_app();

    let element = task_mode_form(
        input(&app),
        false,
        "5",
        "auto",
        None,
        None,
    );

    assert_eq!(element.as_widget().size().width, Length::Shrink);
}

#[test]
fn enabled_task_mode_builds_with_default_executor() {
    let mut app = test_app();
    app.acp_agents = vec!["codex".to_string(), "claude".to_string()];
    app.show_executor_popover = true;

    keep(task_mode_form(
        input(&app),
        true,
        "1",
        "provider/model",
        None,
        None,
    ));
}

#[test]
fn enabled_task_mode_builds_with_selected_executor() {
    let mut app = test_app();
    app.acp_agents = vec!["codex".to_string(), "claude".to_string()];
    keep(task_mode_form(
        input(&app),
        true,
        "99",
        "auto",
        Some("claude".to_string()),
        Some(ReasoningEffortDto::Max),
    ));
}

#[test]
fn enabled_task_mode_with_no_agents_fills_available_width() {
    let app = test_app();

    let element = task_mode_form(
        input(&app),
        true,
        "",
        "",
        None,
        None,
    );

    assert_eq!(element.as_widget().size().width, Length::Fill);
}

#[test]
fn enabled_task_mode_builds_when_selected_executor_is_not_in_agent_list() {
    let mut app = test_app();
    app.acp_agents = vec!["codex".to_string()];

    let element = task_mode_form(
        input(&app),
        true,
        "3",
        "auto",
        Some("external-agent".to_string()),
        Some(ReasoningEffortDto::Medium),
    );

    assert_eq!(element.as_widget().size().width, Length::Fill);
}
