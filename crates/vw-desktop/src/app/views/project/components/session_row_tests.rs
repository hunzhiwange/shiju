use super::{SessionRowDetails, SessionRowStatus, session_row};
use crate::app::{App, Message};
use iced::advanced::{Layout, layout, renderer::Headless, widget::Tree};
use iced::{Font, Pixels, Size, Theme, mouse};

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn desktop_first_response_chinese_preview_and_title_draw_in_both_themes() {
    let (mut app, _) = App::new();
    let id = "draft";
    app.session_previews.insert(id.into(), "首轮预览".into());
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
        details.title = "新会话".into();
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
