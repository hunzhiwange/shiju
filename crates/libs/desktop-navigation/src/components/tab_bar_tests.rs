//! 验证标签栏筛选、关闭交互、标题截断和主题样式边界。
//!
//! 标签夹具覆盖内部页过滤、悬停关闭层与 Unicode 截断，避免索引和显示错位。

use super::*;
use super::strip::visible_tab_count;

use crate::components::top_bar::types::AppTab;
use iced::advanced::clipboard;
use iced::advanced::renderer::Headless;
use iced::advanced::widget::Tree;
use iced::advanced::{Layout, layout};
use iced::widget::button;
use iced::{Background, Color, Event, Font, Length, Pixels, Rectangle, Size, Theme, mouse, touch};
use iced_runtime::user_interface::{Cache, UserInterface};

fn test_app() -> State<'static> {
    static TABS: std::sync::OnceLock<Vec<AppTab>> = std::sync::OnceLock::new();
    static ACTIVE: std::sync::OnceLock<Option<String>> = std::sync::OnceLock::new();
    State {
        open_tabs: TABS.get_or_init(|| vec![tab("home", "首页", Screen::Home)]),
        active_tab_id: ACTIVE.get_or_init(|| Some("home".to_string())),
        hovered_tab_id: &None,
        screen: Screen::Home,
        desktop_locale: config_ui::DesktopLocale::SimplifiedChinese,
        overflow_open: false,
    }
}

fn element_tree(element: Element<'_, Message>) -> Tree {
    Tree::new(element.as_widget())
}

fn tab(id: &str, title: &str, screen: Screen) -> AppTab {
    AppTab { id: id.to_string(), title: title.to_string(), screen, project_path: None }
}

fn background_color(background: Option<Background>) -> Color {
    match background {
        Some(Background::Color(color)) => color,
        other => panic!("expected color background, got {other:?}"),
    }
}

fn max_child_count(tree: &Tree) -> usize {
    tree.children.iter().map(max_child_count).fold(tree.children.len(), usize::max)
}

#[test]
fn view_uses_public_height_and_full_width() {
    let app = test_app();
    let element = view(&app);

    assert_eq!(element.as_widget().size().width, Length::Fill);
    assert_eq!(element.as_widget().size().height, Length::Fixed(TAB_BAR_HEIGHT));
}

fn row_bounds(app: &State<'_>, renderer: &Renderer, width: f32) -> Vec<Rectangle> {
    let mut bar = view(app);
    let mut tree = Tree::new(bar.as_widget());
    let limits = layout::Limits::new(Size::ZERO, Size::new(width, TAB_BAR_HEIGHT));
    let node = bar.as_widget_mut().layout(&mut tree, renderer, &limits);
    Layout::new(&node)
        .children()
        .next()
        .unwrap()
        .children()
        .map(|item| item.bounds())
        .collect()
}

#[tokio::test(flavor = "multi_thread")]
async fn view_filters_internal_tabs_and_omits_add_button_when_apps_is_open() {
    let renderer = <Renderer as Headless>::new(Font::DEFAULT, Pixels(16.0), Some("tiny-skia"))
        .await
        .unwrap();
    assert_eq!(row_bounds(&test_app(), &renderer, 640.0).len(), 2);
    let mut tabs = vec![
        tab("home", "Home", Screen::Home),
        tab("find:internal", "Find", Screen::Home),
        tab("project", "Project", Screen::Project),
    ];
    let app = State { open_tabs: &tabs, ..test_app() };
    assert_eq!(row_bounds(&app, &renderer, 640.0).len(), 3);

    tabs.retain(|tab| tab.id != "project");
    tabs.push(tab("apps", "Apps", Screen::Apps));
    let app = State { open_tabs: &tabs, ..test_app() };
    assert_eq!(row_bounds(&app, &renderer, 640.0).len(), 2);
}

#[test]
fn overflow_uses_the_longest_fitting_prefix_and_reserves_controls() {
    let tabs = [56.0, 160.0, 60.0];
    assert_eq!(visible_tab_count(tabs, 307.0, true), 3);
    assert_eq!(visible_tab_count(tabs, 306.0, true), 2);
    assert_eq!(visible_tab_count(tabs, 277.0, true), 2);
    assert_eq!(visible_tab_count(tabs, 276.0, true), 1);
    assert_eq!(visible_tab_count(tabs, 113.0, true), 0);
    assert_eq!(visible_tab_count(tabs, 114.0, true), 1);
    assert_eq!(visible_tab_count(tabs, 252.0, false), 2);
    assert_eq!(visible_tab_count(tabs, 20.0, false), 0);
    assert_eq!(visible_tab_count([], 20.0, true), 0);
}

#[tokio::test(flavor = "multi_thread")]
async fn overflow_opens_at_the_end_and_selects_or_closes_hidden_tabs() {
    let mut renderer = <Renderer as Headless>::new(Font::DEFAULT, Pixels(16.0), Some("tiny-skia"))
        .await
        .unwrap();
    let tabs = [
        tab("home", "Home", Screen::Home),
        tab("find:internal", "Internal", Screen::Home),
        tab("project-a", "Project A", Screen::Project),
        tab("project-b", "Project B", Screen::Project),
    ];
    let active = Some("project-b".to_string());
    let app = State { open_tabs: &tabs, active_tab_id: &active, ..test_app() };
    let bounds = Size::new(170.0, 400.0);
    let narrow = row_bounds(&app, &renderer, bounds.width);
    assert_eq!(narrow.len(), 3); // Home, add, overflow
    assert_eq!(narrow[1].width, 22.0);
    assert!(narrow[2].x > narrow[1].x + narrow[1].width);
    assert!(narrow[2].x + narrow[2].width <= bounds.width);
    assert_eq!(row_bounds(&app, &renderer, 640.0).len(), 4);

    let click = |app: State<'_>, renderer: &mut Renderer, point| {
        let mut ui = UserInterface::build(view(&app), bounds, Cache::new(), renderer);
        let mut messages = Vec::new();
        ui.update(
            &[
                Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)),
                Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)),
            ],
            mouse::Cursor::Available(point),
            renderer,
            &mut clipboard::Null,
            &mut messages,
        );
        messages
    };
    let messages = click(app, &mut renderer, narrow[2].center());
    assert!(matches!(
        messages.as_slice(),
        [Message::ToggleMenu(Some(super::super::top_bar::Menu::TabOverflow))]
    ));

    let open = State { overflow_open: true, ..app };
    let messages = click(open, &mut renderer, Point::new(50.0, 52.0));
    assert!(matches!(messages.as_slice(), [Message::MenuAction(action)]
        if matches!(action.as_ref(), Message::TabSelected(id) if id == "project-a")));
    let messages = click(open, &mut renderer, Point::new(149.0, 80.0));
    assert!(matches!(messages.as_slice(), [Message::MenuAction(action)]
        if matches!(action.as_ref(), Message::TabClosed(id) if id == "project-b")));
}

#[tokio::test(flavor = "multi_thread")]
async fn open_overflow_captures_an_outside_press_over_a_visible_tab() {
    let mut renderer = <Renderer as Headless>::new(Font::DEFAULT, Pixels(16.0), Some("tiny-skia"))
        .await
        .unwrap();
    let tabs = [
        tab("home", "Home", Screen::Home),
        tab("project", "Project", Screen::Project),
        tab("usage", "Usage", Screen::Usage),
    ];
    let app = State { open_tabs: &tabs, overflow_open: true, ..test_app() };
    let bounds = Size::new(170.0, 400.0);
    let home = row_bounds(&app, &renderer, bounds.width)[0].center();
    for event in [
        Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)),
        Event::Touch(touch::Event::FingerPressed { id: touch::Finger(0), position: home }),
    ] {
        let cursor = if matches!(event, Event::Touch(_)) {
            mouse::Cursor::Unavailable
        } else {
            mouse::Cursor::Available(home)
        };
        let mut ui = UserInterface::build(view(&app), bounds, Cache::new(), &mut renderer);
        let mut messages = Vec::new();
        let (_, statuses) = ui.update(
            &[event],
            cursor,
            &mut renderer,
            &mut clipboard::Null,
            &mut messages,
        );
        assert_eq!(statuses, [iced::event::Status::Captured]);
        assert!(matches!(messages.as_slice(), [Message::ToggleMenu(None)]));
    }
}

#[test]
fn tab_button_close_layer_requires_hovered_non_home_tab() {
    let home = element_tree(tab_btn(
        "home".to_string(),
        "Home".to_string(),
        true,
        true,
        Message::TabSelected("home".to_string()),
    ));
    let idle = element_tree(tab_btn(
        "project".to_string(),
        "Project".to_string(),
        false,
        false,
        Message::TabSelected("project".to_string()),
    ));
    let hovered = element_tree(tab_btn(
        "project".to_string(),
        "Project".to_string(),
        false,
        true,
        Message::TabSelected("project".to_string()),
    ));

    assert_eq!(max_child_count(&home), 1);
    assert_eq!(max_child_count(&idle), 1);
    assert_eq!(max_child_count(&hovered), 2);
}

#[test]
fn tab_button_keeps_shrink_width_and_full_height() {
    let element = tab_btn(
        "project".to_string(),
        "Project".to_string(),
        false,
        false,
        Message::TabSelected("project".to_string()),
    );

    assert_eq!(element.as_widget().size().width, Length::Shrink);
    assert_eq!(element.as_widget().size().height, Length::Fill);
}

#[test]
fn display_tab_label_preserves_boundaries_and_truncates_unicode() {
    let home_boundary = "a".repeat(22);
    let regular_boundary = "界".repeat(24);

    assert_eq!(display_tab_label(&home_boundary, true), home_boundary);
    assert_eq!(
        display_tab_label(&format!("{home_boundary}b"), true),
        format!("{home_boundary}...")
    );
    assert_eq!(display_tab_label(&regular_boundary, false), regular_boundary);
    assert_eq!(
        display_tab_label(&format!("{regular_boundary}界"), false),
        format!("{regular_boundary}...")
    );
}

#[test]
fn dark_theme_detection_matches_builtin_themes() {
    assert!(is_dark_theme(&Theme::Dark));
    assert!(!is_dark_theme(&Theme::Light));
}

#[test]
fn add_button_style_only_highlights_hovered_status() {
    let dark_active = add_button_style(&Theme::Dark, button::Status::Active);
    let dark_hovered = add_button_style(&Theme::Dark, button::Status::Hovered);
    let light_hovered = add_button_style(&Theme::Light, button::Status::Hovered);

    assert!(dark_active.background.is_none());
    assert_eq!(dark_active.text_color, Theme::Dark.palette().text);
    assert_eq!(dark_active.border.radius.top_left, 999.0);
    assert_eq!(
        background_color(dark_hovered.background),
        Theme::Dark.extended_palette().background.strong.color.scale_alpha(0.8)
    );
    assert_eq!(
        background_color(light_hovered.background),
        Theme::Light.extended_palette().background.weak.color
    );
}

#[test]
fn tab_bar_style_switches_theme_border_and_project_visibility() {
    let dark = tab_bar_style(&Theme::Dark, false);
    let light_project = tab_bar_style(&Theme::Light, true);

    assert_eq!(
        background_color(dark.background),
        Theme::Dark.extended_palette().background.base.color
    );
    assert_eq!(dark.border.width, 1.0);
    assert_eq!(dark.border.color, Theme::Dark.extended_palette().background.strong.color);
    assert_eq!(light_project.border.width, 0.0);
    assert_eq!(light_project.border.color, Theme::Light.extended_palette().background.weak.color);
}

#[test]
fn tab_label_style_covers_active_hovered_and_idle_theme_branches() {
    let dark_active = tab_label_style(&Theme::Dark, true, false);
    let dark_hovered = tab_label_style(&Theme::Dark, false, true);
    let light_idle = tab_label_style(&Theme::Light, false, false);

    assert_eq!(
        background_color(dark_active.background),
        Theme::Dark.extended_palette().background.strong.color.scale_alpha(0.85)
    );
    assert_eq!(dark_active.text_color, Some(Theme::Dark.palette().text));
    assert_eq!(
        background_color(dark_hovered.background),
        Theme::Dark.extended_palette().background.weak.color.scale_alpha(0.7)
    );
    assert_eq!(dark_hovered.text_color, Some(Theme::Dark.palette().text.scale_alpha(0.78)));
    assert_eq!(background_color(light_idle.background), Color::TRANSPARENT);
    assert_eq!(light_idle.text_color, Some(Theme::Light.palette().text.scale_alpha(0.55)));
    assert_eq!(light_idle.border.radius.top_left, 6.0);
    assert_eq!(light_idle.border.radius.bottom_left, 0.0);
}

#[tokio::test(flavor = "multi_thread")]
async fn close_corner_triangle_draws_valid_bounds_and_handles_zero_width() {
    let renderer =
        <iced::Renderer as Headless>::new(Font::DEFAULT, Pixels(16.0), Some("tiny-skia"))
            .await
            .expect("tiny-skia headless renderer should be available");
    let triangle = CloseCornerTriangle;

    let empty = triangle.draw(
        &(),
        &renderer,
        &Theme::Dark,
        Rectangle::new(iced::Point::ORIGIN, iced::Size::new(0.0, 18.0)),
        iced::mouse::Cursor::Unavailable,
    );
    let drawn = triangle.draw(
        &(),
        &renderer,
        &Theme::Dark,
        Rectangle::new(iced::Point::ORIGIN, iced::Size::new(18.0, 18.0)),
        iced::mouse::Cursor::Unavailable,
    );

    assert_eq!(empty.len(), 1);
    assert_eq!(drawn.len(), 1);
}

#[tokio::test(flavor = "multi_thread")]
async fn view_draws_unicode_tabs_and_selects_home_in_both_themes() {
    {
        let mut fonts = iced::advanced::graphics::text::font_system().write().unwrap();
        for font in desktop_assets::fonts::load_all() {
            fonts.load_font(font);
        }
    }
    let mut renderer = <iced::Renderer as Headless>::new(
        Font::with_name("Noto Sans CJK SC"), Pixels(16.0), Some("tiny-skia"),
    ).await.unwrap();
    let tabs = vec![tab("home", "首页", Screen::Home), tab("project", "项目 Project", Screen::Project)];
    let hovered = Some("project".to_string());
    let app = State { open_tabs: &tabs, hovered_tab_id: &hovered, ..test_app() };
    let size = Size::new(640, TAB_BAR_HEIGHT as u32);
    for theme in [Theme::Dark, Theme::Light] {
        let mut ui = UserInterface::build(
            view(&app), Size::new(size.width as f32, size.height as f32), Cache::new(), &mut renderer,
        );
        let cursor = mouse::Cursor::Available(Point::new(20.0, 18.0));
        let mut messages = Vec::new();
        let _ = ui.update(
            &[
                Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)),
                Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)),
            ], cursor, &mut renderer, &mut clipboard::Null, &mut messages,
        );
        assert!(messages.iter().any(|message| matches!(message, Message::TabSelected(id) if id == "home")), "{messages:?}");
        ui.draw(&mut renderer, &theme, &iced::advanced::renderer::Style { text_color: theme.palette().text }, cursor);
        let pixels = renderer.screenshot(size, 1.0, Color::TRANSPARENT);
        assert!(pixels.as_chunks::<4>().0.iter().any(|pixel| pixel[3] > 0));
    }
}
