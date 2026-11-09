//! 应用中心弹窗测试，覆盖书签新增编辑、激活优先级和提交条件。
//! 选择索引失效时仍需安全构建界面，且 Cookie 示例必须展示必要字段契约。

use crate::app::message::ViewMessage;
use crate::app::{App, Message};
use iced::advanced::renderer::{Headless, Style};
use iced::advanced::{clipboard, widget};
use iced::keyboard::{self, Key, key::Named};
use iced::{Event, Font, Point, Rectangle, Renderer, Size, Theme, mouse};
use iced_runtime::user_interface::{Cache, UserInterface};
use vw_config_types::ui::DesktopLocale;

fn new_app() -> App {
    App::new().0
}

#[test]
fn active_modal_returns_none_without_modal_state() {
    let app = new_app();
    assert!(super::active_modal(&app).is_none());
}

#[test]
fn active_modal_prefers_edit_modal_over_add_modal() {
    let mut app = new_app();
    app.show_web_links_menu = true;
    app.editing_web_bookmark = Some(2);

    let Some((_modal, close)) = super::active_modal(&app) else {
        panic!("expected edit modal");
    };
    assert!(matches!(close, Message::View(ViewMessage::WebBookmarkEditCancel)));
}

#[test]
fn active_modal_returns_add_bookmark_close_message() {
    let mut app = new_app();
    app.show_web_links_menu = true;

    let Some((_modal, close)) = super::active_modal(&app) else {
        panic!("expected add modal");
    };
    assert!(matches!(close, Message::View(ViewMessage::ToggleWebLinksMenu)));
}

#[test]
fn can_add_web_bookmark_requires_non_blank_title() {
    assert!(!super::can_add_web_bookmark("", "https://example.com"));
    assert!(!super::can_add_web_bookmark(" \n\t", "https://example.com"));
}

#[test]
fn can_add_web_bookmark_requires_non_blank_url() {
    assert!(!super::can_add_web_bookmark("Example", ""));
    assert!(!super::can_add_web_bookmark("Example", " \n\t"));
}

#[test]
fn can_add_web_bookmark_accepts_trimmed_non_empty_fields() {
    assert!(super::can_add_web_bookmark("  Example  ", "  https://example.com  "));
}

#[test]
fn add_web_bookmark_modal_builds_for_incomplete_input() {
    let app = new_app();

    let Some((_modal, close)) = super::add_web_bookmark_modal(&app) else {
        panic!("expected add modal");
    };
    assert!(matches!(close, Message::View(ViewMessage::ToggleWebLinksMenu)));
}

#[test]
fn add_web_bookmark_modal_builds_for_valid_input() {
    let mut app = new_app();
    app.web_bookmark_title_input = "Example".into();
    app.web_bookmark_url_input = "https://example.com".into();

    let Some((_modal, close)) = super::add_web_bookmark_modal(&app) else {
        panic!("expected add modal");
    };
    assert!(matches!(close, Message::View(ViewMessage::ToggleWebLinksMenu)));
}

#[test]
fn edit_web_bookmark_modal_builds_for_selected_bookmark() {
    let mut app = new_app();
    app.editing_web_bookmark = Some(7);

    let Some((_modal, close)) = super::edit_web_bookmark_modal(&app) else {
        panic!("expected edit modal");
    };
    assert!(matches!(close, Message::View(ViewMessage::WebBookmarkEditCancel)));
}

#[test]
fn edit_web_bookmark_modal_uses_safe_index_fallback_without_selection() {
    let app = new_app();

    let Some((_modal, close)) = super::edit_web_bookmark_modal(&app) else {
        panic!("expected edit modal");
    };
    assert!(matches!(close, Message::View(ViewMessage::WebBookmarkEditCancel)));
}

#[test]
fn cookie_config_example_documents_required_cookie_fields() {
    let example = super::WEB_BOOKMARK_COOKIE_CONFIG_EXAMPLE;

    assert!(example.starts_with('['));
    assert!(example.ends_with(']'));
    for field in ["\"name\"", "\"domain\"", "\"days\"", "\"url_filter\""] {
        assert!(example.contains(field));
    }
}

fn run_in_fixture(test_name: &str) -> bool {
    const CHILD: &str = "SHIJU_HELP_ROLLOUT_04_BOOKMARK_CHILD";
    if std::env::var_os(CHILD).is_none() {
        let home = tempfile::tempdir().unwrap();
        std::fs::write(
            home.path().join("gateway-client-bootstrap.json"),
            br#"{"host":"[","servers":[]}"#,
        )
        .unwrap();
        let module = module_path!().split_once("::").unwrap().1;
        let output = std::process::Command::new(std::env::current_exe().unwrap())
            .args(["--exact", &format!("{module}::{test_name}")])
            .env(CHILD, "1")
            .env("HOME", home.path())
            .env("VIBEWINDOW_CONFIG_DIR", home.path())
            .env("VIBEWINDOW_WORKSPACE", home.path())
            .current_dir(home.path())
            .output()
            .unwrap();
        assert!(
            output.status.success()
                && String::from_utf8_lossy(&output.stdout).contains("1 passed; 0 failed"),
            "{}\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        return false;
    }
    true
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn help_rollout_04_bookmarks_render_contextual_help() {
    use crate::app::components::system_settings_apps::preferences_tests::{
        HELP_LOCALES, verify_help,
    };

    if !run_in_fixture("help_rollout_04_bookmarks_render_contextual_help") {
        return;
    }

    let mut app = new_app();
    app.show_web_links_menu = true;
    app.web_bookmark_title_input = "Fixture bookmark".into();
    app.web_bookmark_url_input = "https://example.invalid/fixture".into();
    for mode in &HELP_LOCALES {
        app.desktop_locale = mode.0;
        app.app_theme = mode.1.clone();
        app.editing_web_bookmark = None;
        let add = verify_help(
            super::super::view(&app),
            "bookmark-add-help",
            mode,
            &[("新增网址书签", "保存一个可在应用中心快速打开的网页入口。")],
            &["标题", "网址"],
            "添加",
        )
        .await;
        assert!(matches!(add.as_slice(), [Message::View(ViewMessage::WebBookmarkAddSave)]));

        app.editing_web_bookmark = Some(0);
        let edit = verify_help(
            super::super::view(&app),
            "bookmark-edit-help",
            mode,
            &[
                ("书签信息", "设置标题、网址以及独立窗口的默认尺寸。"),
                ("Cookie配置(JSON)", "支持按 JSON 为指定站点注入 Cookie。"),
            ],
            &["Cookie 配置", "插入例子"],
            "保存",
        )
        .await;
        assert!(matches!(edit.as_slice(), [Message::View(ViewMessage::WebBookmarkEditSave)]));
    }
}

#[derive(Default)]
struct BookmarkLabels(Vec<(String, Rectangle)>);

impl widget::Operation for BookmarkLabels {
    fn traverse(&mut self, children: &mut dyn FnMut(&mut dyn widget::Operation)) {
        children(self);
    }

    fn text(&mut self, _: Option<&widget::Id>, bounds: Rectangle, value: &str) {
        self.0.push((value.into(), bounds));
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn settings_help_gap_18_cookie_example_pins_and_copies_static_text() {
    use crate::app::i18n::I18n;

    if !run_in_fixture("settings_help_gap_18_cookie_example_pins_and_copies_static_text") {
        return;
    }

    {
        let mut fonts = iced::advanced::graphics::text::font_system().write().unwrap();
        for font in crate::fonts::load_all() {
            fonts.load_font(font);
        }
    }
    let mut renderer =
        Renderer::new(Font::with_name("Noto Sans CJK SC"), 14.0.into(), Some("tiny-skia"))
            .await
            .unwrap();
    let mut app = new_app();
    app.show_web_links_menu = true;
    const DRAFT: &str = "fixture-cookie-secret";
    app.edit_web_bookmark_cookie_configs_editor =
        iced::widget::text_editor::Content::with_text(DRAFT);

    for locale in [
        DesktopLocale::SimplifiedChinese,
        DesktopLocale::TraditionalChinese,
        DesktopLocale::English,
    ] {
        app.desktop_locale = locale;
        let i18n = I18n::new(locale);
        let field = i18n.__("Cookie配置(JSON)");
        let guidance = i18n.__("支持按 JSON 为指定站点注入 Cookie。");
        let example = super::WEB_BOOKMARK_COOKIE_CONFIG_EXAMPLE;
        let help = format!("{guidance}\n\n{}\n{example}", i18n.__("配置示例"));

        for theme in [Theme::Light, Theme::Dark] {
            app.app_theme = theme.clone();
            for width in [420, 640] {
                let size = Size::new(width, 650);
                let viewport = Size::new(size.width as f32, size.height as f32);
                app.editing_web_bookmark = None;
                let mut add = UserInterface::build(
                    super::super::view(&app),
                    viewport,
                    Cache::new(),
                    &mut renderer,
                );
                let mut add_labels = BookmarkLabels::default();
                add.operate(&renderer, &mut add_labels);
                assert!(add_labels.0.iter().all(|(label, _)| {
                    label != field
                        && label != i18n.__("Cookie 配置")
                        && label != i18n.__("插入例子")
                }));
                drop(add);

                app.editing_web_bookmark = Some(0);
                let mut edit = UserInterface::build(
                    super::super::view(&app),
                    viewport,
                    Cache::new(),
                    &mut renderer,
                );
                let mut edit_labels = BookmarkLabels::default();
                edit.operate(&renderer, &mut edit_labels);
                let bounds =
                    |label| edit_labels.0.iter().rfind(|(value, _)| value == label).unwrap().1;
                assert!(
                    edit_labels.0.iter().all(|(label, _)| label != guidance && label != example)
                );
                let label = bounds(field);
                let trigger = Point::new(label.x + label.width + 15.0, label.center_y());
                let render = |edit: &mut UserInterface<'_, Message, Theme, Renderer>,
                              renderer: &mut Renderer,
                              cursor| {
                    edit.draw(
                        renderer,
                        &theme,
                        &Style { text_color: theme.palette().text },
                        cursor,
                    );
                    renderer.screenshot(size, 1.0, theme.palette().background)
                };
                let send = |edit: &mut UserInterface<'_, Message, Theme, Renderer>,
                            renderer: &mut Renderer,
                            cursor,
                            events: &[Event]| {
                    let mut messages = Vec::new();
                    edit.update(events, cursor, renderer, &mut clipboard::Null, &mut messages);
                    messages
                };
                let click = |edit: &mut UserInterface<'_, Message, Theme, Renderer>,
                             renderer: &mut Renderer,
                             point,
                             button| {
                    send(
                        edit,
                        renderer,
                        mouse::Cursor::Available(point),
                        &[
                            Event::Mouse(mouse::Event::ButtonPressed(button)),
                            Event::Mouse(mouse::Event::ButtonReleased(button)),
                        ],
                    )
                };
                let away = mouse::Cursor::Unavailable;
                let idle = render(&mut edit, &mut renderer, away);
                let hover = mouse::Cursor::Available(trigger);
                assert!(
                    send(
                        &mut edit,
                        &mut renderer,
                        hover,
                        &[Event::Mouse(mouse::Event::CursorMoved { position: trigger }),]
                    )
                    .is_empty()
                );
                assert!(render(&mut edit, &mut renderer, hover) != idle, "help opens on hover");
                assert!(click(&mut edit, &mut renderer, trigger, mouse::Button::Left).is_empty());
                assert!(
                    send(
                        &mut edit,
                        &mut renderer,
                        away,
                        &[Event::Mouse(mouse::Event::CursorLeft),]
                    )
                    .is_empty()
                );
                let pinned = render(&mut edit, &mut renderer, away);
                assert!(pinned != idle, "Cookie help must stay pinned when the pointer leaves");

                // The card is 300 px wide when space permits, and opens on the roomier side
                // when neither side has that much room.
                let left = trigger.x - 11.0 - 4.0;
                let right = trigger.x + 11.0 + 4.0;
                let space_right = viewport.width - right;
                let card_x = if space_right >= 300.0 || space_right >= left {
                    right
                } else {
                    (left - 300.0).max(0.0)
                };
                let start = Point::new(card_x + 13.0, trigger.y + 7.0);
                let copy = Point::new(start.x + 24.0, start.y + 16.0);
                let escape = Event::Keyboard(keyboard::Event::KeyPressed {
                    key: Key::Named(Named::Escape),
                    modified_key: Key::Named(Named::Escape),
                    physical_key: keyboard::key::Physical::Code(keyboard::key::Code::Escape),
                    location: keyboard::Location::Standard,
                    modifiers: keyboard::Modifiers::empty(),
                    text: None,
                    repeat: false,
                });
                assert!(click(&mut edit, &mut renderer, start, mouse::Button::Right).is_empty());
                assert!(render(&mut edit, &mut renderer, away) != pinned, "right click opens menu");
                assert!(
                    send(&mut edit, &mut renderer, away, std::slice::from_ref(&escape)).is_empty()
                );
                assert!(
                    render(&mut edit, &mut renderer, away) == pinned,
                    "Esc closes the menu first"
                );

                assert!(click(&mut edit, &mut renderer, start, mouse::Button::Right).is_empty());
                assert!(matches!(
                    click(&mut edit, &mut renderer, copy, mouse::Button::Left).as_slice(),
                    [Message::CopyCode(value)] if value == &help && !value.contains(DRAFT)
                ));
                // Continue past the last line so wrapping in each locale cannot
                // move the end of the example below the drag endpoint.
                let end = Point::new(start.x + 65.0, viewport.height - 1.0);
                for (point, event) in [
                    (start, mouse::Event::ButtonPressed(mouse::Button::Left)),
                    (end, mouse::Event::CursorMoved { position: end }),
                    (end, mouse::Event::ButtonReleased(mouse::Button::Left)),
                ] {
                    assert!(
                        send(
                            &mut edit,
                            &mut renderer,
                            mouse::Cursor::Available(point),
                            &[Event::Mouse(event),]
                        )
                        .is_empty()
                    );
                }
                assert!(
                    render(&mut edit, &mut renderer, away) != pinned,
                    "{locale:?}, {theme:?}, {width}px: dragging highlights the selection"
                );
                assert!(click(&mut edit, &mut renderer, start, mouse::Button::Right).is_empty());
                let selected = click(&mut edit, &mut renderer, copy, mouse::Button::Left);
                let card_text = format!("{field}\n{help}");
                assert!(
                    matches!(
                        selected.as_slice(),
                        [Message::CopyCode(value)]
                            if card_text.ends_with(value.as_str())
                                && value.contains("\"domain\"")
                                && value.contains("\"url_filter\"")
                    ),
                    "{locale:?}, {theme:?}, {width}px: drag should copy multiple example lines, got {selected:?}"
                );
                assert!(send(&mut edit, &mut renderer, away, &[escape]).is_empty());
                assert!(render(&mut edit, &mut renderer, away) == idle, "Esc closes the help card");
                assert!(matches!(
                    click(
                        &mut edit,
                        &mut renderer,
                        bounds(i18n.__("插入例子")).center(),
                        mouse::Button::Left
                    )
                    .as_slice(),
                    [Message::View(ViewMessage::WebBookmarkEditCookieConfigsInsertExample)]
                ));
                assert!(matches!(
                    click(
                        &mut edit,
                        &mut renderer,
                        bounds(i18n.__("删除")).center(),
                        mouse::Button::Left
                    )
                    .as_slice(),
                    [Message::View(ViewMessage::WebBookmarkRemove(0))]
                ));
            }
        }
    }
}
