use crate::input_panel::agent_children::{
    AgentChildSession, AgentChildSessionKind, AgentChildSessionStatus,
};
use crate::input_panel::agent_sessions::*;
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

#[test]
fn agent_sessions_popover_hides_without_running_children_and_counts_visible_items() {
    assert!(
        agent_sessions_entry(i18n(), 0, false, &[], std::convert::identity::<Action>).is_none()
    );
    assert_eq!(
        agent_sessions_badge_label(
            desktop_i18n::I18n::new(config_ui::DesktopLocale::SimplifiedChinese),
            1
        ),
        "1 个工作中"
    );
    assert_eq!(
        agent_sessions_badge_label(
            desktop_i18n::I18n::new(config_ui::DesktopLocale::SimplifiedChinese),
            3
        ),
        "3 个工作中"
    );
}

#[test]
fn agent_sessions_popover_lists_running_children() {
    let children = vec![
        child("child-writer", "writer", "实现输入面板工作中徽标", "增加入口并连接真实会话"),
        child(
            "child-reviewer",
            "reviewer",
            "审查超长会话标题是否会在固定宽度弹层中稳定截断",
            "验证浅色与暗色主题",
        ),
    ];

    let entry = agent_sessions_entry(i18n(), 0, true, &children, std::convert::identity::<Action>)
        .expect("running children should show entry");
    assert_eq!(entry.as_widget().children().len(), 2);

    let popover = agent_sessions_popover(i18n(), 0, &children, std::convert::identity::<Action>);
    assert!(!popover.as_widget().children().is_empty());
}

#[cfg(not(target_arch = "wasm32"))]
#[tokio::test(flavor = "multi_thread")]
async fn agent_sessions_popover_layouts_nested_non_ascii_text_in_light_and_dark_themes() {
    use iced::advanced::renderer::Headless;
    use iced::advanced::{layout, widget::Tree};
    use iced::{Font, Pixels, Size};

    let renderer =
        <iced::Renderer as Headless>::new(Font::DEFAULT, Pixels(16.0), Some("tiny-skia"))
            .await
            .expect("tiny-skia headless renderer should be available");

    let mut workflow = child("workflow-child", "子智能体", "递归工作流", "等待孙会话");
    workflow.kind = AgentChildSessionKind::WorkflowPlanning;
    workflow.title = "规划 · 019fb6ed-fc1c-7c21-8d82-c13454526594 · 计划任务".to_string();
    workflow.task.clear();
    workflow.task_path = Some("/home/user/.shiju-harness/storage/plan/task-render".to_string());
    workflow.status = AgentChildSessionStatus::Waiting;
    workflow.plan_run_task_id = Some("task-render".to_string());
    let mut nested = child("agent-grandchild", "reviewer", "检查中文动态文本", "等待递归后代完成");
    nested.status = AgentChildSessionStatus::Waiting;
    nested.depth = 1;
    let children = vec![workflow, nested];

    for theme in [Theme::Light, Theme::Dark] {
        let _ = agent_sessions_popover_style(&theme);
        let _ = agent_sessions_stop_style(&theme, iced::widget::button::Status::Active);
        let mut element =
            agent_sessions_popover(i18n(), 0, &children, std::convert::identity::<Action>);
        let mut tree = Tree::new(element.as_widget());
        let limits = layout::Limits::new(Size::ZERO, Size::new(400.0, 320.0));
        let node = element.as_widget_mut().layout(&mut tree, &renderer, &limits);
        assert!(node.bounds().width > 0.0);
        assert!(node.bounds().height > 0.0);
    }
}
