//! 验证顶部栏窗口控件在不同平台与窗口状态下的行为。
//!
//! 平台与窗口状态分开构造，确保最小化、最大化和关闭动作保持稳定映射。

use super::controls;
use crate::app::state::WorkspaceMode;
use crate::app::{App, Message, Screen};
use iced::advanced::renderer::Headless;
use iced::advanced::{Layout, Shell, clipboard, layout, mouse, overlay, renderer, widget::Tree};
use iced::{Color, Element, Event, Font, Length, Rectangle, Renderer, Size, Theme, Vector};

#[cfg(not(target_arch = "wasm32"))]
const PROJECT_CONTROL_COUNT: usize = 6;
#[cfg(target_arch = "wasm32")]
const PROJECT_CONTROL_COUNT: usize = 4;
#[cfg(not(target_arch = "wasm32"))]
const WORK_PROJECT_CONTROL_COUNT: usize = 5;
#[cfg(target_arch = "wasm32")]
const WORK_PROJECT_CONTROL_COUNT: usize = 3;

fn test_app() -> App {
    let (app, _task) = App::new();
    app
}

fn assert_project_toolbar_shape(element: &Element<'_, Message>) {
    assert_eq!(element.as_widget().size(), Size::new(Length::Shrink, Length::Shrink));

    let controls = element.as_widget().children();
    assert_eq!(controls.len(), PROJECT_CONTROL_COUNT);
    assert!(controls.iter().all(|control| control.children.len() == 2));
}

fn assert_work_project_toolbar_shape(element: &Element<'_, Message>) {
    assert_eq!(element.as_widget().size(), Size::new(Length::Shrink, Length::Shrink));

    let controls = element.as_widget().children();
    assert_eq!(controls.len(), WORK_PROJECT_CONTROL_COUNT);
    assert!(controls.iter().all(|control| control.children.len() == 2));
}

#[test]
fn settings_button_builds_compact_tooltip_control() {
    let element = controls::settings_button(crate::app::i18n::I18n::new(
        vw_config_types::ui::DesktopLocale::English,
    ));

    assert_eq!(element.as_widget().size(), Size::new(Length::Shrink, Length::Fixed(24.0)));
    assert_eq!(element.as_widget().children().len(), 2);
}

#[tokio::test]
async fn settings_tooltip_draws_readable_chinese_text_on_dark_background() {
    let theme = Theme::custom(
        "Low contrast",
        iced::theme::Palette {
            background: Color::from_rgb8(16, 16, 16),
            text: Color::from_rgb8(32, 32, 32),
            ..Theme::Dark.palette()
        },
    );
    iced::advanced::graphics::text::font_system()
        .write()
        .unwrap()
        .load_font(std::borrow::Cow::Borrowed(crate::fonts::NOTO_SANS_CJK_SC_REGULAR));
    let mut renderer =
        Renderer::new(Font::with_name("Noto Sans CJK SC"), 16.0.into(), Some("tiny-skia"))
            .await
            .unwrap();
    let viewport = Rectangle::with_size(Size::new(180.0, 90.0));
    let mut button = controls::settings_button(crate::app::i18n::I18n::new(
        vw_config_types::ui::DesktopLocale::SimplifiedChinese,
    ));
    let mut tree = Tree::new(button.as_widget());
    let node = button.as_widget_mut().layout(
        &mut tree,
        &renderer,
        &layout::Limits::new(Size::ZERO, viewport.size()),
    );
    let root = Layout::new(&node);
    let position = root.bounds().center();
    let cursor = mouse::Cursor::Available(position);
    button.as_widget_mut().update(
        &mut tree,
        &Event::Mouse(mouse::Event::CursorMoved { position }),
        root,
        cursor,
        &renderer,
        &mut clipboard::Null,
        &mut Shell::new(&mut Vec::new()),
        &viewport,
    );
    let overlay = button
        .as_widget_mut()
        .overlay(&mut tree, root, &renderer, &viewport, Vector::ZERO)
        .expect("hovered settings tooltip");
    let mut overlay = overlay::Nested::new(overlay);
    let node = overlay.layout(&renderer, viewport.size());
    overlay.draw(
        &mut renderer,
        &theme,
        &renderer::Style { text_color: Color::BLACK },
        Layout::new(&node),
        cursor,
    );

    let pixels = renderer.screenshot(Size::new(180, 90), 1.0, theme.palette().background);
    assert!(pixels.chunks_exact(4).any(|pixel| pixel[..3].iter().all(|&value| value > 160)));
}

#[test]
fn project_view_tools_returns_empty_space_for_every_non_project_screen() {
    let mut app = test_app();
    let non_project_screens = [
        Screen::Home,
        Screen::Preview,
        Screen::Apps,
        Screen::Usage,
        Screen::Knowledge,
        Screen::WorkflowTool,
        Screen::WorkflowManager,
        Screen::TaskBoard,
    ];

    for screen in non_project_screens {
        app.screen = screen;
        let element = controls::project_view_tools(&app);

        assert_eq!(element.as_widget().size(), Size::new(Length::Shrink, Length::Shrink));
        assert!(element.as_widget().children().is_empty());
    }
}

#[test]
fn project_view_tools_builds_project_controls_when_panels_are_hidden() {
    let mut app = test_app();
    app.screen = Screen::Project;
    app.workspace_mode = WorkspaceMode::Code;
    app.show_settings = false;
    app.terminal.is_visible = false;
    app.show_diff = false;
    app.show_file_manager = false;

    let element = controls::project_view_tools(&app);

    assert_project_toolbar_shape(&element);
}

#[test]
fn project_view_tools_builds_project_controls_when_panels_are_visible() {
    let mut app = test_app();
    app.screen = Screen::Project;
    app.workspace_mode = WorkspaceMode::Code;
    app.show_settings = true;
    app.terminal.is_visible = true;
    app.show_diff = true;
    app.show_file_manager = true;
    #[cfg(not(target_arch = "wasm32"))]
    {
        app.task_pet_window_id = Some(iced::window::Id::unique());
    }

    let element = controls::project_view_tools(&app);

    assert_project_toolbar_shape(&element);
}

#[test]
fn work_project_tools_include_terminal_without_review_controls() {
    let mut app = test_app();
    app.screen = Screen::Project;
    app.workspace_mode = WorkspaceMode::Work;
    app.terminal.is_visible = true;
    app.show_diff = true;

    let element = controls::project_view_tools(&app);

    assert_work_project_toolbar_shape(&element);
}

#[tokio::test(flavor = "multi_thread")]
async fn terminal_modes_toolbar_click_emits_toggle_in_both_modes() {
    use iced::advanced::renderer::Headless;
    use iced::advanced::{Layout, Shell, clipboard, layout, mouse, widget::Tree};
    use iced::{Event, Font, Pixels, Point, Rectangle};
    let renderer =
        <iced::Renderer as Headless>::new(Font::DEFAULT, Pixels(16.0), Some("tiny-skia"))
            .await
            .expect("headless renderer");
    let mut app = test_app();
    app.screen = Screen::Project;
    for mode in [WorkspaceMode::Code, WorkspaceMode::Work] {
        app.workspace_mode = mode;
        let mut element = controls::project_view_tools(&app);
        let mut tree = Tree::new(element.as_widget());
        let node = element.as_widget_mut().layout(
            &mut tree,
            &renderer,
            &layout::Limits::new(Size::ZERO, Size::new(600.0, 40.0)),
        );
        let mut toggles = 0;
        for child in Layout::new(&node).children() {
            let point = child.bounds().center();
            let mut messages = Vec::new();
            for event in [
                mouse::Event::ButtonPressed(mouse::Button::Left),
                mouse::Event::ButtonReleased(mouse::Button::Left),
            ] {
                let mut shell = Shell::new(&mut messages);
                element.as_widget_mut().update(
                    &mut tree,
                    &Event::Mouse(event),
                    Layout::new(&node),
                    mouse::Cursor::Available(Point::new(point.x, point.y)),
                    &renderer,
                    &mut clipboard::Null,
                    &mut shell,
                    &Rectangle::with_size(Size::new(600.0, 40.0)),
                );
            }
            toggles += messages
                .iter()
                .filter(|message| {
                    matches!(
                        message,
                        Message::View(crate::app::message::ViewMessage::ToggleTerminalPanel)
                    )
                })
                .count();
        }
        assert_eq!(
            toggles,
            usize::from(!cfg!(target_arch = "wasm32")),
            "{mode:?} terminal visibility must follow platform support"
        );
    }
}
