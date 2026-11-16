// 宿主契约：系统设置页与双层弹窗装配、selectable 帮助菜单和 providers::update 草稿/校验反馈；纯布局测试迁至 desktop_split_11_tests::custom_model。
use super::super::custom_provider;
use super::view_overlays;
use crate::app::message::SettingsMessage;
use crate::app::state::{
    CustomProviderModelDraft, CustomProviderModelTab, CustomProviderTab,
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
use config_ui::DesktopLocale;

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

fn screenshot(app: &App, renderer: &mut Renderer, theme: &Theme, size: Size<u32>, name: &str) {
    let mut scene = Scene::new(view(app), renderer, size);
    scene.capture(renderer, theme, mouse::Cursor::Unavailable, name);
}

fn settings_view(app: &App) -> Element<'_, Message> {
    crate::app::components::system_settings::view(app)
}

#[derive(Default)]
struct SettingsText(Vec<String>);

impl Operation for SettingsText {
    fn traverse(&mut self, children: &mut dyn FnMut(&mut dyn Operation)) {
        children(self);
    }

    fn text(&mut self, _: Option<&iced::advanced::widget::Id>, _: Rectangle, value: &str) {
        self.0.push(value.to_owned());
    }
}

impl Scene<'_> {
    fn assert_body_text(&mut self, renderer: &Renderer, value: &str, visible: bool) {
        let mut text = SettingsText::default();
        self.ui.operate(renderer, &mut text);
        assert_eq!(
            text.0.iter().any(|item| item.contains(value)),
            visible,
            "body visibility for {value:?}"
        );
    }

    fn assert_migrated_help(
        &mut self,
        renderer: &mut Renderer,
        theme: &Theme,
        trigger_path: &[usize],
        expected: &[&str],
    ) {
        for text in expected {
            self.assert_body_text(renderer, text, false);
        }
        let icon = self.bounds(trigger_path);
        assert_eq!(icon.size(), Size::new(22.0, 22.0));
        let point = icon.center();
        let idle = self.capture(renderer, theme, mouse::Cursor::Unavailable, "");
        assert!(
            self.send(
                renderer,
                &[Event::Mouse(mouse::Event::CursorMoved { position: point })],
                cursor(icon),
            )
            .is_empty()
        );
        assert!(self.capture(renderer, theme, cursor(icon), "") != idle);
        assert!(self.click(renderer, point).is_empty());
        assert!(
            self.send(
                renderer,
                &[Event::Mouse(mouse::Event::CursorLeft)],
                mouse::Cursor::Unavailable
            )
            .is_empty()
        );

        // Help cards open on the side with enough space, up to 300px wide.
        let right = icon.x + icon.width + 4.0;
        let left_space = icon.x - 4.0;
        let right_space = self.size.width as f32 - right;
        let width = 300.0_f32.min(left_space.max(right_space));
        let x = if right_space >= width { right } else { left_space - width };
        let start = Point::new(x + 12.5, icon.y + 15.0);
        let end = Point::new(x + width - 1.0, icon.y + 200.0);
        for (point, event) in [
            (start, mouse::Event::ButtonPressed(mouse::Button::Left)),
            (end, mouse::Event::CursorMoved { position: end }),
            (end, mouse::Event::ButtonReleased(mouse::Button::Left)),
        ] {
            assert!(
                self.send(renderer, &[Event::Mouse(event)], mouse::Cursor::Available(point))
                    .is_empty()
            );
        }
        let copy = Key::Character("c".into());
        let copied = self.send(
            renderer,
            &[Event::Keyboard(keyboard::Event::KeyPressed {
                key: copy.clone(),
                modified_key: copy,
                physical_key: keyboard::key::Physical::Code(keyboard::key::Code::KeyC),
                location: keyboard::Location::Standard,
                modifiers: if cfg!(target_os = "macos") {
                    keyboard::Modifiers::LOGO
                } else {
                    keyboard::Modifiers::CTRL
                },
                text: None,
                repeat: false,
            })],
            mouse::Cursor::Unavailable,
        );
        assert!(
            matches!(copied.as_slice(), [Message::CopyCode(value)] if expected.iter().all(|text| value.contains(text))),
            "localized help must be readable and selectable: {copied:?}"
        );
        assert!(self.send(renderer, &[key(Named::Escape)], mouse::Cursor::Unavailable).is_empty());
    }
}

fn settings_help_fixture() -> App {
    use crate::app::components::system_settings::SystemTab;

    let mut app = fixture();
    app.navigation_panels.show_system_settings = true;
    app.navigation_panels.system_settings_tab = SystemTab::Providers;
    app
}

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn settings_help_gap_03_provider_id_and_account_auth() {
    use crate::app::i18n::settings_models::connection_text::text as tr;

    let mut renderer = renderer().await;
    let mut app = settings_help_fixture();
    for (locale, theme) in [
        (DesktopLocale::English, Theme::Light),
        (DesktopLocale::SimplifiedChinese, Theme::Dark),
        (DesktopLocale::TraditionalChinese, Theme::Light),
    ] {
        app.desktop_locale = locale;
        app.app_theme = theme.clone();
        for width in [420, 960] {
            let size = Size::new(width, 620);
            for editing in [false, true] {
                app.provider_settings.custom_editing_provider_id =
                    editing.then(|| "fixture".into());
                let description = if editing {
                    "作为配置中的稳定标识，编辑模式下保持只读。"
                } else {
                    "作为配置中的稳定标识，只允许小写字母、数字、连字符或下划线。"
                };
                let mut scene = Scene::new(settings_view(&app), &mut renderer, size);
                scene.assert_body_text(&renderer, tr(locale, description), false);
                let icon =
                    if width == 420 { group(&[0, 0, 0, 1]) } else { group(&[0, 0, 0, 0, 1]) };
                let mut expected = vec![tr(locale, description)];
                if !editing {
                    expected.push(tr(locale, "例如 myprovider。"));
                }
                scene.assert_migrated_help(&mut renderer, &theme, &icon, &expected);
                if editing {
                    scene.assert_body_text(&renderer, "fixture", true);
                } else {
                    scene.assert_body_text(
                        &renderer,
                        tr(locale, "小写字母、数字、下划线或连字符。"),
                        true,
                    );
                }
                if width == 420 {
                    let id = if editing { scalar_control(0, &[]) } else { scalar_control(0, &[0]) };
                    let id = scene.bounds(&id);
                    let changes = scene.edit(&mut renderer, id, "-new");
                    if editing {
                        assert!(changes.is_empty());
                    } else {
                        assert!(matches!(
                            changes.as_slice(),
                            [Message::Settings(SettingsMessage::CustomProviderIdChanged(value))] if value == "fixture-new"
                        ));
                    }
                }
            }

            app.provider_settings.custom.provider_id = "shiju".into();
            app.provider_settings.custom_provider_tab = CustomProviderTab::Authentication;
            let mut scene = Scene::new(settings_view(&app), &mut renderer, size);
            let icon = if width == 420 { group(&[0, 0, 0, 1]) } else { group(&[0, 0, 0, 0, 1]) };
            scene.assert_migrated_help(
                &mut renderer,
                &theme,
                &icon,
                &[tr(locale, "登录识矩账户后自动配置模型，无需填写模型密钥。")],
            );
            scene.assert_body_text(&renderer, tr(locale, "密钥由账户管理，不显示明文"), true);
            scene.assert_body_text(&renderer, "API Key", false);
            drop(scene);
            app.provider_settings.custom.provider_id = "fixture".into();
            app.provider_settings.custom_provider_tab = CustomProviderTab::Basic;
        }
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn settings_help_gap_03_model_limits_and_modalities() {
    use crate::app::i18n::settings_models::connection_text::text as tr;

    let mut renderer = renderer().await;
    let mut app = settings_help_fixture();
    apply(&mut app, vec![Message::Settings(SettingsMessage::CustomProviderModelOpen(Some(0)))]);
    for (locale, theme) in [
        (DesktopLocale::English, Theme::Dark),
        (DesktopLocale::SimplifiedChinese, Theme::Light),
        (DesktopLocale::TraditionalChinese, Theme::Dark),
    ] {
        app.desktop_locale = locale;
        app.app_theme = theme.clone();
        for width in [420, 960] {
            let size = Size::new(width, 620);
            for (index, tab) in [
                CustomProviderModelTab::Limits,
                CustomProviderModelTab::Capabilities,
                CustomProviderModelTab::Info,
            ]
            .into_iter()
            .enumerate()
            {
                let mut scene = Scene::new(settings_view(&app), &mut renderer, size);
                let target = scene.bounds(&modal(&[0, 1, (index + 1) % 3]));
                let messages = scene.click(&mut renderer, target.center());
                assert!(matches!(
                    messages.as_slice(),
                    [Message::Settings(SettingsMessage::CustomProviderModelTabSelected(value))] if *value == tab
                ));
                drop(scene);
                apply(&mut app, messages);
                let mut scene = Scene::new(settings_view(&app), &mut renderer, size);
                match tab {
                    CustomProviderModelTab::Limits => {
                        scene.assert_migrated_help(
                            &mut renderer,
                            &theme,
                            &modal(&[1, 0, 0, 0, 1]),
                            &[tr(
                                locale,
                                "空值明确表示沿用内置基线或安全默认值，数值单位为 token。",
                            )],
                        );
                        for (field, label, description, example) in [
                            (
                                0,
                                "上下文限制（token）",
                                "总上下文窗口；留空时沿用内置基线或安全默认值。",
                                "例如 200000。",
                            ),
                            (
                                2,
                                "最大输入（token）",
                                "最大输入窗口；留空时沿用内置基线或安全默认值。",
                                "例如 200000。",
                            ),
                            (
                                4,
                                "最大输出（token）",
                                "最大输出窗口；0 可表示该模型不产生文本输出。",
                                "例如 4096。",
                            ),
                        ] {
                            let icon = if width == 420 {
                                group(&[field, 0, 0, 1])
                            } else {
                                group(&[field, 0, 0, 0, 1])
                            };
                            scene.assert_body_text(&renderer, tr(locale, label), true);
                            scene.assert_migrated_help(
                                &mut renderer,
                                &theme,
                                &icon,
                                &[tr(locale, description), tr(locale, example)],
                            );
                        }
                    }
                    CustomProviderModelTab::Capabilities => {
                        for (field, description) in [
                            (0, "选择 provider 接受的输入类型；留空时按文本模型默认值处理。"),
                            (2, "选择 provider 返回的输出类型；留空时按文本模型默认值处理。"),
                        ] {
                            let icon = group(&[2, field, 0, 0, 1]);
                            scene.assert_migrated_help(
                                &mut renderer,
                                &theme,
                                &icon,
                                &[tr(locale, description)],
                            );
                        }
                    }
                    CustomProviderModelTab::Info => {
                        let save = scene.bounds(&modal(&[2, 2]));
                        assert!(matches!(
                            scene.click(&mut renderer, save.center()).as_slice(),
                            [Message::Settings(SettingsMessage::CustomProviderModelModalSave)]
                        ));
                    }
                }
            }
        }
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn help_rollout_02_renders_contextual_help() {
    let mut renderer = renderer().await;
    let mut app = fixture();
    apply(&mut app, vec![Message::Settings(SettingsMessage::CustomProviderModelOpen(Some(0)))]);
    for (locale, theme, mode) in [
        (DesktopLocale::English, Theme::Light, "en-light"),
        (DesktopLocale::SimplifiedChinese, Theme::Dark, "zh-dark"),
        (DesktopLocale::TraditionalChinese, Theme::Light, "zh-hant-light"),
    ] {
        app.desktop_locale = locale;
        app.app_theme = theme.clone();
        for (tab, targets) in [
            (
                CustomProviderModelTab::Info,
                vec![
                    ("title", modal(&[0, 0, 0, 0, 1])),
                    ("info", modal(&[1, 0, 0, 0, 1])),
                    ("id", group(&[0, 0, 0, 1])),
                    ("name", group(&[2, 0, 0, 1])),
                    ("family", group(&[4, 0, 0, 1])),
                ],
            ),
            (
                CustomProviderModelTab::Capabilities,
                vec![
                    ("capabilities-heading", modal(&[1, 0, 0, 0, 1])),
                    ("capabilities", group(&[0, 0, 0, 1])),
                ],
            ),
        ] {
            apply(
                &mut app,
                vec![Message::Settings(SettingsMessage::CustomProviderModelTabSelected(tab))],
            );
            let mut scene = Scene::new(view(&app), &mut renderer, Size::new(320, 520));
            let idle = scene.capture(
                &mut renderer,
                &theme,
                mouse::Cursor::Unavailable,
                &format!("model-help-{mode}-{tab:?}-idle"),
            );
            for (name, path) in targets {
                let target = scene.bounds(&path);
                assert_eq!(
                    target.size(),
                    Size::new(22.0, 22.0),
                    "{name} has a separate help target"
                );
                assert!(target.x >= 0.0 && target.x + target.width <= 320.0);
                let hover = Event::Mouse(mouse::Event::CursorMoved { position: target.center() });
                assert!(scene.send(&mut renderer, &[hover], cursor(target)).is_empty());
                let open = scene.capture(
                    &mut renderer,
                    &theme,
                    cursor(target),
                    &format!("model-help-{mode}-{name}-open"),
                );
                assert!(idle != open, "{name} help is drawn above the modal");
                assert!(
                    scene
                        .send(&mut renderer, &[key(Named::Escape)], mouse::Cursor::Unavailable)
                        .is_empty()
                );
                assert!(
                    idle == scene.capture(
                        &mut renderer,
                        &theme,
                        mouse::Cursor::Unavailable,
                        &format!("model-help-{mode}-{name}-dismissed"),
                    ),
                    "{name} closes without dismissing the modal"
                );
            }
        }
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn help_rollout_02_dismisses_menu_before_card_and_keeps_modal_actions() {
    use crate::app::i18n::settings_models::connection_text::text as tr;
    use crate::app::message::ViewMessage;

    let mut renderer = renderer().await;
    let mut app = settings_help_fixture();
    app.app_theme = Theme::Light;
    apply(&mut app, vec![Message::Settings(SettingsMessage::CustomProviderModelOpen(Some(0)))]);
    let mut scene = Scene::new(settings_view(&app), &mut renderer, Size::new(420, 620));
    let icon = scene.bounds(&group(&[0, 0, 0, 1]));
    let idle = scene.capture(
        &mut renderer,
        &Theme::Light,
        mouse::Cursor::Unavailable,
        "model-help-id-idle",
    );
    let start = Point::new(icon.x + icon.width + 4.0 + 12.5, icon.y + 16.0);
    let end = Point::new(start.x + 240.0, start.y + 100.0);
    let mouse_event = |scene: &mut Scene<'_>, renderer: &mut Renderer, point, event| {
        scene.send(renderer, &[Event::Mouse(event)], mouse::Cursor::Available(point))
    };
    let right = mouse::Event::ButtonPressed(mouse::Button::Right);
    assert!(scene.click(&mut renderer, icon.center()).is_empty());
    let pinned = scene.capture(
        &mut renderer,
        &Theme::Light,
        mouse::Cursor::Unavailable,
        "model-help-id-pinned",
    );
    assert!(pinned != idle);

    // Without a selection the search payload is the localized explanation.
    assert!(mouse_event(&mut scene, &mut renderer, start, right).is_empty());
    let line_height = iced::advanced::text::LineHeight::default().to_absolute(12.0.into());
    let row_height = f32::from(line_height) + 10.0;
    let menu_item =
        |index: f32| Point::new(start.x + 24.0, start.y + 3.0 + row_height * (index + 0.5));
    let description = tr(app.desktop_locale, "用于请求 provider 的真实 model_id。");
    let search = format!("https://www.google.com/search?q={}", urlencoding::encode(description));
    assert!(matches!(
        scene.click(&mut renderer, menu_item(3.0)).as_slice(),
        [Message::View(ViewMessage::OpenUrlExternal(url))] if url == &search
    ));

    let left = mouse::Button::Left;
    for (point, event) in [
        (start, mouse::Event::ButtonPressed(left)),
        (end, mouse::Event::CursorMoved { position: end }),
        (end, mouse::Event::ButtonReleased(left)),
    ] {
        assert!(mouse_event(&mut scene, &mut renderer, point, event).is_empty());
    }
    let selected = scene.capture(
        &mut renderer,
        &Theme::Light,
        mouse::Cursor::Unavailable,
        "model-help-id-selected",
    );
    assert!(selected != pinned, "drag highlights the title and description");
    assert!(mouse_event(&mut scene, &mut renderer, start, right).is_empty());
    let menu = scene.capture(
        &mut renderer,
        &Theme::Light,
        mouse::Cursor::Unavailable,
        "model-help-id-menu",
    );
    assert!(menu != selected);
    assert!(
        scene.send(&mut renderer, &[key(Named::Escape)], mouse::Cursor::Unavailable).is_empty()
    );
    let menu_closed = scene.capture(
        &mut renderer,
        &Theme::Light,
        mouse::Cursor::Unavailable,
        "model-help-id-menu-dismissed",
    );
    assert!(
        menu_closed == selected,
        "the first Escape closes only the menu and preserves selection"
    );

    assert!(mouse_event(&mut scene, &mut renderer, start, right).is_empty());
    let expected = format!("{}\n{description}", tr(app.desktop_locale, "模型 ID"));
    assert!(matches!(
        scene.click(&mut renderer, menu_item(0.0)).as_slice(),
        [Message::CopyCode(text)] if text == &expected
    ));

    assert!(
        scene.send(&mut renderer, &[key(Named::Escape)], mouse::Cursor::Unavailable).is_empty()
    );
    let card_closed = scene.capture(
        &mut renderer,
        &Theme::Light,
        mouse::Cursor::Unavailable,
        "model-help-id-card-dismissed",
    );
    assert!(card_closed == idle, "the next Escape closes only the card");
    assert!(
        scene.send(&mut renderer, &[key(Named::Escape)], mouse::Cursor::Unavailable).is_empty()
    );

    let save = scene.bounds(&modal(&[2, 2]));
    assert!(matches!(
        scene.click(&mut renderer, save.center()).as_slice(),
        [Message::Settings(SettingsMessage::CustomProviderModelModalSave)]
    ));
    let cancel = scene.bounds(&modal(&[2, 1]));
    let messages = scene.click(&mut renderer, cancel.center());
    drop(scene);
    assert!(matches!(
        messages.as_slice(),
        [Message::Settings(SettingsMessage::CustomProviderModelClose)]
    ));
    apply(&mut app, messages);
    assert!(app.provider_settings.custom_model_modal.is_none());
    assert!(app.provider_settings.custom_provider_modal_open);

    let mut parent = Scene::new(settings_view(&app), &mut renderer, Size::new(420, 620));
    let cancel = parent.bounds(&modal(&[2, 1])).center();
    let messages = parent.click(&mut renderer, cancel);
    assert!(matches!(
        messages.as_slice(),
        [Message::Settings(SettingsMessage::CustomProviderClose)]
    ));
    drop(parent);
    apply(&mut app, messages);
    assert!(!app.provider_settings.custom_provider_modal_open);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn help_rollout_02_keeps_help_inside_the_window_and_clips_scrolled_triggers() {
    use crate::app::i18n::settings_models::connection_text::text as tr;

    let mut renderer = renderer().await;
    let mut app = fixture();
    let theme = Theme::Light;
    app.app_theme = theme.clone();
    apply(&mut app, vec![Message::Settings(SettingsMessage::CustomProviderModelOpen(Some(0)))]);
    let size = Size::new(320, 315);
    let mut scene = Scene::new(view(&app), &mut renderer, size);
    let body = scene.bounds(&modal(&[1]));
    let icon = scene.bounds(&group(&[0, 0, 0, 1]));
    let save = scene.bounds(&modal(&[2, 2])).center();
    assert!(body.contains(icon.center()));
    let idle =
        scene.capture(&mut renderer, &theme, mouse::Cursor::Unavailable, "model-help-short-idle");

    assert!(scene.click(&mut renderer, icon.center()).is_empty());
    let pinned =
        scene.capture(&mut renderer, &theme, mouse::Cursor::Unavailable, "model-help-short-open");
    let save_pixel = (save.y as usize * size.width as usize + save.x as usize) * 4;
    assert_ne!(&pinned[save_pixel..save_pixel + 4], &idle[save_pixel..save_pixel + 4]);
    assert!(scene.click(&mut renderer, save).is_empty(), "help intercepts clicks over the footer");

    let right = Point::new(size.width as f32 - 8.0, icon.y + 16.0);
    let right_press = Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Right));
    assert!(scene.send(&mut renderer, &[right_press], mouse::Cursor::Available(right)).is_empty());
    let menu =
        scene.capture(&mut renderer, &theme, mouse::Cursor::Unavailable, "model-help-short-menu");
    assert!(menu != pinned, "the menu opens near the lower right corner");
    let line_height = iced::advanced::text::LineHeight::default().to_absolute(12.0.into());
    let row_height = f32::from(line_height) + 10.0;
    let menu_height = 6.0 + 5.0 * row_height;
    assert!(right.y + menu_height > size.height as f32);
    let copy =
        Point::new(size.width as f32 - 116.0, right.y - menu_height + 3.0 + row_height / 2.0);
    assert!(matches!(
        scene.click(&mut renderer, copy).as_slice(),
        [Message::CopyCode(text)] if text == tr(app.desktop_locale, "用于请求 provider 的真实 model_id。")
    ));

    let offset = scene.scroll(&mut renderer, body, 500.0);
    assert!(icon.y + icon.height - offset < body.y, "the help trigger scrolls out of view");
    let scrolled = scene.capture(
        &mut renderer,
        &theme,
        mouse::Cursor::Unavailable,
        "model-help-short-scrolled",
    );
    assert!(
        scene.send(&mut renderer, &[key(Named::Escape)], mouse::Cursor::Unavailable).is_empty()
    );
    assert!(
        scene.capture(
            &mut renderer,
            &theme,
            mouse::Cursor::Unavailable,
            "model-help-short-scrolled-escape"
        ) == scrolled,
        "a clipped trigger no longer keeps its help card open"
    );
    assert!(matches!(
        scene.click(&mut renderer, save).as_slice(),
        [Message::Settings(SettingsMessage::CustomProviderModelModalSave)]
    ));
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
            // The capabilities tab still needs a scrollbar at this height.
            let message = scene.click(view(app), capabilities);
            apply(app, vec![Message::Settings(message)]);
            // Inspect only the active card, then exercise cancel in the provider stack.
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
