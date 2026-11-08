use super::super::custom_provider;
use super::view_overlays;
use crate::app::message::SettingsMessage;
use crate::app::state::{
    CustomProviderModelDraft, CustomProviderModelTab, CustomProviderTab, ModelCapabilityDefaults,
    ProviderHeaderDraft,
};
use crate::app::{App, Message};
use iced::advanced::renderer::{Headless, Style};
use iced::advanced::widget::operation::{Operation, Scrollable};
use iced::advanced::{Layout, clipboard, layout, widget::Tree};
use iced::keyboard::{self, Key, key::Named};
use iced::widget::button;
use iced::{Element, Event, Font, Length, Point, Rectangle, Renderer, Size, Theme, Vector, mouse};
use iced_runtime::user_interface::{Cache, UserInterface};
use vw_config_types::ui::DesktopLocale;

fn fixture() -> App {
    let mut app = None;
    crate::app::config::with_editor_settings_gateway(|fixture, _, _| app = Some(fixture));
    let mut app = app.expect("isolated settings fixture");
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

async fn renderer() -> Renderer {
    static FONTS: std::sync::Once = std::sync::Once::new();
    FONTS.call_once(|| {
        let mut fonts = iced::advanced::graphics::text::font_system().write().unwrap();
        for font in crate::fonts::load_all() {
            fonts.load_font(font);
        }
    });
    Renderer::new(Font::with_name("Noto Sans CJK SC"), 14.0.into(), Some("tiny-skia"))
        .await
        .expect("tiny-skia renderer")
}

fn view(app: &App) -> Element<'_, Message> {
    let background = button("Underlying settings")
        .width(Length::Fill)
        .height(Length::Fill)
        .on_press(Message::Settings(SettingsMessage::ProvidersRefresh));
    view_overlays(app, custom_provider::view_overlays(app, background.into()))
}

struct Scene<'a> {
    ui: UserInterface<'a, Message, Theme, Renderer>,
    layout: layout::Node,
    size: Size<u32>,
}

impl<'a> Scene<'a> {
    fn new(mut element: Element<'a, Message>, renderer: &mut Renderer, size: Size<u32>) -> Self {
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

    fn bounds(&self, path: &[usize]) -> Rectangle {
        path.iter()
            .fold(Layout::new(&self.layout), |node, &index| {
                node.children().nth(index).unwrap_or_else(|| panic!("missing layout {path:?}"))
            })
            .bounds()
    }

    fn send(
        &mut self,
        renderer: &mut Renderer,
        events: &[Event],
        cursor: mouse::Cursor,
    ) -> Vec<Message> {
        let mut messages = Vec::new();
        self.ui.update(events, cursor, renderer, &mut clipboard::Null, &mut messages);
        messages
    }

    fn capture(
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
        if let Some(directory) = std::env::var_os("SHIJU_SETTINGS_PREVIEW_DIR") {
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

fn screenshot(app: &App, renderer: &mut Renderer, theme: &Theme, size: Size<u32>, name: &str) {
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
                        Some(crate::app::state::CustomProviderModelModalState {
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
    let api_key = scene.bounds(&scalar_control(0, &[]));
    let value = scene.bounds(&scalar_control(0, &[0]));
    assert!(api_key.width >= 280.0, "the input needs the full field width at 420 px");
    assert!((api_key.center().y - value.center().y).abs() < 0.5);
    assert!(matches!(
        scene.edit(&mut renderer, api_key, "-edited").as_slice(),
        [Message::Settings(SettingsMessage::CustomProviderApiKeyChanged(key))]
            if key == "sk-fixture-edited"
    ));
    let name = scene.bounds(&control(2, &[1, 0]));
    let value = scene.bounds(&control(2, &[1, 1]));
    let remove = scene.bounds(&control(2, &[1, 2]));
    assert!(name.width > 110.0 && value.width > 110.0);
    assert!((name.center().y - remove.center().y).abs() < 0.5);
    assert!(matches!(
        scene.edit(&mut renderer, name, "-edited").as_slice(),
        [Message::Settings(SettingsMessage::CustomProviderHeaderKeyChanged(0, key))]
            if key == "x-fixture-0-edited"
    ));

    let body = scene.bounds(&modal(&[1]));
    let close = scene.bounds(&modal(&[0, 0, 1]));
    let cancel = scene.bounds(&modal(&[2, 1]));
    let save = scene.bounds(&modal(&[2, 2]));
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
    let id = scene.bounds(&scalar_control(0, &[]));
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

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn ui_refresh_09_preserves_values_and_disabled_states() {
    let mut renderer = renderer().await;
    let mut app = fixture();
    app.provider_settings.custom_provider_tab = CustomProviderTab::Models;
    app.provider_settings.custom.models[0].inherited_capabilities = Some(ModelCapabilityDefaults {
        reasoning: true,
        tool_call: true,
        input_modalities: vec!["text".into(), "image".into()],
        ..Default::default()
    });

    app.provider_settings.custom_models_discovering = true;
    let mut scene = narrow(&app, &mut renderer);
    let discover = scene.bounds(&control(0, &[1]));
    let save = scene.bounds(&modal(&[2, 2]));
    for button in [discover, save] {
        assert!(scene.click(&mut renderer, button.center()).is_empty());
    }
    scene.capture(&mut renderer, &Theme::Dark, cursor(discover), "provider-models-discovering");
    drop(scene);
    app.provider_settings.custom_models_discovering = false;
    let mut scene = narrow(&app, &mut renderer);
    let discover = scene.bounds(&control(0, &[1]));
    assert!(matches!(
        scene.click(&mut renderer, discover.center()).as_slice(),
        [Message::Settings(SettingsMessage::CustomProviderModelsDiscover)]
    ));
    let add = scene.bounds(&control(0, &[2]));
    let edit = scene.bounds(&group(&[4, 0, 2]));
    let filter = scene.bounds(&control(2, &[2]));
    let filtered = scene.click(&mut renderer, filter.center());
    assert!(matches!(
        filtered.as_slice(),
        [Message::Settings(SettingsMessage::CustomProviderModelFilterSelected(
            crate::app::state::CustomProviderModelFilter::Disabled
        ))]
    ));
    let add_messages = scene.click(&mut renderer, add.center());
    assert!(matches!(
        add_messages.as_slice(),
        [Message::Settings(SettingsMessage::CustomProviderModelOpen(None))]
    ));
    let body = scene.bounds(&modal(&[1]));
    let offset = scene.scroll(&mut renderer, body, 400.0);
    let edit_point = edit.center() - Vector::new(0.0, offset);
    assert!(body.contains(edit_point), "the model edit button must scroll into view");
    let hover = mouse::Cursor::Available(edit_point);
    let normal = scene.capture(
        &mut renderer,
        &Theme::Dark,
        mouse::Cursor::Unavailable,
        "provider-models-scrolled",
    );
    assert!(
        scene
            .send(
                &mut renderer,
                &[Event::Mouse(mouse::Event::CursorMoved { position: edit_point })],
                hover
            )
            .is_empty()
    );
    let hovered = scene.capture(&mut renderer, &Theme::Dark, hover, "provider-model-edit-tooltip");
    assert!(normal != hovered, "model actions must respond to hover after scrolling");
    let edit_messages = scene.click(&mut renderer, edit_point);
    assert!(matches!(
        edit_messages.as_slice(),
        [Message::Settings(SettingsMessage::CustomProviderModelOpen(Some(0)))]
    ));
    drop(scene);

    apply(&mut app, add_messages);
    let mut scene = narrow(&app, &mut renderer);
    let save = scene.bounds(&modal(&[2, 2]));
    let id = scene.bounds(&scalar_control(0, &[]));
    assert!(scene.click(&mut renderer, id.center()).is_empty());
    assert!(
        scene.send(&mut renderer, &[key(Named::Enter), key(Named::Escape)], cursor(id)).is_empty()
    );
    let invalid_save = scene.click(&mut renderer, save.center());
    drop(scene);
    apply(&mut app, invalid_save);
    assert!(app.provider_settings.custom_model_modal.is_some());
    assert!(app.provider_settings.save_error.is_some());
    let mut scene = narrow(&app, &mut renderer);
    scene.capture(&mut renderer, &Theme::Light, mouse::Cursor::Unavailable, "model-required-id");
    let id = scene.bounds(&scalar_control(0, &[]));
    let id_messages = scene.edit(&mut renderer, id, "fixture-added");
    assert!(matches!(
        id_messages.as_slice(),
        [Message::Settings(SettingsMessage::CustomProviderModelModalIdChanged(id))]
            if id == "fixture-added"
    ));
    drop(scene);
    apply(&mut app, id_messages);
    save_model(&mut app, &mut renderer);
    assert_eq!(app.provider_settings.custom.models[1].model_id, "fixture-added");
    assert!(app.provider_settings.custom.models[1].tool_call.is_none());

    apply(&mut app, edit_messages);
    let mut scene = narrow(&app, &mut renderer);
    let capability_tab = scene.bounds(&modal(&[0, 1, 2]));
    let messages = scene.click(&mut renderer, capability_tab.center());
    drop(scene);
    apply(&mut app, messages);
    assert_eq!(
        app.provider_settings.custom_model_modal.as_ref().unwrap().active_tab,
        CustomProviderModelTab::Capabilities
    );
    let mut scene = narrow(&app, &mut renderer);
    let reasoning = scene.bounds(&control(0, &[0]));
    let last = scene.bounds(&control(0, &[4]));
    let options = scene.bounds(&control(0, &[]));
    assert!(last.y > reasoning.y && last.x + last.width <= options.x + options.width);
    let hover = Event::Mouse(mouse::Event::CursorMoved { position: last.center() });
    assert!(scene.send(&mut renderer, &[hover], cursor(last)).is_empty());
    scene.capture(&mut renderer, &Theme::Dark, cursor(last), "model-capability-hover");
    let messages = scene.click(&mut renderer, reasoning.center());
    assert!(matches!(
        messages.as_slice(),
        [Message::Settings(SettingsMessage::CustomProviderModelModalReasoningChanged(false))]
    ));
    let image = scene.bounds(&group(&[2, 0, 0, 1, 0, 1]));
    let modalities = scene.click(&mut renderer, image.center());
    assert!(matches!(
        modalities.as_slice(),
        [Message::Settings(SettingsMessage::CustomProviderModelModalInputModalityToggled(modality, false))]
            if modality == "image"
    ));
    let limits = scene.bounds(&modal(&[0, 1, 1]));
    let tab_messages = scene.click(&mut renderer, limits.center());
    drop(scene);
    apply(&mut app, messages);
    apply(&mut app, modalities);
    apply(&mut app, tab_messages);

    let mut scene = narrow(&app, &mut renderer);
    let limit = scene.bounds(&scalar_control(0, &[]));
    let messages = scene.edit(&mut renderer, limit, "32768");
    assert!(matches!(
        messages.as_slice(),
        [Message::Settings(SettingsMessage::CustomProviderModelModalContextLimitChanged(value))]
            if value == "32768"
    ));
    drop(scene);
    apply(&mut app, messages);
    save_model(&mut app, &mut renderer);
    let saved = &app.provider_settings.custom.models[0];
    assert!(saved.model_id.starts_with("fixture-model-with-"));
    assert_eq!(saved.context_limit, "32768");
    assert_eq!(saved.reasoning, Some(false));
    assert!(saved.tool_call.is_none() && saved.input_limit.is_empty());
    assert_eq!(saved.input_modalities.as_deref().unwrap(), ["text"]);
    assert!(saved.output_modalities.is_none());
    assert!(saved.inherited_capabilities.as_ref().unwrap().tool_call);

    let _ = crate::app::message::settings::providers::update(
        &mut app,
        SettingsMessage::CustomProviderModelOpen(Some(0)),
    );
    let mut scene = narrow(&app, &mut renderer);
    let cancel = scene.bounds(&modal(&[2, 1]));
    let messages = scene.click(&mut renderer, cancel.center());
    assert!(matches!(
        messages.as_slice(),
        [Message::Settings(SettingsMessage::CustomProviderModelClose)]
    ));
    drop(scene);
    apply(&mut app, messages);
    assert!(app.provider_settings.custom_provider_modal_open);
    assert!(app.provider_settings.custom_model_modal.is_none());

    apply(&mut app, filtered);
    app.provider_settings.custom_editing_provider_id = Some("fixture".into());
    screenshot(&app, &mut renderer, &Theme::Light, Size::new(420, 620), "provider-models-empty");
    let _ = crate::app::message::settings::providers::update(
        &mut app,
        SettingsMessage::CustomProviderSaveDone(Err("Fixture save failed".into())),
    );
    let mut scene = narrow(&app, &mut renderer);
    let save = scene.bounds(&modal(&[2, 2]));
    scene.capture(&mut renderer, &Theme::Dark, mouse::Cursor::Unavailable, "provider-save-failed");
    assert!(matches!(
        scene.click(&mut renderer, save.center()).as_slice(),
        [Message::Settings(SettingsMessage::CustomProviderSave)]
    ));
}

fn save_model(app: &mut App, renderer: &mut Renderer) {
    let mut scene = narrow(app, renderer);
    let save = scene.bounds(&modal(&[2, 2]));
    let messages = scene.click(renderer, save.center());
    assert!(matches!(
        messages.as_slice(),
        [Message::Settings(SettingsMessage::CustomProviderModelModalSave)]
    ));
    drop(scene);
    apply(app, messages);
    assert!(
        app.provider_settings.custom_model_modal.is_none(),
        "{:?}",
        app.provider_settings.save_error
    );
}

fn apply(app: &mut App, messages: Vec<Message>) {
    assert_eq!(messages.len(), 1, "expected one settings message");
    for message in messages {
        let Message::Settings(message) = message else { panic!("expected a settings message") };
        // Only the synchronous draft updates are needed for the fixture.
        let _ = crate::app::message::settings::providers::update(app, message);
    }
}

fn narrow<'a>(app: &'a App, renderer: &mut Renderer) -> Scene<'a> {
    Scene::new(view(app), renderer, Size::new(420, 620))
}

fn modal(path: &[usize]) -> Vec<usize> {
    // Stack, active layer, centering container, inset, card, contents.
    [&[1, 1, 0, 0, 0][..], path].concat()
}

fn group(path: &[usize]) -> Vec<usize> {
    modal(&[&[1, 0, 0, 1, 0][..], path].concat())
}

fn control(field: usize, path: &[usize]) -> Vec<usize> {
    group(&[&[field, 0, 1, 0][..], path].concat())
}

fn scalar_control(field: usize, path: &[usize]) -> Vec<usize> {
    group(&[&[field, 0, 1][..], path].concat())
}

fn cursor(bounds: Rectangle) -> mouse::Cursor {
    mouse::Cursor::Available(bounds.center())
}

impl Scene<'_> {
    fn scroll(&mut self, renderer: &mut Renderer, body: Rectangle, distance: f32) -> f32 {
        let wheel = Event::Mouse(mouse::Event::WheelScrolled {
            delta: mouse::ScrollDelta::Pixels { x: 0.0, y: -distance },
        });
        assert!(self.send(renderer, &[wheel], cursor(body)).is_empty());
        let mut offset = ScrollOffset::default();
        self.ui.operate(renderer, &mut offset);
        offset.0
    }

    fn click(&mut self, renderer: &mut Renderer, point: Point) -> Vec<Message> {
        self.send(
            renderer,
            &[
                Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)),
                Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)),
            ],
            mouse::Cursor::Available(point),
        )
    }

    fn edit(&mut self, renderer: &mut Renderer, bounds: Rectangle, value: &str) -> Vec<Message> {
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

fn key(key: Named) -> Event {
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

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn settings_tabs_editors_appearance_model() {
    use crate::app::components::system_settings_common::tab_appearance_tests;
    use crate::app::i18n::settings_models::connection_text::text as tr;

    fn active(app: &App) -> Element<'_, Message> {
        let background =
            iced::widget::Space::new().width(iced::Length::Fill).height(iced::Length::Fill);
        view_overlays(app, background.into())
    }

    tab_appearance_tests::run_at_least(
        concat!(module_path!(), "::settings_tabs_editors_appearance_model"),
        420,
        || App::new().0,
        |scene, app| {
            apply(app, vec![Message::Settings(SettingsMessage::CustomProviderOpen)]);
            apply(app, vec![Message::Settings(SettingsMessage::CustomProviderModelOpen(None))]);
            let name = "模型草稿 · Model draft";
            apply(
                app,
                vec![Message::Settings(SettingsMessage::CustomProviderModelModalNameChanged(
                    name.into(),
                ))],
            );
            let [info, limits, capabilities, title, save, cancel] =
                ["模型信息", "限制", "能力与模态", "添加模型", "保存", "取消"]
                    .map(|label| tr(app.desktop_locale, label));
            scene.save_preview(app, view(app), "model");
            let message = scene.check(app, view(app), &[info, limits, capabilities], 1);
            assert!(matches!(
                message,
                SettingsMessage::CustomProviderModelTabSelected(CustomProviderModelTab::Limits)
            ));
            apply(app, vec![Message::Settings(message)]);
            assert_eq!(
                app.provider_settings.custom_model_modal.as_ref().unwrap().active_tab,
                CustomProviderModelTab::Limits
            );

            for (label, tab) in [
                (capabilities, CustomProviderModelTab::Capabilities),
                (info, CustomProviderModelTab::Info),
            ] {
                let message = scene.click(view(app), label);
                apply(app, vec![Message::Settings(message)]);
                let draft = app.provider_settings.custom_model_modal.as_ref().unwrap();
                assert_eq!(draft.active_tab, tab);
                assert_eq!(draft.display_name, name);
            }
            // The overlay below the model belongs to the provider. Inspect only the active
            // card for footer actions and scroll bounds, then exercise cancel in the stack.
            scene.check_modal_scroll(active(app), title, save);
            let message = scene.click(active(app), cancel);
            assert!(matches!(message, SettingsMessage::CustomProviderModelClose));
            apply(app, vec![Message::Settings(message)]);
            assert!(app.provider_settings.custom_model_modal.is_none());
            assert!(app.provider_settings.custom_provider_modal_open);
        },
    )
    .await;
}
