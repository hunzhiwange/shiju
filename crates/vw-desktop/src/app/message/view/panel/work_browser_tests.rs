use iced::advanced::renderer::Headless;
use iced::advanced::{Layout, Shell, clipboard, layout, mouse, renderer, widget::Tree};
use iced::{Event, Font, Pixels, Rectangle, Size, Theme};
use std::io;
use std::sync::{Arc, Mutex};

use super::{external_browser_message, view};
use crate::app::state::{WorkResourceMode, WorkspaceMode};
use crate::app::webview_runtime::{BrowserProcess, WebviewBounds, WorkBrowserEvent};
use crate::app::{App, Message, Screen, message::ViewMessage};

const VIEWPORT: Size = Size::new(760.0, 160.0);

#[test]
fn work_browser_reserves_status_bar_at_bottom() {
    assert_eq!(super::BROWSER_BOTTOM_INSET, crate::app::impls::STATUS_BAR_HEIGHT + 10.0);
}

#[test]
fn work_browser_content_starts_below_resource_header_and_toolbar() {
    assert_eq!(super::BROWSER_CONTENT_TOP_INSET, 134.0);
    assert_eq!(
        super::BROWSER_CONTENT_TOP_INSET,
        super::RESOURCE_PANEL_HEADER_HEIGHT
            + super::BROWSER_TOOLBAR_MAX_HEIGHT
            + super::BROWSER_TOOLBAR_CONTENT_GAP
    );
}

struct NavigationProcess {
    events: Arc<Mutex<Vec<WorkBrowserEvent>>>,
}

impl BrowserProcess for NavigationProcess {
    fn write_protocol(&mut self, _bytes: &[u8]) -> io::Result<()> {
        Ok(())
    }

    fn take_events(&mut self) -> Vec<WorkBrowserEvent> {
        std::mem::take(&mut self.events.lock().expect("navigation events lock"))
    }

    fn is_running(&mut self) -> io::Result<bool> {
        Ok(true)
    }

    fn terminate(&mut self) -> io::Result<()> {
        Ok(())
    }

    fn terminate_now(&mut self) -> io::Result<()> {
        Ok(())
    }
}

fn visible_work_browser_app() -> App {
    let mut app = App::new().0;
    app.screen = Screen::Project;
    app.workspace_mode = WorkspaceMode::Work;
    app.work_resource_mode = WorkResourceMode::Browser;
    app.window_size = (1_400.0, 900.0);
    app
}

#[test]
fn work_browser_defaults_to_blank_surface() {
    let mut app = App::new().0;

    assert!(app.work_browser.address.is_empty());
    assert!(app.work_browser.closed);
    assert!(!app.work_browser_controller.has_running_child());
}

#[test]
fn work_browser_close_terminates_current_page_and_keeps_blank_surface() {
    let mut app = visible_work_browser_app();
    app.work_browser.error = Some("旧错误".to_string());
    app.work_browser_controller
        .ensure_started_with(
            "https://example.com/",
            WebviewBounds { x: 0, y: 0, width: 800, height: 600 },
            |_| Ok(Box::new(NavigationProcess { events: Arc::new(Mutex::new(Vec::new())) })),
        )
        .expect("controlled browser should start");

    let _ = super::update(&mut app, ViewMessage::WorkBrowserClose);

    assert!(app.work_browser.address.is_empty());
    assert!(app.work_browser.error.is_none());
    assert!(app.work_browser.closed);
    assert!(!app.work_browser_controller.has_running_child());

    let _ = super::sync_lifecycle(&mut app);
    assert!(app.work_browser.error.is_none());
    assert!(!app.work_browser_controller.has_running_child());
}

#[test]
fn work_browser_navigate_reopens_closed_blank_surface() {
    let mut app = visible_work_browser_app();
    app.work_browser.closed = true;
    app.work_browser.address = "https://example.com/next".to_string();
    app.work_browser_controller
        .ensure_started_with(
            "https://example.com/",
            WebviewBounds { x: 0, y: 0, width: 800, height: 600 },
            |_| Ok(Box::new(NavigationProcess { events: Arc::new(Mutex::new(Vec::new())) })),
        )
        .expect("controlled browser should start");

    let _ = super::update(&mut app, ViewMessage::WorkBrowserNavigate);

    assert!(!app.work_browser.closed);
    assert!(app.work_browser.error.is_none());
}

#[test]
fn work_browser_runtime_tick_updates_address_from_webview_navigation() {
    let mut app = App::new().0;
    let events = Arc::new(Mutex::new(Vec::new()));
    app.work_browser_controller
        .ensure_started_with(
            "https://example.com/",
            WebviewBounds { x: 0, y: 0, width: 800, height: 600 },
            |_| Ok(Box::new(NavigationProcess { events: Arc::clone(&events) })),
        )
        .expect("controlled browser should start");
    events.lock().expect("navigation events lock").push(WorkBrowserEvent::NavigationChanged(
        "https://example.com/docs/next?tab=api#response".to_string(),
    ));

    let _ = std::hint::black_box(super::update(&mut app, ViewMessage::WorkBrowserRuntimeTick));

    assert_eq!(app.work_browser.address, "https://example.com/docs/next?tab=api#response");
}

#[tokio::test(flavor = "multi_thread")]
async fn work_browser_controller_toolbar_uses_compact_height() {
    let renderer =
        <iced::Renderer as Headless>::new(Font::DEFAULT, Pixels(16.0), Some("tiny-skia"))
            .await
            .expect("tiny-skia headless renderer should be available");
    let app = App::new().0;
    let mut element = view(&app);
    let limits = layout::Limits::new(Size::ZERO, VIEWPORT);
    let mut tree = Tree::new(element.as_widget());
    let node = element.as_widget_mut().layout(&mut tree, &renderer, &limits);
    let toolbar = Layout::new(&node).children().next().expect("browser toolbar");

    assert!(
        toolbar.bounds().height <= 38.0,
        "browser toolbar should remain compact, got {}px",
        toolbar.bounds().height
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn work_browser_toolbar_close_action_is_reachable() {
    let renderer =
        <iced::Renderer as Headless>::new(Font::DEFAULT, Pixels(16.0), Some("tiny-skia"))
            .await
            .expect("tiny-skia headless renderer should be available");
    let app = App::new().0;
    let mut element = view(&app);
    let limits = layout::Limits::new(Size::ZERO, VIEWPORT);
    let mut tree = Tree::new(element.as_widget());
    let node = element.as_widget_mut().layout(&mut tree, &renderer, &limits);
    let toolbar = Layout::new(&node).children().next().expect("browser toolbar");
    let controls = toolbar.children().next().expect("browser toolbar controls");
    let close_bounds = controls.children().last().expect("browser close control").bounds();
    let cursor = mouse::Cursor::Available(close_bounds.center());
    let mut clipboard = clipboard::Null;
    let mut messages = Vec::new();

    for event in [
        Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)),
        Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)),
    ] {
        let mut shell = Shell::new(&mut messages);
        element.as_widget_mut().update(
            &mut tree,
            &event,
            Layout::new(&node),
            cursor,
            &renderer,
            &mut clipboard,
            &mut shell,
            &Rectangle::with_size(VIEWPORT),
        );
    }

    assert!(
        messages
            .into_iter()
            .any(|message| matches!(message, Message::View(ViewMessage::WorkBrowserClose)))
    );
}

#[test]
fn work_browser_controller_toolbar_external_action_normalizes_safe_url() {
    assert!(matches!(
        external_browser_message(" example.com "),
        Some(Message::View(ViewMessage::OpenUrlExternal(url))) if url == "https://example.com"
    ));
    assert!(external_browser_message("file:///tmp/private").is_none());
}

#[tokio::test(flavor = "multi_thread")]
async fn work_browser_controller_toolbar_draws_dark_chinese_error_without_panic() {
    let mut renderer =
        <iced::Renderer as Headless>::new(Font::DEFAULT, Pixels(16.0), Some("tiny-skia"))
            .await
            .expect("tiny-skia headless renderer should be available");
    let mut app = App::new().0;
    app.work_browser.address = "https://example.com/文档".to_string();
    app.work_browser.error = Some("无法启动内置浏览器，请检查辅助程序。".to_string());

    let mut element = view(&app);
    let limits = layout::Limits::new(Size::ZERO, VIEWPORT);
    let mut tree = Tree::new(element.as_widget());
    let node = element.as_widget_mut().layout(&mut tree, &renderer, &limits);
    let viewport = Rectangle::with_size(VIEWPORT);
    element.as_widget().draw(
        &tree,
        &mut renderer,
        &Theme::Dark,
        &renderer::Style::default(),
        Layout::new(&node),
        mouse::Cursor::Unavailable,
        &viewport,
    );
}
