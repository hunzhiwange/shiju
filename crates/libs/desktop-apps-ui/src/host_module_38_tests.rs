use crate::bookmark_modals::{self, Event as ViewMessage};
use config_ui::DesktopLocale;
use desktop_i18n::I18n;
use iced::advanced::widget::{Id, Operation, operation};
use iced::advanced::{clipboard, renderer::Headless};
use iced::keyboard::{self, Key, key::Named};
use iced::{Event, Font, Point, Rectangle, Renderer, Size, Theme, Vector, mouse};
use iced_runtime::user_interface::{Cache, UserInterface};
#[derive(Debug, Clone)]
enum Message {
    View(ViewMessage),
}
type Ui<'a> = UserInterface<'a, Message, Theme, Renderer>;
struct Fixture {
    desktop_locale: DesktopLocale,
    web_navigation: crate::web::State,
}
fn fixture() -> Fixture {
    Fixture { desktop_locale: DesktopLocale::SimplifiedChinese, web_navigation: Default::default() }
}
mod apps {
    use super::*;
    pub fn view(app: &Fixture) -> iced::Element<'_, Message> {
        let web = &app.web_navigation;
        let i18n = I18n::new(app.desktop_locale);
        let editing = web.editing_web_bookmark.is_some();
        use desktop_ui_controls::system_settings_common::{
            settings_field_label, settings_group_heading_with_help, settings_hover_help_label,
        };
        let title = if editing { "书签信息" } else { "新增网址书签" };
        let description = if editing {
            "设置标题、网址以及独立窗口的默认尺寸。"
        } else {
            "保存一个可在应用中心快速打开的网页入口。"
        };
        let label = i18n.__("Cookie配置(JSON)");
        let input = bookmark_modals::Input {
            i18n,
            message: Message::View,
            editing_web_bookmark: web.editing_web_bookmark,
            heading: settings_group_heading_with_help(
                i18n,
                &Theme::Dark,
                i18n.__(title),
                i18n.__(description),
            )
            .into(),
            cookie_help: settings_hover_help_label(
                i18n,
                &Theme::Dark,
                label,
                settings_field_label(label),
                format!(
                    "{}\n\n{}\n{}",
                    i18n.__("支持按 JSON 为指定站点注入 Cookie。"),
                    i18n.__("配置示例"),
                    bookmark_modals::WEB_BOOKMARK_COOKIE_CONFIG_EXAMPLE
                ),
            )
            .wrap()
            .into(),
            edit_web_bookmark_cookie_configs_editor: &web.edit_web_bookmark_cookie_configs_editor,
            web_bookmark_title_input: &web.web_bookmark_title_input,
            web_bookmark_url_input: &web.web_bookmark_url_input,
            web_bookmark_width_input: &web.web_bookmark_width_input,
            web_bookmark_height_input: &web.web_bookmark_height_input,
            edit_web_bookmark_title_input: &web.edit_web_bookmark_title_input,
            edit_web_bookmark_url_input: &web.edit_web_bookmark_url_input,
            edit_web_bookmark_width_input: &web.edit_web_bookmark_width_input,
            edit_web_bookmark_height_input: &web.edit_web_bookmark_height_input,
        };
        let modal = if editing {
            bookmark_modals::edit_web_bookmark_modal(input)
        } else {
            bookmark_modals::add_web_bookmark_modal(input)
        };
        crate::apps_view::view(
            iced::widget::Space::new().into(),
            iced::widget::Space::new().into(),
            modal,
        )
    }
}
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

fn assert_input_center(
    mut view: Element<'_, Message>,
    renderer: &Renderer,
    input: Rectangle,
    size: Size<u32>,
) {
    fn find(
        layout: iced::advanced::Layout<'_>,
        input: Rectangle,
    ) -> Option<iced::advanced::Layout<'_>> {
        if layout.bounds() == input {
            Some(layout)
        } else {
            layout.children().find_map(|child| find(child, input))
        }
    }

    let mut tree = iced::advanced::widget::Tree::new(view.as_widget());
    let node = view.as_widget_mut().layout(
        &mut tree,
        renderer,
        &iced::advanced::layout::Limits::new(Size::ZERO, viewport(size).size()),
    );
    let layout = find(iced::advanced::Layout::new(&node), input).expect("bookmark input layout");
    let label = layout.children().next().expect("input text layout").bounds();
    assert!((input.center_y() - label.center_y()).abs() < 0.5, "{input:?} {label:?}");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn ui_refresh_41_preserves_values_and_disabled_states() {
    let mut app = fixture();
    app.desktop_locale = DesktopLocale::TraditionalChinese;
    let i18n = I18n::new(app.desktop_locale);
    let mut renderer = renderer().await;
    app.web_navigation.show_web_links_menu = true;
    let size = Size::new(420, 520);
    let mut ui = scene(apps::view(&app), &mut renderer, size);
    let add = position(&bounds(&mut ui, &renderer).labels, i18n.__("添加"));
    assert!(click(&mut ui, &mut renderer, add.center()).is_empty());
    capture(&mut ui, &mut renderer, &Theme::Dark, size, "bookmark-empty", add.center());
    drop(ui);

    app.web_navigation.web_bookmark_title_input = "專案書籤".into();
    app.web_navigation.web_bookmark_url_input =
        format!("https://example.test/{}", "project/".repeat(24));
    app.web_navigation.web_bookmark_width_input = "640".into();
    app.web_navigation.web_bookmark_height_input = "480".into();
    let mut ui = scene(apps::view(&app), &mut renderer, size);
    capture(&mut ui, &mut renderer, &Theme::Dark, size, "bookmark-add", Point::ORIGIN);
    let controls = bounds(&mut ui, &renderer);
    let (body, _) = controls.body.unwrap();
    for value in [
        &app.web_navigation.web_bookmark_title_input,
        &app.web_navigation.web_bookmark_url_input,
        &app.web_navigation.web_bookmark_width_input,
        &app.web_navigation.web_bookmark_height_input,
    ] {
        let input = position(&controls.inputs, value);
        assert_inside(input, Rectangle { x: body.x + 7.0, width: body.width - 14.0, ..body });
        assert_input_center(apps::view(&app), &renderer, input, size);
    }
    assert!(matches!(
        click(&mut ui, &mut renderer, position(&controls.labels, i18n.__("添加")).center())
            .as_slice(),
        [Message::View(ViewMessage::WebBookmarkAddSave)]
    ));
    assert!(matches!(
        click(&mut ui, &mut renderer, position(&controls.labels, i18n.__("取消")).center())
            .as_slice(),
        [Message::View(ViewMessage::ToggleWebLinksMenu)]
    ));
    drop(ui);

    app.web_navigation.editing_web_bookmark = Some(3);
    app.web_navigation.edit_web_bookmark_title_input =
        app.web_navigation.web_bookmark_title_input.clone();
    app.web_navigation.edit_web_bookmark_url_input =
        app.web_navigation.web_bookmark_url_input.clone();
    app.web_navigation.edit_web_bookmark_cookie_configs_editor =
        iced::widget::text_editor::Content::with_text(
            r#"[{"name":"fixture","domain":"example.test"}]"#,
        );
    let mut ui = scene(apps::view(&app), &mut renderer, size);
    capture(&mut ui, &mut renderer, &Theme::Light, size, "bookmark-edit", Point::ORIGIN);
    let controls = bounds(&mut ui, &renderer);
    let (body, _) = controls.body.expect("bookmark form scroll viewport");
    assert!(
        body.height < size.height as f32,
        "the modal footer needs space outside the scroll viewport"
    );
    let title = position(&controls.labels, i18n.__("编辑网址"));
    let save = position(&controls.labels, i18n.__("保存"));
    let cancel = position(&controls.labels, i18n.__("取消"));
    let url = position(&controls.inputs, &app.web_navigation.edit_web_bookmark_url_input);
    let name = position(&controls.inputs, &app.web_navigation.edit_web_bookmark_title_input);
    assert_fixed_controls(title, body, save, size);
    assert!(click(&mut ui, &mut renderer, name.center()).is_empty());
    assert!(key(&mut ui, &mut renderer, Key::Named(Named::End), None).is_empty());
    assert!(matches!(
        key(&mut ui, &mut renderer, Key::Character("A".into()), Some("A")).as_slice(),
        [Message::View(ViewMessage::WebBookmarkEditTitleChanged(value))]
            if value == &format!("{}A", app.web_navigation.edit_web_bookmark_title_input)
    ));
    assert!(click(&mut ui, &mut renderer, url.center()).is_empty());
    assert!(key(&mut ui, &mut renderer, Key::Named(Named::End), None).is_empty());
    assert!(matches!(
        key(&mut ui, &mut renderer, Key::Character("x".into()), Some("x")).as_slice(),
        [Message::View(ViewMessage::WebBookmarkEditUrlChanged(value))]
            if value == &format!("{}x", app.web_navigation.edit_web_bookmark_url_input)
    ));
    for key_name in [Named::Enter, Named::Escape] {
        assert!(key(&mut ui, &mut renderer, Key::Named(key_name), None).is_empty());
    }
    assert!(scroll(&mut ui, &mut renderer, body.center(), -800.0).is_empty());
    let scrolled = bounds(&mut ui, &renderer);
    let offset = scrolled.body.unwrap().1;
    assert!(offset.y > 0.0);
    assert_eq!(position(&scrolled.labels, i18n.__("编辑网址")), title);
    assert_eq!(position(&scrolled.labels, i18n.__("保存")), save);
    assert_eq!(position(&scrolled.labels, i18n.__("取消")), cancel);
    let example = position(&scrolled.labels, i18n.__("插入例子")).center() - offset;
    assert!(body.contains(example));
    capture(&mut ui, &mut renderer, &Theme::Light, size, "bookmark-edit-scrolled", example);
    assert!(matches!(
        click(&mut ui, &mut renderer, example).as_slice(),
        [Message::View(ViewMessage::WebBookmarkEditCookieConfigsInsertExample)]
    ));
    assert!(matches!(
        click(&mut ui, &mut renderer, save.center()).as_slice(),
        [Message::View(ViewMessage::WebBookmarkEditSave)]
    ));
    assert!(matches!(
        click(&mut ui, &mut renderer, cancel.center()).as_slice(),
        [Message::View(ViewMessage::WebBookmarkEditCancel)]
    ));
    let delete = position(&scrolled.labels, i18n.__("删除"));
    assert!(matches!(
        click(&mut ui, &mut renderer, delete.center()).as_slice(),
        [Message::View(ViewMessage::WebBookmarkRemove(3))]
    ));
    let editor = *scrolled.focus.last().expect("cookie editor");
    let with_cookie =
        capture(&mut ui, &mut renderer, &Theme::Light, size, "cookie-content", Point::ORIGIN);
    drop(ui);

    app.web_navigation.edit_web_bookmark_cookie_configs_editor =
        iced::widget::text_editor::Content::new();
    let mut ui = scene(apps::view(&app), &mut renderer, size);
    assert!(scroll(&mut ui, &mut renderer, body.center(), -800.0).is_empty());
    let empty_cookie =
        capture(&mut ui, &mut renderer, &Theme::Light, size, "cookie-empty", Point::ORIGIN);
    let width = size.width as usize;
    let x = (editor.x + editor.width).ceil() as usize + 1;
    let right = (body.x + body.width - 8.0).floor() as usize;
    let y = (editor.y - offset.y).ceil() as usize + 1;
    assert!(x < right);
    for row in y..y + 42 {
        let strip = ((row * width + x) * 4)..((row * width + right) * 4);
        assert_eq!(
            &with_cookie[strip.clone()],
            &empty_cookie[strip],
            "cookie text spills outside its editor at row {row}"
        );
    }
    drop(ui);
}
