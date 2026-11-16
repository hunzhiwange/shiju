// 宿主契约：可选择帮助菜单的 CopyCode/OpenUrlExternal 映射，以及提交校验错误投影。
// 列表、目录、连接弹窗的局部渲染迁至 desktop_split_11_tests::providers。
use super::{catalog, connect, connected, custom_provider};
use crate::app::message::SettingsMessage;
use crate::app::state::{ModelCatalogEntry, ProviderConnectState, ProviderSummary};
use crate::app::{App, Message};
use iced::advanced::renderer::Headless;
use iced::advanced::{Layout, clipboard, layout, widget::Tree};
use iced::keyboard::{self, Key, key::Named};
use iced::{Element, Event, Font, Point, Rectangle, Renderer, Size, Theme, mouse};
use iced_runtime::user_interface::{Cache, UserInterface};
use config_ui::DesktopLocale;

fn fixture() -> App {
    let mut app = None;
    crate::app::config::with_editor_settings_gateway(|fixture, _, _| app = Some(fixture));
    let mut app = app.expect("isolated settings fixture");
    app.desktop_locale = DesktopLocale::English;
    let s = &mut app.provider_settings;
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
        .unwrap()
}



#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn help_rollout_05_keeps_credentials_and_validation_outside_help() {
    use crate::app::i18n::settings_models::connection_text::text as tr;
    use crate::app::message::ViewMessage;

    let mut renderer = renderer().await;
    let mut app = fixture();
    app.app_theme = Theme::Light;
    app.provider_settings.connect_modal = Some(ProviderConnectState {
        provider_id: "fixture-private-id".into(),
        provider_name: "Fixture Private Provider".into(),
        api_key: "fixture-private-key".into(),
    });

    let mut scene = Scene::new(
        connect::view_overlays(&app, background()),
        &mut renderer,
        Size::new(700.0, 620.0),
    );
    let icon = scene.bounds(&modal_path(&[1, 0, 0, 0, 0, 1]));
    assert!(scene.click(&mut renderer, icon.center()).is_empty());
    let card_text = Point::new(icon.x + icon.width + 4.0 + 18.0, icon.y + 16.0);
    let menu_row_height =
        f32::from(iced::advanced::text::LineHeight::default().to_absolute(12.0.into())) + 10.0;
    let menu_item = |row: f32| {
        Point::new(card_text.x + 24.0, card_text.y + 3.0 + menu_row_height * (row + 0.5))
    };
    let description = tr(app.desktop_locale, "录入并保存当前 Provider 的 API 密钥。");
    let expected_url =
        format!("https://www.google.com/search?q={}", urlencoding::encode(description));
    for (row, copy) in [(0.0, true), (3.0, false)] {
        assert!(
            scene
                .send(
                    &mut renderer,
                    &[Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Right))],
                    card_text,
                )
                .is_empty(),
            "opening the menu has no external action"
        );
        let messages = scene.click(&mut renderer, menu_item(row));
        assert!(match messages.as_slice() {
            [Message::CopyCode(text)] if copy => text == description,
            [Message::View(ViewMessage::OpenUrlExternal(url))] if !copy => url == &expected_url,
            _ => false,
        });
    }
    assert!(scene.send(&mut renderer, &[key(Named::Escape)], icon.center()).is_empty());
    drop(scene);

    app.provider_settings.connect_modal.as_mut().unwrap().api_key.clear();
    let mut scene = Scene::new(
        connect::view_overlays(&app, background()),
        &mut renderer,
        Size::new(420.0, 440.0),
    );
    let save = scene.bounds(&modal_path(&[2, 0, 1])).center();
    let messages = scene.click(&mut renderer, save);
    assert!(matches!(
        messages.as_slice(),
        [Message::Settings(SettingsMessage::ProviderConnectSubmit)]
    ));
    drop(scene);
    for message in messages {
        let _ = app.update(message);
    }
    assert_eq!(
        app.provider_settings.connect_error.as_ref().unwrap().display(app.desktop_locale),
        tr(app.desktop_locale, "请输入 API 密钥")
    );
    assert!(app.provider_settings.connect_modal.is_some());

    app.provider_settings.connect_modal = None;
    app.provider_settings.custom_provider_modal_open = true;
    for (id, editing, expected_error) in [
        ("INVALID", None, "提供商 ID 仅支持小写字母、数字、连字符或下划线"),
        ("fixture", Some("fixture".into()), "基础 URL 不能为空"),
    ] {
        app.provider_settings.custom.provider_id = id.into();
        app.provider_settings.custom.base_url.clear();
        app.provider_settings.custom_editing_provider_id = editing;
        let mut scene = Scene::new(
            custom_provider::view_overlays(&app, background()),
            &mut renderer,
            Size::new(420.0, 620.0),
        );
        let save = scene.bounds(&modal_path(&[2, 2])).center();
        let messages = scene.click(&mut renderer, save);
        assert!(matches!(
            messages.as_slice(),
            [Message::Settings(SettingsMessage::CustomProviderSave)]
        ));
        drop(scene);
        for message in messages {
            let _ = app.update(message);
        }
        assert_eq!(
            app.provider_settings.save_error.as_ref().unwrap().display(app.desktop_locale),
            tr(app.desktop_locale, expected_error)
        );
        assert!(app.provider_settings.custom_provider_modal_open);
    }
    app.provider_settings.custom_models_discovering = true;
    let mut scene = Scene::new(
        custom_provider::view_overlays(&app, background()),
        &mut renderer,
        Size::new(420.0, 620.0),
    );
    let save = scene.bounds(&modal_path(&[2, 2])).center();
    let cancel = scene.bounds(&modal_path(&[2, 1])).center();
    assert!(scene.click(&mut renderer, save).is_empty());
    assert!(matches!(
        scene.click(&mut renderer, cancel).as_slice(),
        [Message::Settings(SettingsMessage::CustomProviderClose)]
    ));
}




fn background() -> Element<'static, Message> {
    iced::widget::button("Underlying settings")
        .width(iced::Length::Fill)
        .height(iced::Length::Fill)
        .on_press(Message::Settings(SettingsMessage::ProvidersRefresh))
        .into()
}

// Stack → active layer → centering container → inset → card → contents.
fn modal_path(path: &[usize]) -> Vec<usize> {
    [&[1, 1, 0, 0, 0][..], path].concat()
}

struct Scene<'a> {
    ui: UserInterface<'a, Message, Theme, Renderer>,
    layout: layout::Node,
}

impl<'a> Scene<'a> {
    fn new(mut element: Element<'a, Message>, renderer: &mut Renderer, size: Size) -> Self {
        let mut tree = Tree::new(element.as_widget());
        let layout = element.as_widget_mut().layout(
            &mut tree,
            renderer,
            &layout::Limits::new(Size::ZERO, size),
        );
        let ui = UserInterface::build(element, size, Cache::new(), renderer);
        Self { ui, layout }
    }

    fn bounds(&self, path: &[usize]) -> Rectangle {
        path.iter()
            .fold(Layout::new(&self.layout), |layout, &index| {
                layout
                    .children()
                    .nth(index)
                    .unwrap_or_else(|| panic!("missing layout path {path:?}"))
            })
            .bounds()
    }

    fn send(&mut self, renderer: &mut Renderer, events: &[Event], point: Point) -> Vec<Message> {
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

    fn click(&mut self, renderer: &mut Renderer, point: Point) -> Vec<Message> {
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

fn ime(text: &str) -> Event {
    Event::InputMethod(iced::advanced::input_method::Event::Commit(text.into()))
}
