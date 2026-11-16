use crate::app::{App, Message};
use iced::{
    Font, Rectangle, Renderer, Size, Theme,
    advanced::{
        renderer::{Headless, Style},
        widget::{Id, Operation},
    },
    mouse,
};
use iced_runtime::user_interface::{Cache, UserInterface};
#[derive(Default)]
struct Labels(Vec<String>);
impl Operation for Labels {
    fn traverse(&mut self, children: &mut dyn FnMut(&mut dyn Operation)) {
        children(self);
    }
    fn text(&mut self, _: Option<&Id>, _: Rectangle, value: &str) {
        self.0.push(value.into());
    }
}
fn draw(element: iced::Element<'_, Message>, theme: Theme) -> Vec<String> {
    let runtime = tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap();
    let mut renderer =
        runtime.block_on(Renderer::new(Font::DEFAULT, 14.0.into(), Some("tiny-skia"))).unwrap();
    {
        let mut fonts = iced::advanced::graphics::text::font_system().write().unwrap();
        for font in crate::fonts::load_all() {
            fonts.load_font(font);
        }
    }
    let mut ui =
        UserInterface::build(element, Size::new(1200., 3000.), Cache::new(), &mut renderer);
    ui.draw(
        &mut renderer,
        &theme,
        &Style { text_color: theme.palette().text },
        mouse::Cursor::Unavailable,
    );
    let mut labels = Labels::default();
    ui.operate(&renderer, &mut labels);
    labels.0
}
#[test]
fn cc_mcp_t48_settings_exposes_resync_progress() {
    let (_lock, _home, _guard) = crate::app::config::isolated_settings();
    let app = App::new().0;
    for theme in [Theme::Light, Theme::Dark] {
        let labels = draw(crate::app::components::system_settings_mcp::view(&app), theme);
        assert!(
            labels.iter().any(|text| text == "重新同步进度"),
            "missing native import confirmation entry: {labels:?}"
        );
    }
}
