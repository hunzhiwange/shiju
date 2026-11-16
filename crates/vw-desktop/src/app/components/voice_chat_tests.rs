use super::*;
#[derive(Default)]
struct Labels(Vec<(String, iced::Rectangle)>);
impl iced::advanced::widget::Operation for Labels {
    fn traverse(&mut self, children: &mut dyn FnMut(&mut dyn iced::advanced::widget::Operation)) {
        children(self);
    }
    fn text(
        &mut self,
        _: Option<&iced::advanced::widget::Id>,
        bounds: iced::Rectangle,
        value: &str,
    ) {
        self.0.push((value.into(), bounds));
    }
}
#[tokio::test]
async fn voice_moon_ui_local_keeps_chat_without_modal() {
    use iced::advanced::renderer::Headless;
    use iced_runtime::user_interface::{Cache, UserInterface};
    let (mut app, _) = App::new();
    app.navigation_tabs.project_path = Some("/repo".into());
    app.active_session_id = Some("moon-ui".into());
    app.current_session_runtime_mut().acp_agent = None;
    let _ = crate::app::message::voice::update(&mut app, Event::ChatStart);
    let mut renderer =
        iced::Renderer::new(iced::Font::DEFAULT, 14.0.into(), Some("tiny-skia")).await.unwrap();
    let mut ui = UserInterface::build(
        with_voice_chat(&app, text("聊天正文").into()),
        iced::Size::new(700.0, 640.0),
        Cache::new(),
        &mut renderer,
    );
    let mut labels = Labels::default();
    ui.operate(&renderer, &mut labels);
    assert!(labels.0.iter().any(|(s, _)| s == "聊天正文"));
    assert!(!labels.0.iter().any(|(s, _)| s == "本机逐轮语音"));
}
