//! 工作模式整体布局的绘制测试，覆盖侧栏、聊天与资源面板的组合状态。
//! 布局签名用于确认全屏、半屏和拖拽分割不会重复或遗漏表面。
use iced::advanced::renderer::Headless;
use iced::advanced::{Layout, Shell, clipboard, layout, mouse, renderer, widget::Tree};
use iced::{Event, Font, Pixels, Point, Rectangle, Size, Theme};

use crate::app::message::ViewMessage;
use crate::app::state::{WorkResourceMode, WorkspaceMode};
use crate::app::{App, Message};

use super::super::handles::HResizeHandle;

const VIEWPORT: Size = Size::new(1_200.0, 800.0);

#[tokio::test(flavor = "multi_thread")]
async fn work_layout_keeps_single_chat_surface_when_code_panels_are_enabled() {
    let renderer =
        <iced::Renderer as Headless>::new(Font::DEFAULT, Pixels(16.0), Some("tiny-skia"))
            .await
            .expect("tiny-skia headless renderer should be available");
    let mut app = App::new().0;
    app.show_settings = true;
    app.show_diff = false;
    app.show_file_manager = false;
    app.terminal.is_visible = false;
    app.chat_panel_fullscreen = false;
    app.git_diff_fullscreen = false;

    let without_code_panels = {
        let mut element = super::build_work_layout(&app, 476.0, 56.0, 0.6, 12.0, 0.0, 0.0);
        layout_signature(&mut element, &renderer)
    };

    app.show_diff = true;
    app.show_file_manager = true;
    app.chat_panel_fullscreen = true;
    app.git_diff_fullscreen = true;

    let with_code_panels = {
        let mut element = super::build_work_layout(&app, 476.0, 56.0, 0.6, 12.0, 0.0, 0.0);
        layout_signature(&mut element, &renderer)
    };

    assert_eq!(
        without_code_panels, with_code_panels,
        "code-only panel state must not alter the work layout tree"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn work_layout_renders_matching_backgrounds_in_dark_and_light_themes() {
    let mut renderer =
        <iced::Renderer as Headless>::new(Font::DEFAULT, Pixels(16.0), Some("tiny-skia"))
            .await
            .expect("tiny-skia headless renderer should be available");
    let mut app = App::new().0;
    app.workspace_mode = WorkspaceMode::Work;
    app.active_theme_skin = None;
    app.terminal.is_visible = false;
    app.work_sidebar_collapsed = false;
    app.work_sidebar_width = 280.0;
    app.work_resources_visible = true;
    app.work_resource_mode = WorkResourceMode::Overview;

    for theme in [Theme::Dark, Theme::Light] {
        app.app_theme = theme.clone();
        let mut element = super::super::view(&app);
        draw_element(&mut element, &mut renderer, &theme);

        let size = Size::new(VIEWPORT.width as u32, VIEWPORT.height as u32);
        let pixels = renderer.screenshot(size, 1.0, theme.palette().background);
        let pixel_at = |x: usize| {
            let offset = (size.height as usize / 2 * size.width as usize + x) * 4;
            &pixels[offset..offset + 4]
        };
        let sidebar_edge = app.work_sidebar_width as usize;
        let sidebar = pixel_at(sidebar_edge - 3);
        let divider = pixel_at(sidebar_edge + 2);
        let chat = pixel_at(sidebar_edge + HResizeHandle::HIT_WIDTH as usize + 12);
        let resources = pixel_at(size.width as usize - 28);

        assert_eq!(sidebar, divider, "{theme:?} divider background");
        assert_eq!(sidebar, chat, "{theme:?} chat background");
        assert_eq!(sidebar, resources, "{theme:?} resources background");
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn work_layout_uses_fixed_work_sidebar_when_code_sidebar_state_changes() {
    let renderer =
        <iced::Renderer as Headless>::new(Font::DEFAULT, Pixels(16.0), Some("tiny-skia"))
            .await
            .expect("tiny-skia headless renderer should be available");
    let mut app = App::new().0;
    app.show_settings = false;

    let collapsed = {
        let mut element = super::build_work_layout(&app, 476.0, 56.0, 0.6, 12.0, 0.0, 0.0);
        layout_signature(&mut element, &renderer)
    };

    app.show_settings = true;
    let expanded = {
        let mut element = super::build_work_layout(&app, 476.0, 56.0, 0.6, 12.0, 0.0, 0.0);
        layout_signature(&mut element, &renderer)
    };

    assert_eq!(collapsed, expanded, "work layout must not reuse the collapsible code sidebar");
}

#[tokio::test(flavor = "multi_thread")]
async fn work_resource_panel_split_ratio_and_fullscreen_have_independent_layouts() {
    let renderer =
        <iced::Renderer as Headless>::new(Font::DEFAULT, Pixels(16.0), Some("tiny-skia"))
            .await
            .expect("tiny-skia headless renderer should be available");
    let mut app = App::new().0;
    app.work_resources_visible = true;
    app.work_split_ratio = 0.25;
    let narrow_chat = {
        let mut element = super::build_work_layout(&app, 476.0, 56.0, 0.6, 12.0, 0.0, 0.0);
        layout_signature(&mut element, &renderer)
    };

    app.work_split_ratio = 0.75;
    let wide_chat = {
        let mut element = super::build_work_layout(&app, 476.0, 56.0, 0.6, 12.0, 0.0, 0.0);
        layout_signature(&mut element, &renderer)
    };
    assert_ne!(narrow_chat, wide_chat, "work split ratio must resize the two work surfaces");

    app.work_resource_fullscreen = true;
    let resource_fullscreen = {
        let mut element = super::build_work_layout(&app, 476.0, 56.0, 0.6, 12.0, 0.0, 0.0);
        layout_signature(&mut element, &renderer)
    };
    assert_ne!(wide_chat, resource_fullscreen, "resource fullscreen must replace the work layout");
}

#[tokio::test(flavor = "multi_thread")]
async fn work_resource_fullscreen_hides_sidebar_and_restores_right_half() {
    let renderer =
        <iced::Renderer as Headless>::new(Font::DEFAULT, Pixels(16.0), Some("tiny-skia"))
            .await
            .expect("tiny-skia headless renderer should be available");
    let mut app = App::new().0;
    app.work_resources_visible = true;
    app.work_resource_fullscreen = true;

    let fullscreen = {
        let mut element = super::build_work_layout(&app, 476.0, 56.0, 0.6, 12.0, 0.0, 0.0);
        layout_signature(&mut element, &renderer)
    };

    app.work_sidebar_collapsed = true;
    let fullscreen_with_collapsed_sidebar = {
        let mut element = super::build_work_layout(&app, 476.0, 56.0, 0.6, 12.0, 0.0, 0.0);
        layout_signature(&mut element, &renderer)
    };
    assert_eq!(
        fullscreen, fullscreen_with_collapsed_sidebar,
        "resource fullscreen must occupy the work area without the sidebar"
    );

    app.work_resource_fullscreen = false;
    let restored = {
        let mut element = super::build_work_layout(&app, 476.0, 56.0, 0.6, 12.0, 0.0, 0.0);
        layout_signature(&mut element, &renderer)
    };
    assert_ne!(
        fullscreen, restored,
        "exiting resource fullscreen must restore the right-side split"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn work_half_fullscreen_keeps_sidebar_and_selects_one_surface() {
    let renderer =
        <iced::Renderer as Headless>::new(Font::DEFAULT, Pixels(16.0), Some("tiny-skia"))
            .await
            .expect("tiny-skia headless renderer should be available");
    let mut app = App::new().0;
    app.work_resources_visible = true;

    app.work_resource_half_fullscreen = true;
    let resource_half = {
        let mut element = super::build_work_layout(&app, 476.0, 56.0, 0.6, 12.0, 0.0, 0.0);
        layout_signature(&mut element, &renderer)
    };

    app.work_sidebar_collapsed = true;
    let resource_half_collapsed_sidebar = {
        let mut element = super::build_work_layout(&app, 476.0, 56.0, 0.6, 12.0, 0.0, 0.0);
        layout_signature(&mut element, &renderer)
    };
    assert_ne!(
        resource_half, resource_half_collapsed_sidebar,
        "resource half fullscreen must keep the work sidebar"
    );

    app.work_sidebar_collapsed = false;
    app.work_resource_half_fullscreen = false;
    app.work_chat_half_fullscreen = true;
    let chat_half = {
        let mut element = super::build_work_layout(&app, 476.0, 56.0, 0.6, 12.0, 0.0, 0.0);
        layout_signature(&mut element, &renderer)
    };
    assert_ne!(
        resource_half, chat_half,
        "chat and resource half fullscreen must select different surfaces"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn work_layout_chat_fullscreen_hides_sidebar_and_resources() {
    let renderer =
        <iced::Renderer as Headless>::new(Font::DEFAULT, Pixels(16.0), Some("tiny-skia"))
            .await
            .expect("tiny-skia headless renderer should be available");
    let mut app = App::new().0;
    app.work_resources_visible = true;

    let regular = {
        let mut element = super::build_work_layout(&app, 476.0, 56.0, 0.6, 12.0, 0.0, 0.0);
        layout_signature(&mut element, &renderer)
    };

    app.work_chat_fullscreen = true;
    let fullscreen = {
        let mut element = super::build_work_layout(&app, 476.0, 56.0, 0.6, 12.0, 0.0, 0.0);
        layout_signature(&mut element, &renderer)
    };

    assert_ne!(regular, fullscreen, "fullscreen must show only the chat surface");
}

#[tokio::test(flavor = "multi_thread")]
async fn work_layout_hidden_resources_leave_chat_as_the_only_right_surface() {
    let renderer =
        <iced::Renderer as Headless>::new(Font::DEFAULT, Pixels(16.0), Some("tiny-skia"))
            .await
            .expect("tiny-skia headless renderer should be available");
    let mut app = App::new().0;

    assert!(!app.work_resources_visible);
    let hidden = {
        let mut element = super::build_work_layout(&app, 476.0, 56.0, 0.6, 12.0, 0.0, 0.0);
        layout_signature(&mut element, &renderer)
    };

    app.work_chat_half_fullscreen = true;
    let chat_only = {
        let mut element = super::build_work_layout(&app, 476.0, 56.0, 0.6, 12.0, 0.0, 0.0);
        layout_signature(&mut element, &renderer)
    };

    assert_eq!(hidden, chat_only, "hiding work resources must remove the whole right module");
}

#[tokio::test(flavor = "multi_thread")]
async fn work_layout_drag_handles_emit_sidebar_and_chat_split_messages() {
    let renderer =
        <iced::Renderer as Headless>::new(Font::DEFAULT, Pixels(16.0), Some("tiny-skia"))
            .await
            .expect("tiny-skia headless renderer should be available");
    let mut app = App::new().0;
    app.work_resources_visible = true;

    let mut sidebar_element = super::build_work_layout(&app, 476.0, 56.0, 0.6, 12.0, 0.0, 0.0);
    let sidebar_messages = press_at(
        &mut sidebar_element,
        &renderer,
        Point::new(app.work_sidebar_width + HResizeHandle::HIT_WIDTH / 2.0, 400.0),
    );
    assert!(
        sidebar_messages.iter().any(|message| {
            matches!(message, Message::View(ViewMessage::WorkSidebarDragStarted))
        })
    );

    let right_width = VIEWPORT.width - app.work_sidebar_width - HResizeHandle::HIT_WIDTH;
    let split_x = app.work_sidebar_width
        + HResizeHandle::HIT_WIDTH
        + (right_width - HResizeHandle::HIT_WIDTH) * app.work_split_ratio
        + HResizeHandle::HIT_WIDTH / 2.0;
    let mut split_element = super::build_work_layout(&app, 476.0, 56.0, 0.6, 12.0, 0.0, 0.0);
    let split_messages = press_at(&mut split_element, &renderer, Point::new(split_x, 400.0));
    assert!(
        split_messages
            .iter()
            .any(|message| matches!(message, Message::View(ViewMessage::WorkSplitDragStarted)))
    );
}

fn layout_signature(
    element: &mut iced::Element<'_, crate::app::Message>,
    renderer: &iced::Renderer,
) -> Vec<(usize, i32, i32, i32, i32)> {
    let limits = layout::Limits::new(Size::ZERO, VIEWPORT);
    let mut tree = Tree::new(element.as_widget());
    let node = element.as_widget_mut().layout(&mut tree, renderer, &limits);
    let mut signature = Vec::new();
    collect_layout(Layout::new(&node), 0, &mut signature);
    signature
}

fn collect_layout(
    layout: Layout<'_>,
    depth: usize,
    signature: &mut Vec<(usize, i32, i32, i32, i32)>,
) {
    let bounds = layout.bounds();
    signature.push((
        depth,
        bounds.x.round() as i32,
        bounds.y.round() as i32,
        bounds.width.round() as i32,
        bounds.height.round() as i32,
    ));
    for child in layout.children() {
        collect_layout(child, depth + 1, signature);
    }
}

fn draw_element(
    element: &mut iced::Element<'_, crate::app::Message>,
    renderer: &mut iced::Renderer,
    theme: &Theme,
) {
    let limits = layout::Limits::new(Size::ZERO, VIEWPORT);
    let mut tree = Tree::new(element.as_widget());
    let node = element.as_widget_mut().layout(&mut tree, renderer, &limits);
    let viewport = Rectangle::with_size(VIEWPORT);

    element.as_widget().draw(
        &tree,
        renderer,
        theme,
        &renderer::Style::default(),
        Layout::new(&node),
        mouse::Cursor::Unavailable,
        &viewport,
    );
}

fn press_at(
    element: &mut iced::Element<'_, Message>,
    renderer: &iced::Renderer,
    point: Point,
) -> Vec<Message> {
    let limits = layout::Limits::new(Size::ZERO, VIEWPORT);
    let mut tree = Tree::new(element.as_widget());
    let node = element.as_widget_mut().layout(&mut tree, renderer, &limits);
    let mut clipboard = clipboard::Null;
    let mut messages = Vec::new();
    let mut shell = Shell::new(&mut messages);
    element.as_widget_mut().update(
        &mut tree,
        &Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)),
        Layout::new(&node),
        mouse::Cursor::Available(point),
        renderer,
        &mut clipboard,
        &mut shell,
        &Rectangle::with_size(VIEWPORT),
    );
    messages
}

#[tokio::test(flavor = "multi_thread")]
async fn terminal_modes_layout_allocates_one_bottom_panel_in_both_themes_and_modes() {
    let mut renderer =
        <iced::Renderer as Headless>::new(Font::DEFAULT, Pixels(16.0), Some("tiny-skia"))
            .await
            .expect("headless renderer");
    let mut app = App::new().0;
    app.terminal.is_visible = true;
    app.terminal.height = 240.0;
    for theme in [Theme::Dark, Theme::Light] {
        for work in [false, true] {
            let mut element = if work {
                super::build_work_layout(&app, 476.0, 56.0, 0.6, 12.0, 0.0, 0.0)
            } else {
                super::build_base_layout(&app, 476.0, 56.0, 0.6, 12.0, 0.0, 0.0, 0.0)
            };
            let signature = layout_signature(&mut element, &renderer);
            assert_eq!(
                signature
                    .iter()
                    .filter(|(depth, _, y, _, h)| *depth == 1 && *y == 560 && *h == 240)
                    .count(),
                1,
                "exactly one terminal must occupy the bottom 240px, work={work}"
            );
            assert!(
                signature
                    .iter()
                    .all(|(_, x, y, w, h)| *x >= 0 && *y >= 0 && x + w <= 1200 && y + h <= 800)
            );
            draw_element(&mut element, &mut renderer, &theme);
        }
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn terminal_modes_work_fullscreen_temporarily_hides_terminal_and_restores_layout() {
    let renderer =
        <iced::Renderer as Headless>::new(Font::DEFAULT, Pixels(16.0), Some("tiny-skia"))
            .await
            .expect("headless renderer");
    let mut app = App::new().0;
    app.terminal.is_visible = false;
    let hidden = layout_signature(
        &mut super::build_work_layout(&app, 476.0, 56.0, 0.6, 12.0, 0.0, 0.0),
        &renderer,
    );
    app.terminal.is_visible = true;
    let shown = layout_signature(
        &mut super::build_work_layout(&app, 476.0, 56.0, 0.6, 12.0, 0.0, 0.0),
        &renderer,
    );
    assert_ne!(hidden, shown);
    for chat in [false, true] {
        app.work_chat_fullscreen = chat;
        app.work_resource_fullscreen = !chat;
        let visible = layout_signature(
            &mut super::build_work_layout(&app, 476.0, 56.0, 0.6, 12.0, 0.0, 0.0),
            &renderer,
        );
        app.terminal.is_visible = false;
        let hidden = layout_signature(
            &mut super::build_work_layout(&app, 476.0, 56.0, 0.6, 12.0, 0.0, 0.0),
            &renderer,
        );
        assert_eq!(visible, hidden);
        app.terminal.is_visible = true;
    }
    app.work_chat_fullscreen = false;
    app.work_resource_fullscreen = false;
    assert_eq!(
        shown,
        layout_signature(
            &mut super::build_work_layout(&app, 476.0, 56.0, 0.6, 12.0, 0.0, 0.0),
            &renderer
        )
    );
}

#[cfg(not(target_arch = "wasm32"))]
#[tokio::test(flavor = "multi_thread")]
async fn terminal_fullscreen_toolbar_round_trips_workspace_layout() {
    let mut renderer =
        <iced::Renderer as Headless>::new(Font::DEFAULT, Pixels(16.0), Some("tiny-skia"))
            .await
            .expect("headless renderer");
    let mut app = App::new().0;
    app.terminal = crate::app::TerminalState::blank_with_settings(
        true,
        crate::app::Shell::Bash,
        crate::app::TerminalTheme::System,
        "monospace".into(),
        15.0,
        240.0,
    );
    app.terminal
        .add_terminal_with_command(
            "中文终端标签".repeat(20),
            Some(std::env::temp_dir()),
            "exec cat".into(),
        )
        .expect("test PTY");

    let size = Size::new(240.0, 500.0);
    for theme in [Theme::Light, Theme::Dark] {
        for mode in [WorkspaceMode::Code, WorkspaceMode::Work] {
            app.app_theme = theme.clone();
            app.workspace_mode = mode;
            app.show_diff = true;
            app.work_resources_visible = true;

            for _ in 0..2 {
                let enter = terminal_toolbar_frame(&app, &mut renderer, size, 268, true)
                    .expect("docked toolbar should emit a terminal fullscreen message");
                let _ = app.update(enter);
                let restore = terminal_toolbar_frame(&app, &mut renderer, size, 0, true)
                    .expect("fullscreen toolbar should emit a terminal restore message");
                let _ = app.update(restore);
                terminal_toolbar_frame(&app, &mut renderer, size, 268, false);
                assert_eq!(app.terminal.height, 240.0);
                assert!(app.show_diff && app.work_resources_visible);
            }
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn terminal_toolbar_frame(
    app: &App,
    renderer: &mut iced::Renderer,
    size: Size,
    header_y: i32,
    click: bool,
) -> Option<Message> {
    use iced::mouse::{Button, Cursor, Event as MouseEvent, ScrollDelta};
    use iced_runtime::user_interface::{Cache, UserInterface};

    let mut view = super::super::view(app);
    let mut tree = Tree::new(view.as_widget());
    let node =
        view.as_widget_mut().layout(&mut tree, renderer, &layout::Limits::new(Size::ZERO, size));
    let mut signature = Vec::new();
    collect_layout(Layout::new(&node), 0, &mut signature);
    let expected_content = (size.width as i32, size.height as i32 - header_y - 34);
    assert!(
        signature
            .iter()
            .any(|&(_, x, y, w, h)| { x == 0 && y == header_y + 34 && (w, h) == expected_content }),
        "expected full-width terminal content {expected_content:?} below header at y={header_y}"
    );
    eprintln!(
        "{:?} / {:?}: terminal header y={header_y}, content={expected_content:?}",
        app.workspace_mode, app.app_theme
    );

    let mut ui = UserInterface::build(view, size, Cache::new(), renderer);
    let style = renderer::Style { text_color: app.app_theme.palette().text };
    ui.draw(renderer, &app.app_theme, &style, Cursor::Unavailable);
    let pixel_size = Size::new(size.width as u32, size.height as u32);
    let pixels = renderer.screenshot(pixel_size, 1.0, app.app_theme.palette().background);
    assert_eq!(pixels.len(), (pixel_size.width * pixel_size.height * 4) as usize);

    if !click {
        return None;
    }
    let mut clipboard = clipboard::Null;
    let mut messages = Vec::new();
    // Scroll the overflowing tab strip before clicking the fixed right-hand control.
    let (_, scroll_status) = ui.update(
        &[Event::Mouse(MouseEvent::WheelScrolled {
            delta: ScrollDelta::Pixels { x: -200.0, y: 0.0 },
        })],
        Cursor::Available(Point::new(80.0, header_y as f32 + 12.0)),
        renderer,
        &mut clipboard,
        &mut messages,
    );
    assert_eq!(scroll_status, [iced::event::Status::Captured]);
    let cursor = Cursor::Available(Point::new(size.width - 16.0, header_y as f32 + 17.0));
    ui.draw(renderer, &app.app_theme, &style, cursor);
    let _ = ui.update(
        &[
            Event::Mouse(MouseEvent::ButtonPressed(Button::Left)),
            Event::Mouse(MouseEvent::ButtonReleased(Button::Left)),
        ],
        cursor,
        renderer,
        &mut clipboard,
        &mut messages,
    );
    messages.into_iter().find(|message| {
        matches!(message, Message::Terminal(crate::app::message::TerminalMessage::ToggleFullscreen))
    })
}
