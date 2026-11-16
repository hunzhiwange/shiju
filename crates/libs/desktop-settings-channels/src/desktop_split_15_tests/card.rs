#![cfg(not(target_arch = "wasm32"))]
use crate::{messages::Message, overview::*, state::ChannelsSettingsTab};
use config_ui::DesktopLocale;
use iced::advanced::{
    renderer::Headless,
    widget::{
        Id, Operation,
        operation::{Focusable, TextInput},
    },
};
use iced::widget::text;
use iced::{Element, Font, Rectangle, Renderer, Size, Theme};
use iced_runtime::user_interface::{Cache, UserInterface};
async fn renderer() -> Renderer {
    {
        let mut fonts = iced::advanced::graphics::text::font_system().write().unwrap();
        for font in desktop_assets::fonts::load_all() {
            fonts.load_font(font);
        }
    }
    Renderer::new(Font::with_name("Noto Sans CJK SC"), 14.0.into(), Some("tiny-skia"))
        .await
        .unwrap()
}

fn layout(
    element: &mut Element<'_, Message>,
    renderer: &Renderer,
    size: Size,
) -> iced::advanced::layout::Node {
    let mut tree = iced::advanced::widget::Tree::new(element.as_widget());
    element.as_widget_mut().layout(
        &mut tree,
        renderer,
        &iced::advanced::layout::Limits::new(Size::ZERO, size),
    )
}

#[derive(Default)]
struct Geometry {
    labels: Vec<(String, Rectangle)>,
    inputs: Vec<(String, Rectangle)>,
    focusable: Vec<Rectangle>,
}

impl Operation for Geometry {
    fn traverse(&mut self, children: &mut dyn FnMut(&mut dyn Operation)) {
        children(self);
    }

    fn text(&mut self, _: Option<&Id>, bounds: Rectangle, text: &str) {
        self.labels.push((text.into(), bounds));
    }

    fn text_input(&mut self, _: Option<&Id>, bounds: Rectangle, state: &mut dyn TextInput) {
        self.inputs.push((state.text().into(), bounds));
    }

    fn focusable(&mut self, _: Option<&Id>, bounds: Rectangle, _: &mut dyn Focusable) {
        self.focusable.push(bounds);
    }
}

impl Geometry {
    fn label(&self, name: &str) -> Rectangle {
        self.labels
            .iter()
            .find(|(label, _)| label == name)
            .unwrap_or_else(|| panic!("missing {name} in {:?}", self.labels))
            .1
    }
}

fn geometry(ui: &mut UserInterface<'_, Message, Theme, Renderer>, renderer: &Renderer) -> Geometry {
    let mut geometry = Geometry::default();
    ui.operate(renderer, &mut geometry);
    geometry
}

#[tokio::test]
async fn ui_refresh_17_channel_card_content_fits_and_is_vertically_centered() {
    let mut renderer = renderer().await;
    let descriptor = channel_tab_descriptors()
        .into_iter()
        .find(|descriptor| descriptor.tab == ChannelsSettingsTab::DingTalk)
        .unwrap();
    let mut card =
        china_channel_card(DesktopLocale::English, &descriptor, ChannelsSettingsTab::Global);
    // 640px content, page gutters and two gaps between three cards.
    let width = (640.0 - 24.0 - 24.0) / 3.0;
    let node = layout(&mut card, &renderer, Size::new(width, 120.0));
    let card_layout = iced::advanced::Layout::new(&node);
    let content = card_layout.children().next().unwrap();
    for item in content.children() {
        assert!((item.bounds().center_y() - card_layout.bounds().center_y()).abs() < 0.5);
    }
    let mut ui = UserInterface::build(card, Size::new(width, 120.0), Cache::new(), &mut renderer);
    let geometry = geometry(&mut ui, &renderer);
    for (label, text_size) in [("DingTalk", 15), ("Configuration", 12)] {
        let bounds = geometry.label(label);
        let text_width =
            layout(&mut text(label).size(text_size).into(), &renderer, Size::new(640.0, 120.0))
                .size()
                .width;
        assert!(bounds.x + text_width <= node.size().width - 8.0, "{label}: {bounds:?}");
    }
}
