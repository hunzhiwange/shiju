use super::support::custom_provider;
use super::support::model_overlay as view_overlays;
use crate::provider_state::{CustomProviderModelDraft, CustomProviderTab, ProviderHeaderDraft};
use crate::provider_view_test_support::Fixture as App;
use crate::state::CustomProviderModelTab;
use crate::view_message::Message;
use crate::view_message::SettingsMessage;
use config_ui::DesktopLocale;
use iced::advanced::renderer::{Headless, Style};
use iced::advanced::widget::operation::{Operation, Scrollable};
use iced::advanced::{Layout, clipboard, layout, widget::Tree};
use iced::keyboard::{self, Key, key::Named};
use iced::widget::button;
use iced::{Element, Event, Font, Length, Point, Rectangle, Renderer, Size, Theme, Vector, mouse};
use iced_runtime::user_interface::{Cache, UserInterface};

pub(crate) fn fixture() -> App {
    let mut app = App::new().0;
    app.desktop_locale = DesktopLocale::English;
    let settings = &mut app.provider_settings;
    settings.custom_provider_modal_open = true;
    settings.custom.provider_id = "fixture".into();
    settings.custom.display_name = "Fixture 測試供应商".into();
    settings.custom.base_url =
        format!("https://fixture.invalid/{}/v1", "long-fixture-route/".repeat(12));
    settings.custom.api_key = "sk-fixture".into();
    settings.custom.headers = (0..18)
        .map(|i| ProviderHeaderDraft { key: format!("x-fixture-{i}"), value: format!("value-{i}") })
        .collect();
    settings.custom.models = vec![CustomProviderModelDraft {
        model_id: "fixture-model-with-a-very-long-id-for-testing-narrow-layout".into(),
        display_name: "Fixture 虚构模型 測試模型 with a long display name".into(),
        enabled: true,
        ..Default::default()
    }];
    app
}

pub(crate) async fn renderer() -> Renderer {
    static FONTS: std::sync::Once = std::sync::Once::new();
    FONTS.call_once(|| {
        let mut fonts = iced::advanced::graphics::text::font_system().write().unwrap();
        for font in desktop_assets::fonts::load_all() {
            fonts.load_font(font);
        }
    });
    Renderer::new(Font::with_name("Noto Sans CJK SC"), 14.0.into(), Some("tiny-skia"))
        .await
        .expect("tiny-skia renderer")
}

pub(crate) fn view(app: &App) -> Element<'_, Message> {
    let background = button("Underlying settings")
        .width(Length::Fill)
        .height(Length::Fill)
        .on_press(Message::Settings(SettingsMessage::ProvidersRefresh));
    view_overlays(app, custom_provider::view_overlays(app, background.into()))
}

pub(crate) struct Scene<'a> {
    pub(crate) ui: UserInterface<'a, Message, Theme, Renderer>,
    layout: layout::Node,
    size: Size<u32>,
}

impl<'a> Scene<'a> {
    pub(crate) fn new(
        mut element: Element<'a, Message>,
        renderer: &mut Renderer,
        size: Size<u32>,
    ) -> Self {
        let bounds = Size::new(size.width as f32, size.height as f32);
        let mut tree = Tree::new(element.as_widget());
        let node = element.as_widget_mut().layout(
            &mut tree,
            renderer,
            &layout::Limits::new(Size::ZERO, bounds),
        );
        let ui = UserInterface::build(element, bounds, Cache::new(), renderer);
        Self { ui, layout: node, size }
    }

    pub(crate) fn bounds(&self, path: &[usize]) -> Rectangle {
        path.iter()
            .fold(Layout::new(&self.layout), |node, &index| {
                node.children().nth(index).unwrap_or_else(|| panic!("missing layout {path:?}"))
            })
            .bounds()
    }

    pub(crate) fn send(
        &mut self,
        renderer: &mut Renderer,
        events: &[Event],
        cursor: mouse::Cursor,
    ) -> Vec<Message> {
        let mut messages = Vec::new();
        self.ui.update(events, cursor, renderer, &mut clipboard::Null, &mut messages);
        messages
    }

    pub(crate) fn capture(
        &mut self,
        renderer: &mut Renderer,
        theme: &Theme,
        cursor: mouse::Cursor,
        name: &str,
    ) -> Vec<u8> {
        assert!(
            self.send(
                renderer,
                &[Event::Window(iced::window::Event::RedrawRequested(iced::time::Instant::now()))],
                cursor,
            )
            .is_empty()
        );
        self.ui.draw(renderer, theme, &Style { text_color: theme.palette().text }, cursor);
        let pixels = renderer.screenshot(self.size, 1.0, theme.palette().background);
        if let Some(directory) =
            std::env::var_os("SHIJU_SETTINGS_PREVIEW_DIR").filter(|_| !name.is_empty())
        {
            image::save_buffer(
                std::path::PathBuf::from(directory).join(format!("{name}.png")),
                &pixels,
                self.size.width,
                self.size.height,
                image::ColorType::Rgba8,
            )
            .unwrap();
        }
        pixels
    }
}

pub(crate) fn screenshot(
    app: &App,
    renderer: &mut Renderer,
    theme: &Theme,
    size: Size<u32>,
    name: &str,
) {
    let mut scene = Scene::new(view(app), renderer, size);
    scene.capture(renderer, theme, mouse::Cursor::Unavailable, name);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn ui_refresh_09_layout_and_interaction() {
    let mut renderer = renderer().await;
    let mut app = fixture();
    for (locale, name) in [
        (DesktopLocale::SimplifiedChinese, "zh"),
        (DesktopLocale::TraditionalChinese, "zh-hant"),
        (DesktopLocale::English, "en"),
    ] {
        app.desktop_locale = locale;
        for (theme, mode) in [(Theme::Light, "light"), (Theme::Dark, "dark")] {
            for width in [420, 640, 960] {
                let size = Size::new(width, 620);
                for (tab, tab_name) in [
                    (CustomProviderTab::Basic, "basic"),
                    (CustomProviderTab::Authentication, "auth"),
                    (CustomProviderTab::Models, "models"),
                ] {
                    app.provider_settings.custom_provider_tab = tab;
                    screenshot(
                        &app,
                        &mut renderer,
                        &theme,
                        size,
                        &format!("provider-{tab_name}-{name}-{mode}-{width}"),
                    );
                }
                for (tab, tab_name) in [
                    (CustomProviderModelTab::Info, "info"),
                    (CustomProviderModelTab::Limits, "limits"),
                    (CustomProviderModelTab::Capabilities, "capabilities"),
                ] {
                    app.provider_settings.custom_model_modal =
                        Some(crate::state::CustomProviderModelModalState {
                            active_tab: tab,
                            model_id: "fixture-model".into(),
                            context_limit: "32768".into(),
                            ..Default::default()
                        });
                    screenshot(
                        &app,
                        &mut renderer,
                        &theme,
                        size,
                        &format!("model-{tab_name}-{name}-{mode}-{width}"),
                    );
                }
                app.provider_settings.custom_model_modal = None;
            }
        }
    }

    app.desktop_locale = DesktopLocale::English;
    app.provider_settings.custom_provider_tab = CustomProviderTab::Authentication;
    let mut scene = narrow(&app, &mut renderer);
    let api_key = scene.bounds(&scalar_control(0, &[0]));
    let optional = scene.bounds(&scalar_control(0, &[1]));
    assert!(api_key.width >= 280.0, "the input needs the full field width at 420 px");
    assert!(optional.y >= api_key.y + api_key.height, "the hint stays below the input");
    assert!(matches!(
        scene.edit(&mut renderer, api_key, "-edited").as_slice(),
        [Message::Settings(SettingsMessage::CustomProviderApiKeyChanged(key))]
            if key == "sk-fixture-edited"
    ));
    let name = scene.bounds(&scalar_control(2, &[1, 0]));
    let value = scene.bounds(&scalar_control(2, &[1, 1]));
    let remove = scene.bounds(&scalar_control(2, &[1, 2]));
    assert!(name.width > 110.0 && value.width > 110.0);
    assert!((name.center().y - remove.center().y).abs() < 0.5);
    assert!(matches!(
        scene.edit(&mut renderer, name, "-edited").as_slice(),
        [Message::Settings(SettingsMessage::CustomProviderHeaderKeyChanged(0, key))]
            if key == "x-fixture-0-edited"
    ));

    let body = scene.bounds(&modal(&[1]));
    let close = scene.bounds(&modal(&[0, 0, 1]));
    let cancel = scene.bounds(&modal(&[2, 0]));
    let save = scene.bounds(&modal(&[2, 1]));
    assert!(scene.scroll(&mut renderer, body, 230.0) > 150.0);

    let before = scene.capture(
        &mut renderer,
        &Theme::Dark,
        mouse::Cursor::Unavailable,
        "provider-auth-scrolled",
    );
    let outside = mouse::Cursor::Available(Point::new(2.0, 2.0));
    assert!(
        scene
            .send(
                &mut renderer,
                &[Event::Mouse(mouse::Event::CursorMoved { position: Point::new(2.0, 2.0) })],
                outside
            )
            .is_empty()
    );
    let after = scene.capture(&mut renderer, &Theme::Dark, outside, "provider-auth-backdrop-hover");
    assert!(before == after, "the modal backdrop must suppress hover on the settings page");
    assert!(matches!(
        scene.click(&mut renderer, cancel.center()).as_slice(),
        [Message::Settings(SettingsMessage::CustomProviderClose)]
    ));
    assert!(matches!(
        scene.click(&mut renderer, close.center()).as_slice(),
        [Message::Settings(SettingsMessage::CustomProviderClose)]
    ));
    assert!(matches!(
        scene.click(&mut renderer, save.center()).as_slice(),
        [Message::Settings(SettingsMessage::CustomProviderSave)]
    ));
    drop(scene);

    app.provider_settings.custom_provider_tab = CustomProviderTab::Basic;
    let mut scene = narrow(&app, &mut renderer);
    let id = scene.bounds(&scalar_control(0, &[0]));
    assert!(matches!(
        scene.edit(&mut renderer, id, "-new").as_slice(),
        [Message::Settings(SettingsMessage::CustomProviderIdChanged(id))] if id == "fixture-new"
    ));
    drop(scene);

    app.provider_settings.custom_editing_provider_id = Some("fixture".into());
    let mut scene = narrow(&app, &mut renderer);
    let id = scene.bounds(&scalar_control(0, &[]));
    assert!(scene.edit(&mut renderer, id, "-ignored").is_empty());
    scene.capture(
        &mut renderer,
        &Theme::Light,
        mouse::Cursor::Unavailable,
        "provider-read-only-id",
    );
    drop(scene);

    app.provider_settings.custom.provider_id = "shiju".into();
    app.provider_settings.custom_provider_tab = CustomProviderTab::Authentication;
    screenshot(&app, &mut renderer, &Theme::Dark, Size::new(420, 620), "provider-account-auth");
}
pub(crate) fn narrow<'a>(app: &'a App, renderer: &mut Renderer) -> Scene<'a> {
    Scene::new(view(app), renderer, Size::new(420, 620))
}

pub(crate) fn modal(path: &[usize]) -> Vec<usize> {
    // Stack, active layer, centering container, inset, card, contents.
    [&[1, 1, 0, 0][..], path].concat()
}

pub(crate) fn group(path: &[usize]) -> Vec<usize> {
    modal(&[&[1, 0, 0, 1, 0][..], path].concat())
}

pub(crate) fn scalar_control(field: usize, path: &[usize]) -> Vec<usize> {
    group(&[&[field, 0, 1][..], path].concat())
}

pub(crate) fn cursor(bounds: Rectangle) -> mouse::Cursor {
    mouse::Cursor::Available(bounds.center())
}

impl Scene<'_> {
    pub(crate) fn scroll(
        &mut self,
        renderer: &mut Renderer,
        body: Rectangle,
        distance: f32,
    ) -> f32 {
        let wheel = Event::Mouse(mouse::Event::WheelScrolled {
            delta: mouse::ScrollDelta::Pixels { x: 0.0, y: -distance },
        });
        assert!(self.send(renderer, &[wheel], cursor(body)).is_empty());
        let mut offset = ScrollOffset::default();
        self.ui.operate(renderer, &mut offset);
        offset.0
    }

    pub(crate) fn click(&mut self, renderer: &mut Renderer, point: Point) -> Vec<Message> {
        self.send(
            renderer,
            &[
                Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)),
                Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)),
            ],
            mouse::Cursor::Available(point),
        )
    }

    pub(crate) fn edit(
        &mut self,
        renderer: &mut Renderer,
        bounds: Rectangle,
        value: &str,
    ) -> Vec<Message> {
        assert!(self.click(renderer, bounds.center()).is_empty());
        self.send(
            renderer,
            &[
                key(Named::End),
                Event::InputMethod(iced::advanced::input_method::Event::Commit(value.into())),
            ],
            cursor(bounds),
        )
    }
}

#[derive(Default)]
struct ScrollOffset(f32);

impl Operation for ScrollOffset {
    fn traverse(&mut self, operate: &mut dyn FnMut(&mut dyn Operation)) {
        operate(self);
    }

    fn scrollable(
        &mut self,
        _id: Option<&iced::advanced::widget::Id>,
        _bounds: Rectangle,
        _content: Rectangle,
        translation: Vector,
        _state: &mut dyn Scrollable,
    ) {
        self.0 = translation.y;
    }
}

pub(crate) fn key(key: Named) -> Event {
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
