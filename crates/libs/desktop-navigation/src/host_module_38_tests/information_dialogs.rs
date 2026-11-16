#![allow(dead_code)]
use super::navigation_fixture::{Fixture, help};
use crate::components::{
    about_modal,
    message::{Message as InstallMessage, ViewMessage},
};
use config_ui::DesktopLocale;
use desktop_i18n::I18n;
use iced::advanced::widget::{Id, Operation, operation};
use iced::advanced::{clipboard, renderer::Headless};
use iced::keyboard::{self, Key, key::Named};
use iced::{Event, Font, Point, Rectangle, Renderer, Size, Theme, Vector, mouse};
use iced_runtime::user_interface::{Cache, UserInterface};
#[derive(Debug, Clone)]
enum Message {
    About(about_modal::Message),
    Install(InstallMessage),
}
fn fixture() -> Fixture {
    Fixture::default()
}
fn view(input: &Fixture) -> iced::Element<'_, Message> {
    about_modal::view(&about_modal::Input {
        desktop_locale: input.desktop_locale,
        last_copied_code_hash: None,
    })
    .map(Message::About)
}
mod install_cli_modal {
    use super::*;
    pub fn view_app_update(input: &Fixture) -> iced::Element<'_, Message> {
        input.view().map(Message::Install)
    }
    pub fn view_localized<'a>(
        i18n: I18n,
        title: &'a str,
        body: &'a str,
    ) -> iced::Element<'a, Message> {
        crate::components::install_cli_modal::view_localized(i18n, title, body, help)
            .map(Message::Install)
    }
}
type Ui<'a> = UserInterface<'a, Message, Theme, Renderer>;
const APP_NAME: &str = "VibeWindow";
const APP_VERSION: &str = env!("CARGO_PKG_VERSION");
const ABOUT_TITLE: &str = "关于识矩";
const ABOUT_SUBTITLE: &str =
    "Rust-first 的自主代理运行时，连接桌面端、CLI、网关服务和长驻守护进程。";
const ABOUT_DESCRIPTION: &str = "项目包含桌面应用、命令行入口、HTTP/WebSocket 网关、任务与记忆管理、通信通道、模型/提供商解析、ACP 桥接和跨平台 WebView helper。";
const AUTHOR_INFO: &str = "开发者：刘祥敏。";

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

fn scene<'a>(view: Element<'a, Message>, renderer: &mut Renderer, size: Size<u32>) -> Ui<'a> {
    UserInterface::build(
        view,
        Size::new(size.width as f32, size.height as f32),
        Cache::new(),
        renderer,
    )
}

fn send(ui: &mut Ui<'_>, renderer: &mut Renderer, point: Point, events: &[Event]) -> Vec<Message> {
    let mut messages = Vec::new();
    ui.update(
        events,
        mouse::Cursor::Available(point),
        renderer,
        &mut clipboard::Null,
        &mut messages,
    );
    messages
}

fn click(ui: &mut Ui<'_>, renderer: &mut Renderer, point: Point) -> Vec<Message> {
    send(
        ui,
        renderer,
        point,
        &[
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)),
            Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)),
        ],
    )
}

fn key(ui: &mut Ui<'_>, renderer: &mut Renderer, key: Key, text: Option<&str>) -> Vec<Message> {
    send(
        ui,
        renderer,
        Point::ORIGIN,
        &[Event::Keyboard(keyboard::Event::KeyPressed {
            modified_key: key.clone(),
            key,
            physical_key: keyboard::key::Physical::Unidentified(
                keyboard::key::NativeCode::Unidentified,
            ),
            location: keyboard::Location::Standard,
            modifiers: keyboard::Modifiers::empty(),
            text: text.map(Into::into),
            repeat: false,
        })],
    )
}

fn capture(
    ui: &mut Ui<'_>,
    renderer: &mut Renderer,
    theme: &Theme,
    size: Size<u32>,
    name: &str,
    point: Point,
) -> Vec<u8> {
    let redraw = send(
        ui,
        renderer,
        point,
        &[Event::Window(iced::window::Event::RedrawRequested(iced::time::Instant::now()))],
    );
    assert!(redraw.is_empty(), "{redraw:?}");
    // UserInterface rebuilds the active overlay after update before drawing it.
    ui.draw(
        renderer,
        theme,
        &iced::advanced::renderer::Style { text_color: theme.palette().text },
        mouse::Cursor::Available(point),
    );
    let pixels = renderer.screenshot(size, 1.0, theme.palette().background);
    if let Some(directory) = std::env::var_os("SHIJU_UI_REFRESH_41_SCREENSHOTS") {
        ::image::save_buffer(
            std::path::PathBuf::from(directory).join(format!("{name}.png")),
            &pixels,
            size.width,
            size.height,
            ::image::ColorType::Rgba8,
        )
        .unwrap();
    }
    pixels
}

#[derive(Default)]
struct Bounds {
    labels: Vec<(String, Rectangle)>,
    inputs: Vec<(String, Rectangle)>,
    focus: Vec<Rectangle>,
    body: Option<(Rectangle, Vector)>,
    menu: Option<Rectangle>,
}

impl Operation for Bounds {
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
        input: &mut dyn operation::TextInput,
    ) {
        self.inputs.push((input.text().into(), bounds));
    }

    fn focusable(&mut self, _: Option<&Id>, bounds: Rectangle, _: &mut dyn operation::Focusable) {
        self.focus.push(bounds);
    }

    fn scrollable(
        &mut self,
        id: Option<&Id>,
        bounds: Rectangle,
        _: Rectangle,
        offset: Vector,
        _: &mut dyn operation::Scrollable,
    ) {
        if id == Some(&Id::from("settings-select-menu")) {
            self.menu = Some(bounds);
        } else {
            self.body = Some((bounds, offset));
        }
    }
}

fn bounds(ui: &mut Ui<'_>, renderer: &Renderer) -> Bounds {
    let mut result = Bounds::default();
    ui.operate(renderer, &mut result);
    result
}

fn position(controls: &[(String, Rectangle)], value: &str) -> Rectangle {
    controls
        .iter()
        .find(|(text, _)| text == value)
        .unwrap_or_else(|| panic!("missing {value:?} in {controls:?}"))
        .1
}

fn hover(ui: &mut Ui<'_>, renderer: &mut Renderer, point: Point) -> Vec<Message> {
    send(ui, renderer, point, &[Event::Mouse(mouse::Event::CursorMoved { position: point })])
}

fn scroll(ui: &mut Ui<'_>, renderer: &mut Renderer, point: Point, y: f32) -> Vec<Message> {
    send(
        ui,
        renderer,
        point,
        &[Event::Mouse(mouse::Event::WheelScrolled {
            delta: mouse::ScrollDelta::Pixels { x: 0.0, y },
        })],
    )
}

fn assert_inside(bounds: Rectangle, area: Rectangle) {
    assert!(
        bounds.width > 0.0
            && bounds.height > 0.0
            && bounds.x >= area.x
            && bounds.y >= area.y
            && bounds.x + bounds.width <= area.x + area.width
            && bounds.y + bounds.height <= area.y + area.height,
        "{bounds:?} is outside {area:?}"
    );
}

fn viewport(size: Size<u32>) -> Rectangle {
    Rectangle::with_size(Size::new(size.width as f32, size.height as f32))
}

fn assert_fixed_controls(title: Rectangle, body: Rectangle, action: Rectangle, size: Size<u32>) {
    assert_inside(title, viewport(size));
    assert_inside(action, viewport(size));
    assert!(title.y + title.height < body.y);
    assert!(action.y > body.y + body.height);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn ui_refresh_41_layout_and_interaction() {
    let mut app = fixture();
    let mut renderer = renderer().await;
    for (width, locale, theme) in [
        (420, DesktopLocale::TraditionalChinese, Theme::Dark),
        (640, DesktopLocale::SimplifiedChinese, Theme::Light),
        (960, DesktopLocale::English, Theme::Dark),
        (420, DesktopLocale::English, Theme::Light),
    ] {
        app.desktop_locale = locale;
        let i18n = I18n::new(locale);
        let size = Size::new(width, 560);
        let name = format!("{width}-{locale:?}");
        let mut ui = scene(view(&app), &mut renderer, size);
        capture(&mut ui, &mut renderer, &theme, size, &format!("about-{name}"), Point::ORIGIN);
        let controls = bounds(&mut ui, &renderer);
        let title = position(&controls.labels, i18n.__(ABOUT_TITLE));
        let copy = position(&controls.labels, i18n.__("复制应用信息"));
        let close = position(&controls.labels, i18n.__("关闭"));
        assert_fixed_controls(title, controls.body.unwrap().0, copy, size);
        let expected = format!(
            "{APP_NAME} v{APP_VERSION}\n{}\n\n{}\n{}",
            i18n.__(ABOUT_SUBTITLE),
            i18n.__(ABOUT_DESCRIPTION),
            i18n.__(AUTHOR_INFO)
        );
        assert!(matches!(
            click(&mut ui, &mut renderer, copy.center()).as_slice(),
            [Message::About(about_modal::Message::CopyCode(value))] if value == &expected
        ));
        assert!(matches!(
            click(&mut ui, &mut renderer, close.center()).as_slice(),
            [Message::About(about_modal::Message::Close)]
        ));
        drop(ui);

        app.update_panel.cli_install_modal_title = "安装 CLI 命令工具".into();
        app.update_panel.cli_install_modal_message =
            format!("路径：/tmp/{}/shiju\nCLI setup fixture", "測試路徑/".repeat(24));
        app.update_panel.cli_install_modal_current_version = "1.2.3".into();
        app.update_panel.cli_install_modal_server_version = "1.2.4".into();
        app.update_panel.cli_install_modal_show_install_action = true;
        app.desktop_update.source = Some(desktop_release::UpdateSource::Shiju);
        let mut ui = scene(install_cli_modal::view_app_update(&app), &mut renderer, size);
        capture(&mut ui, &mut renderer, &theme, size, &format!("cli-{name}"), Point::ORIGIN);
        let controls = bounds(&mut ui, &renderer);
        let title = position(&controls.labels, i18n.__("安装 CLI 命令工具"));
        let check = position(&controls.labels, i18n.__("检测更新"));
        let (body, _) = controls.body.unwrap();
        assert_fixed_controls(title, body, check, size);
        assert!(matches!(
            click(&mut ui, &mut renderer, check.center()).as_slice(),
            [Message::Install(InstallMessage::View(ViewMessage::CheckCliToolUpdate))]
        ));
        let install = position(&controls.labels, i18n.__("安装 CLI"));
        assert_inside(install, viewport(size));
        assert!(matches!(
            click(&mut ui, &mut renderer, install.center()).as_slice(),
            [Message::Install(InstallMessage::View(ViewMessage::RunInstallCliTool))]
        ));

        // A narrow view must keep the shared source menu inside the overlay.
        if locale == DesktopLocale::TraditionalChinese {
            let [source] = controls.focus.as_slice() else { panic!("source select") };
            assert_eq!(source.height, 38.0);
            assert_inside(*source, body);
            assert!(click(&mut ui, &mut renderer, source.center()).is_empty());
            let menu = bounds(&mut ui, &renderer).menu.expect("source menu");
            assert_inside(menu, viewport(size));
            let first = Point::new(menu.center_x(), menu.y + 24.0);
            let second = Point::new(menu.center_x(), menu.y + 60.0);
            assert!(hover(&mut ui, &mut renderer, first).is_empty());
            let first_frame = capture(&mut ui, &mut renderer, &theme, size, "source-first", first);
            assert!(hover(&mut ui, &mut renderer, second).is_empty());
            let second_frame =
                capture(&mut ui, &mut renderer, &theme, size, "source-second", second);
            assert_ne!(first_frame, second_frame);
            assert!(hover(&mut ui, &mut renderer, Point::ORIGIN).is_empty());
            let exited =
                capture(&mut ui, &mut renderer, &theme, size, "source-exit", Point::ORIGIN);
            assert_ne!(second_frame, exited);
            assert!(key(&mut ui, &mut renderer, Key::Named(Named::Escape), None).is_empty());
            assert!(bounds(&mut ui, &renderer).menu.is_none());

            for key_name in [Named::Enter, Named::Home, Named::ArrowDown] {
                assert!(key(&mut ui, &mut renderer, Key::Named(key_name), None).is_empty());
            }
            assert!(matches!(
                key(&mut ui, &mut renderer, Key::Named(Named::Enter), None).as_slice(),
                [Message::Install(InstallMessage::View(ViewMessage::AppUpdateSourceSelected(
                    desktop_release::UpdateSource::Github
                )))]
            ));
        }

        assert!(scroll(&mut ui, &mut renderer, body.center(), -160.0).is_empty());
        let scrolled = bounds(&mut ui, &renderer);
        let offset = scrolled.body.unwrap().1;
        assert!(offset.y > 0.0);
        assert_eq!(position(&scrolled.labels, i18n.__("检测更新")), check);
        assert_eq!(position(&scrolled.labels, i18n.__("安装 CLI 命令工具")), title);
        let service = position(&scrolled.labels, i18n.__("服务管理")).center() - offset;
        assert!(body.contains(service));
        assert!(matches!(
            click(&mut ui, &mut renderer, service).as_slice(),
            [Message::Install(InstallMessage::View(ViewMessage::CliCommandsTabSelected(
                crate::components::cli_command_help::CliCommandsTab::Service
            )))]
        ));
        let copy = position(&scrolled.labels, i18n.__("复制")).center() - offset;
        assert!(body.contains(copy));
        assert!(matches!(
            click(&mut ui, &mut renderer, copy).as_slice(),
            [Message::Install(InstallMessage::CopyCode(command))] if command == "shiju --help"
        ));
        let close = position(&scrolled.labels, i18n.__("关闭"));
        assert!(matches!(
            click(&mut ui, &mut renderer, close.center()).as_slice(),
            [Message::Install(InstallMessage::View(ViewMessage::CloseInstallCliModal))]
        ));
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn ui_refresh_41_cli_preserves_values_and_disabled_states() {
    let mut app = fixture();
    app.desktop_locale = DesktopLocale::TraditionalChinese;
    let i18n = I18n::new(app.desktop_locale);
    let mut renderer = renderer().await;
    let size = Size::new(420, 520);
    app.update_panel.cli_install_modal_title = "安装 CLI 命令工具".into();
    app.update_panel.cli_install_modal_message = "正在请求所选来源的最新 CLI 版本...".into();
    app.update_panel.cli_install_modal_is_checking_update = true;
    app.update_panel.cli_install_modal_show_install_action = true;
    app.desktop_update.source = Some(desktop_release::UpdateSource::Github);
    app.desktop_update.busy = true;
    let mut ui = scene(install_cli_modal::view_app_update(&app), &mut renderer, size);
    let controls = bounds(&mut ui, &renderer);
    let checking = position(&controls.labels, i18n.__("检测中..."));
    let install = position(&controls.labels, i18n.__("安装 CLI"));
    let source = position(&controls.labels, "github.com");
    assert!(controls.focus.is_empty());
    for rect in [checking, install, source] {
        assert_inside(rect, viewport(size));
        assert!(click(&mut ui, &mut renderer, rect.center()).is_empty());
    }
    capture(&mut ui, &mut renderer, &Theme::Dark, size, "cli-checking", install.center());

    let failure = format!("安装失败：/tmp/{}/shiju", "測試路徑".repeat(40));
    let mut ui = scene(
        install_cli_modal::view_localized(i18n, "安装 CLI 命令工具", &failure),
        &mut renderer,
        size,
    );
    let controls = bounds(&mut ui, &renderer);
    let description = position(&controls.labels, &failure);
    let (body, _) = controls.body.unwrap();
    assert!(description.height > 40.0 && description.width < body.width);
    capture(&mut ui, &mut renderer, &Theme::Light, size, "cli-failure", Point::ORIGIN);
}
