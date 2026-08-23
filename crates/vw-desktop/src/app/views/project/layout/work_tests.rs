//! 工作模式整体布局的绘制测试，覆盖侧栏、聊天与资源面板的组合状态。
//! 布局签名用于确认全屏、半屏和拖拽分割不会重复或遗漏表面。
use iced::advanced::renderer::Headless;
use iced::advanced::{Layout, Shell, clipboard, layout, mouse, renderer, widget::Tree};
use iced::{Event, Font, Pixels, Point, Rectangle, Size, Theme};

use crate::app::message::ViewMessage;
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
    app.terminal.is_visible = true;
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
async fn work_layout_draws_in_dark_and_light_themes() {
    let mut renderer =
        <iced::Renderer as Headless>::new(Font::DEFAULT, Pixels(16.0), Some("tiny-skia"))
            .await
            .expect("tiny-skia headless renderer should be available");
    let mut app = App::new().0;
    app.show_settings = true;

    for theme in [Theme::Dark, Theme::Light] {
        let mut element = super::build_work_layout(&app, 476.0, 56.0, 0.6, 12.0, 0.0, 0.0);
        draw_element(&mut element, &mut renderer, &theme);
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
