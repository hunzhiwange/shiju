//! 验证代理作用域标签、设置表单以及对话框和帮助浮层。
//!
//! 作用域稳定值与显示标签分开校验，避免持久化键被用户文案变化带偏。

use super::*;
use crate::app::App;
use iced::Element;
use iced::advanced::widget::Tree;
use iced::widget::text;
use vw_config_types::proxy::ProxyScope;

fn test_app() -> App {
    App::new().0
}

fn keep_element(element: Element<'_, Message>) {
    std::hint::black_box(element);
}

fn element_tree(element: Element<'_, Message>) -> Tree {
    Tree::new(element.as_widget())
}

#[test]
fn proxy_scope_label_returns_stable_user_labels() {
    assert_eq!(proxy_scope_label(ProxyScope::Environment), "系统环境");
    assert_eq!(proxy_scope_label(ProxyScope::Vibewindow), "ShiJu");
    assert_eq!(proxy_scope_label(ProxyScope::Services), "指定服务");
}

#[test]
fn proxy_scope_options_match_scope_labels() {
    assert_eq!(PROXY_SCOPE_OPTIONS, ["系统环境", "ShiJu", "指定服务"]);

    for scope in [ProxyScope::Environment, ProxyScope::Vibewindow, ProxyScope::Services] {
        assert!(PROXY_SCOPE_OPTIONS.contains(&proxy_scope_label(scope)));
    }
}

#[test]
fn field_row_accepts_fill_width_control() {
    let element: Element<'_, Message> =
        field_row("HTTP 代理", "HTTP 请求代理地址。", text("value"));
    drop(element);
}

#[test]
fn view_builds_default_and_populated_proxy_settings() {
    let app = test_app();
    keep_element(view(&app));

    let mut populated = test_app();
    populated.proxy_settings.enabled = true;
    populated.proxy_settings.scope = ProxyScope::Environment;
    populated.proxy_settings.http_proxy = "http://127.0.0.1:7890".to_string();
    populated.proxy_settings.https_proxy = "http://127.0.0.1:7891".to_string();
    populated.proxy_settings.all_proxy = "socks5://127.0.0.1:7892".to_string();
    populated.proxy_settings.no_proxy_input = "localhost, 127.0.0.1, *.internal".to_string();
    populated.proxy_settings.services_input = "provider.openai, tool.http_request".to_string();
    keep_element(view(&populated));

    populated.proxy_settings.scope = ProxyScope::Services;
    keep_element(view(&populated));
}

#[test]
fn overlays_return_base_dialog_or_help_modal() {
    let app = test_app();
    keep_element(view_overlays(&app, text("dialog").into()));

    let mut with_help = test_app();
    with_help.proxy_settings.show_help_modal = true;
    keep_element(view_overlays(&with_help, text("dialog").into()));
}

#[test]
fn view_groups_proxy_fields_into_expected_sections() {
    let tree = element_tree(content(&test_app()));

    assert_eq!(tree.children.len(), 7);
    assert_eq!(tree.children[2].children.len(), 3);
    assert_eq!(tree.children[4].children.len(), 5);
    assert_eq!(tree.children[6].children.len(), 3);
}

#[cfg(not(target_arch = "wasm32"))]
pub(in crate::app::components) mod rollout {
    use crate::app::{App, Message};
    use iced::advanced::clipboard;
    use iced::advanced::renderer::{Headless, Style};
    use iced::advanced::widget::{Id, Operation};
    use iced::{Element, Event, Font, Point, Rectangle, Renderer, Size, Theme, mouse};
    use iced_runtime::user_interface::{Cache, UserInterface};
    use vw_config_types::ui::{DesktopLocale, GatewayClientSystemSettingsConfig};

    pub(in crate::app::components) const MODES: [(DesktopLocale, Theme); 3] = [
        (DesktopLocale::SimplifiedChinese, Theme::Light),
        (DesktopLocale::English, Theme::Dark),
        (DesktopLocale::TraditionalChinese, Theme::Dark),
    ];
    const SIZE: Size<u32> = Size::new(480, 1240);
    type Ui<'a> = UserInterface<'a, Message, Theme, Renderer>;

    pub(in crate::app::components) fn isolated(test: &str) -> bool {
        const CHILD: &str = "SHIJU_ROLLOUT_17_CHILD";
        if std::env::var_os(CHILD).is_some() {
            return true;
        }
        let config_dir = match std::env::var_os("SHIJU_ROLLOUT_17_OUTPUT") {
            Some(dir) => tempfile::tempdir_in(dir).unwrap(),
            None => tempfile::tempdir().unwrap(),
        };
        let bootstrap = GatewayClientSystemSettingsConfig {
            host: "[".into(),
            servers: Vec::new(),
            ..Default::default()
        };
        std::fs::write(
            config_dir.path().join("gateway-client-bootstrap.json"),
            serde_json::to_vec(&bootstrap).unwrap(),
        )
        .unwrap();
        let output = std::process::Command::new(std::env::current_exe().unwrap())
            .args(["--exact", test.split_once("::").unwrap().1])
            .env(CHILD, "1")
            .env("HOME", config_dir.path())
            .env("VIBEWINDOW_CONFIG_DIR", config_dir.path())
            .env("VIBEWINDOW_WORKSPACE", config_dir.path())
            .current_dir(config_dir.path())
            .output()
            .unwrap();
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            output.status.success() && stdout.contains("1 passed; 0 failed"),
            "{stdout}\n{}",
            String::from_utf8_lossy(&output.stderr)
        );
        false
    }

    #[derive(Default)]
    struct Geometry {
        labels: Vec<(String, Rectangle)>,
        inputs: Vec<(String, Rectangle)>,
    }

    impl Operation for Geometry {
        fn traverse(&mut self, children: &mut dyn FnMut(&mut dyn Operation)) {
            children(self);
        }

        fn text(&mut self, _: Option<&Id>, bounds: Rectangle, value: &str) {
            self.labels.push((value.into(), bounds));
        }

        fn text_input(
            &mut self,
            _: Option<&Id>,
            bounds: Rectangle,
            state: &mut dyn iced::advanced::widget::operation::TextInput,
        ) {
            self.inputs.push((state.text().into(), bounds));
        }
    }

    pub(in crate::app::components) struct Scene<'a> {
        ui: Ui<'a>,
        renderer: &'a mut Renderer,
        app: &'a App,
        name: &'a str,
    }

    pub(in crate::app::components) async fn renderer() -> Renderer {
        {
            let mut fonts = iced::advanced::graphics::text::font_system().write().unwrap();
            for font in crate::fonts::load_all() {
                fonts.load_font(font);
            }
        }
        Renderer::new(Font::with_name("Noto Sans CJK SC"), 14.0.into(), Some("tiny-skia"))
            .await
            .unwrap()
    }

    impl<'a> Scene<'a> {
        pub(in crate::app::components) fn new(
            app: &'a App,
            renderer: &'a mut Renderer,
            name: &'a str,
            page: Element<'a, Message>,
        ) -> Self {
            use iced::Length;
            use iced::widget::scrollable::{Direction, Scrollbar};
            use iced::widget::{container, scrollable};

            let frame = scrollable(container(page).padding([0, 7]).width(Length::Fill))
                .direction(Direction::Vertical(Scrollbar::new().width(4).scroller_width(4)))
                .height(Length::Fill);
            let ui = UserInterface::build(
                frame,
                Size::from((SIZE.width, SIZE.height)),
                Cache::new(),
                renderer,
            );
            Self { ui, renderer, app, name }
        }

        fn geometry(&mut self) -> Geometry {
            let mut geometry = Geometry::default();
            self.ui.operate(self.renderer, &mut geometry);
            geometry
        }

        pub(in crate::app::components) fn label(&mut self, label: &str) -> Rectangle {
            let bounds = self
                .geometry()
                .labels
                .into_iter()
                .find(|(text, _)| text == label)
                .unwrap_or_else(|| panic!("{}: missing label {label:?}", self.name))
                .1;
            self.assert_fits(bounds);
            bounds
        }

        fn assert_fits(&self, bounds: Rectangle) {
            assert!(bounds.width > 0.0 && bounds.height > 0.0, "{bounds:?}");
            assert!(bounds.x >= 0.0 && bounds.x + bounds.width <= SIZE.width as f32, "{bounds:?}");
            assert!(
                bounds.y >= 0.0 && bounds.y + bounds.height <= SIZE.height as f32,
                "{bounds:?}"
            );
        }

        fn send(&mut self, cursor: mouse::Cursor, events: &[Event]) -> Vec<Message> {
            let mut messages = Vec::new();
            self.ui.update(events, cursor, self.renderer, &mut clipboard::Null, &mut messages);
            messages.retain(|message| !matches!(message, Message::None));
            messages
        }

        pub(in crate::app::components) fn click(&mut self, point: Point) -> Vec<Message> {
            self.send(
                mouse::Cursor::Available(point),
                &[
                    Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)),
                    Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)),
                ],
            )
        }

        fn key(&mut self, name: iced::keyboard::key::Named) -> Vec<Message> {
            use iced::keyboard::{self, Key};
            let key = Key::Named(name);
            self.send(
                mouse::Cursor::Unavailable,
                &[Event::Keyboard(keyboard::Event::KeyPressed {
                    modified_key: key.clone(),
                    key,
                    physical_key: keyboard::key::Physical::Unidentified(
                        keyboard::key::NativeCode::Unidentified,
                    ),
                    location: keyboard::Location::Standard,
                    modifiers: keyboard::Modifiers::empty(),
                    text: None,
                    repeat: false,
                })],
            )
        }

        pub(in crate::app::components) fn edit(&mut self, value: &str) -> Vec<Message> {
            let bounds = self
                .geometry()
                .inputs
                .into_iter()
                .find(|(text, _)| text == value)
                .unwrap_or_else(|| panic!("{}: missing fixture input {value:?}", self.name))
                .1;
            self.assert_fits(bounds);
            let end = Point::new(bounds.x + bounds.width - 3.0, bounds.center_y());
            assert!(self.click(end).is_empty());
            assert!(self.key(iced::keyboard::key::Named::End).is_empty());
            self.key(iced::keyboard::key::Named::Backspace)
        }

        fn frame(&mut self, tag: &str, cursor: mouse::Cursor) -> Vec<u8> {
            assert!(
                self.send(
                    cursor,
                    &[Event::Window(iced::window::Event::RedrawRequested(
                        iced::time::Instant::now(),
                    ))]
                )
                .is_empty()
            );
            let theme = &self.app.app_theme;
            self.ui.draw(self.renderer, theme, &Style { text_color: theme.palette().text }, cursor);
            let pixels = self.renderer.screenshot(SIZE, 1.0, theme.palette().background);
            if let Some(out) = std::env::var_os("SHIJU_ROLLOUT_17_OUTPUT") {
                let path = std::path::PathBuf::from(out)
                    .join("screenshots")
                    .join(format!("{}-{:?}-{tag}.png", self.name, self.app.desktop_locale,));
                ::image::save_buffer(
                    path,
                    &pixels,
                    SIZE.width,
                    SIZE.height,
                    ::image::ColorType::Rgba8,
                )
                .unwrap();
            }
            pixels
        }

        pub(in crate::app::components) fn help(&mut self, title: &str, description: &str) -> Point {
            assert!(
                self.geometry().labels.iter().all(|(text, _)| text != description),
                "{}: {title}: static explanation is still inline",
                self.name
            );
            let bounds = self.label(title);
            let away = mouse::Cursor::Unavailable;
            let idle = self.frame(&format!("{title}-idle"), away);
            let point = [
                Point::new(bounds.x + bounds.width + 15.0, bounds.center_y()),
                Point::new(bounds.x + 11.0, bounds.y + bounds.height + 15.0),
            ]
            .into_iter()
            .filter(|point| {
                Rectangle::with_size(Size::from((SIZE.width, SIZE.height))).contains(*point)
            })
            .find(|&position| {
                let cursor = mouse::Cursor::Available(position);
                assert!(
                    self.send(cursor, &[Event::Mouse(mouse::Event::CursorMoved { position })])
                        .is_empty()
                );
                self.frame(&format!("{title}-hover"), cursor) != idle
            })
            .unwrap_or_else(|| panic!("{}: hovering beside {title} must open help", self.name));

            assert!(self.click(point).is_empty());
            let open = self.frame(&format!("{title}-open"), away);
            assert!(open != idle, "{title}: clicking must keep help open when the pointer leaves");
            assert!(self.key(iced::keyboard::key::Named::Escape).is_empty());
            let closed = self.frame(&format!("{title}-closed"), away);
            assert!(closed == idle, "{title}: Escape must restore the initial frame");
            point
        }

        pub(in crate::app::components) fn actions(&mut self, icon: Point, description: &str) {
            assert!(self.click(icon).is_empty());
            let body = Point::new(icon.x + 29.0, icon.y + 5.0);
            let line_height = iced::advanced::text::LineHeight::default().to_absolute(12.0.into());
            let item_height = f32::from(line_height) + 10.0;
            for index in [0, 3] {
                assert!(
                    self.send(
                        mouse::Cursor::Available(body),
                        &[Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Right)),]
                    )
                    .is_empty()
                );
                let item =
                    Point::new(body.x + 24.0, body.y + 3.0 + item_height * (index as f32 + 0.5));
                let messages = self.click(item);
                assert!(
                    match (index, messages.as_slice()) {
                        (0, [Message::CopyCode(value)]) => value == description,
                        (
                            3,
                            [
                                Message::View(crate::app::message::ViewMessage::OpenUrlExternal(
                                    url,
                                )),
                            ],
                        ) => {
                            *url == format!(
                                "https://www.google.com/search?q={}",
                                urlencoding::encode(description)
                            )
                        }
                        _ => false,
                    },
                    "unexpected help action: {messages:?}"
                );
            }
            assert!(self.key(iced::keyboard::key::Named::Escape).is_empty());
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn help_rollout_17_renders_contextual_help() {
    use super::super::{system_settings::SystemTab, system_settings_network as network};
    use crate::app::i18n::I18n;
    use crate::app::message::SettingsMessage;
    use rollout::{MODES, Scene, isolated, renderer};

    if !isolated(concat!(module_path!(), "::help_rollout_17_renders_contextual_help")) {
        return;
    }
    let mut renderer = renderer().await;
    let mut app = test_app();
    app.system_settings_tab = SystemTab::Proxy;
    app.proxy_settings.scope = ProxyScope::Services;
    app.proxy_settings.http_proxy = "http://proxy.example.invalid:1234".into();
    app.proxy_settings.save_error = Some("Fixture proxy connection failed".into());

    for (locale, theme) in MODES {
        app.desktop_locale = locale;
        app.app_theme = theme;
        let t = |key| I18n::new(locale).__(key);
        let mut scene = Scene::new(&app, &mut renderer, "proxy-network", network::view(&app));
        for label in [
            "Fixture proxy connection failed",
            "决定代理应用到环境、全局流量或指定服务。",
            "当未命中 HTTP/HTTPS 时使用的兜底代理。",
            "仅在范围为指定服务时生效。",
        ] {
            scene.label(t(label));
        }
        let description = t("HTTP 请求代理地址。");
        let icon = scene.help(t("HTTP 代理"), description);
        scene.actions(icon, description);
        for (title, description) in [
            ("基础行为", "控制代理开关与作用域选择。"),
            ("代理地址", "分别配置 HTTP、HTTPS 与兜底代理地址。"),
            ("排除与路由", "配置 NO_PROXY 和指定服务选择器。"),
            ("启用", "控制是否启用代理能力。"),
            ("HTTPS 代理", "HTTPS 请求代理地址。"),
            ("排除列表", "支持逗号或换行分隔，格式与 NO_PROXY 一致。"),
        ] {
            scene.help(t(title), t(description));
        }
        assert!(matches!(
            scene.edit("http://proxy.example.invalid:1234").as_slice(),
            [Message::Settings(SettingsMessage::ProxyHttpChanged(value))]
                if value == "http://proxy.example.invalid:123"
        ));

        drop(scene);
        let mut scene = Scene::new(&app, &mut renderer, "proxy-standalone", view(&app));
        scene.help(t("代理配置"), t("配置 HTTP、HTTPS、SOCKS5 代理以及生效范围。"));
    }
}
