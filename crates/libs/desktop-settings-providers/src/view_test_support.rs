#![allow(dead_code)]
use iced::advanced::renderer::Headless;
use iced::advanced::{Layout, clipboard, layout, widget::Tree};
use iced::keyboard::{self, Key, key::Named};
use iced::{Element, Event, Font, Point, Rectangle, Renderer, Size, Theme, mouse};
use iced_runtime::user_interface::{Cache, UserInterface};
pub async fn renderer() -> Renderer {
    static FONTS: std::sync::Once = std::sync::Once::new();
    FONTS.call_once(|| {
        let mut fonts = iced::advanced::graphics::text::font_system().write().unwrap();
        for font in desktop_assets::fonts::load_all() {
            fonts.load_font(font);
        }
    });
    Renderer::new(Font::with_name("Noto Sans CJK SC"), 14.0.into(), Some("tiny-skia"))
        .await
        .unwrap()
}

// Stack → active layer → centering container → inset → card → contents.
pub fn modal_path(path: &[usize]) -> Vec<usize> {
    [&[1, 1, 0, 0, 0][..], path].concat()
}

pub struct Scene<'a, Message> {
    ui: UserInterface<'a, Message, Theme, Renderer>,
    layout: layout::Node,
}

impl<'a, Message: Clone + 'static> Scene<'a, Message> {
    pub fn new(mut element: Element<'a, Message>, renderer: &mut Renderer, size: Size) -> Self {
        let mut tree = Tree::new(element.as_widget());
        let layout = element.as_widget_mut().layout(
            &mut tree,
            renderer,
            &layout::Limits::new(Size::ZERO, size),
        );
        let ui = UserInterface::build(element, size, Cache::new(), renderer);
        Self { ui, layout }
    }

    pub fn bounds(&self, path: &[usize]) -> Rectangle {
        path.iter()
            .fold(Layout::new(&self.layout), |layout, &index| {
                layout
                    .children()
                    .nth(index)
                    .unwrap_or_else(|| panic!("missing layout path {path:?}"))
            })
            .bounds()
    }

    pub fn send(
        &mut self,
        renderer: &mut Renderer,
        events: &[Event],
        point: Point,
    ) -> Vec<Message> {
        let mut messages = Vec::new();
        self.ui.update(
            events,
            mouse::Cursor::Available(point),
            renderer,
            &mut clipboard::Null,
            &mut messages,
        );
        messages
    }

    pub fn click(&mut self, renderer: &mut Renderer, point: Point) -> Vec<Message> {
        self.send(
            renderer,
            &[
                Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)),
                Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)),
            ],
            point,
        )
    }
}

pub fn key(key: Named) -> Event {
    let key = Key::Named(key);
    Event::Keyboard(keyboard::Event::KeyPressed {
        modified_key: key.clone(),
        key,
        physical_key: keyboard::key::Physical::Unidentified(
            keyboard::key::NativeCode::Unidentified,
        ),
        location: keyboard::Location::Standard,
        modifiers: keyboard::Modifiers::empty(),
        text: None,
        repeat: false,
    })
}

pub fn ime(text: &str) -> Event {
    Event::InputMethod(iced::advanced::input_method::Event::Commit(text.into()))
}

use desktop_settings_providers::provider_state::{
    ModelCatalogEntry, ProviderSettingsState, ProviderSummary,
};
pub fn seed_catalog(s: &mut ProviderSettingsState) {
    s.catalog_items = (0..14)
        .map(|i| ModelCatalogEntry {
            provider_id: format!("fixture-{i:02}-with-a-long-provider-identifier"),
            provider_name: format!("Fixture {i:02} 虚构模型供應商 — A long provider display name"),
            model_id: "fixture-model".into(),
            model_name: "Fixture model".into(),
        })
        .collect();
    s.providers = vec![ProviderSummary {
        id: s.catalog_items[0].provider_id.clone(),
        name: s.catalog_items[0].provider_name.clone(),
        source_label: "Fixture source".into(),
        connected: true,
    }];
    s.popular_patterns = vec![s.providers[0].id.clone(), "missing-fixture".into()];
}
