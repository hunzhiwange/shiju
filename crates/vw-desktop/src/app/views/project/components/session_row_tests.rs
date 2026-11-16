//! 宿主保留：SessionRuntimeState 的首轮请求和预览投影为 RowInput；中文绘制覆盖宿主标题与预览选择路径。
use super::*;
use iced::advanced::{Layout, layout, renderer::Headless, widget::Tree};
use iced::{Font, Pixels, Size, Theme, mouse};
use crate::app::{App, Message};
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn desktop_first_response_chinese_preview_and_title_draw_in_both_themes() {
    let (mut app, _) = App::new();
    let id = "draft";
    app.session_state.session_previews.insert(id.into(), "首轮预览".into());
    let mut details = SessionRowDetails {
        title: String::new(),
        directory: "/project".into(),
        updated: 0,
        project: "测试项目".into(),
        project_id: "project".into(),
        created: 0,
        changes: None,
        pending: false,
    };
    let size = Size::new(360.0, 40.0);
    for theme in [Theme::Light, Theme::Dark] {
        let mut renderer =
            <iced::Renderer as Headless>::new(Font::DEFAULT, Pixels(16.0), Some("tiny-skia"))
                .await
                .unwrap();
        let mut draw = |details: &SessionRowDetails, status| {
            let mut element = session_row(&app, id, details, status, Message::None);
            let mut tree = Tree::new(element.as_widget());
            let node = element.as_widget_mut().layout(
                &mut tree,
                &renderer,
                &layout::Limits::new(Size::ZERO, size),
            );
            element.as_widget().draw(
                &tree,
                &mut renderer,
                &theme,
                &iced::advanced::renderer::Style { text_color: theme.palette().text },
                Layout::new(&node),
                mouse::Cursor::Unavailable,
                &iced::Rectangle::with_size(size),
            );
            renderer.screenshot(Size::new(360, 40), 1.0, theme.palette().background)
        };
        details.pending = true;
        details.title = "首轮中文问题：修复对话标题".into();
        let running = draw(&details, SessionRowStatus::Running);
        details.pending = false;
        details.title.clear();
        let preview = draw(&details, SessionRowStatus::Idle);
        details.title = "服务端标题".into();
        let title = draw(&details, SessionRowStatus::Idle);
        assert_ne!(running, preview);
        assert_ne!(preview, title);
    }
}

#[test]
fn running_draft_title_uses_first_non_empty_input_line() {
    use crate::app::state::{AgentRequest, SessionDraft};
    let (mut app, _) = App::new();
    let runtime = app.get_session_runtime_mut("draft");
    runtime.draft = Some(SessionDraft {
        project_path: "/project".into(),
        directory: "/project".into(),
        history_len: 0,
    });
    runtime.request_lifecycle.is_requesting = true;
    runtime.request_lifecycle.active_agent_request = Some(std::sync::Arc::new(AgentRequest {
        id: 1,
        session: "draft".into(),
        query: " \r\n\t\n  首轮中文问题  \r\n补充说明".into(),
        root: None,
        model: None,
        reasoning_effort: None,
        acp_test: false,
        acp_model_source: Default::default(),
        acp_agent: None,
        acp_allowed_tools: None,
        agent: None,
        allowed_tools: None,
        require_agent_tool: false,
        acp_force_new_session: false,
        acp_history_mode: crate::app::state::AcpHistoryReplayMode::Discard,
        acp_recent_count: 0,
        full_access_enabled: Some(false),
        resume_history_only: false,
        workflow_mode_enabled: false,
        goal_mode_enabled: false,
        history: Vec::new(),
    }));
    let (_, info) = super::running_draft_sessions(&app).next().unwrap();
    assert_eq!(info.title, "首轮中文问题");
    assert!(info.is_first_response_pending());
    assert!(app.session_state.sessions.is_empty());
}