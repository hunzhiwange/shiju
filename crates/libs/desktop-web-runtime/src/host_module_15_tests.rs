use crate::{
    webview_runtime::{BrowserProcess, WebviewBounds, WorkBrowserController, WorkBrowserEvent},
    work_browser as domain,
};
use domain::{Message as ViewMessage, external_browser_message};
use iced::advanced::renderer::Headless;
use iced::advanced::{Layout, Shell, clipboard, layout, mouse, renderer, widget::Tree};
use iced::{Event, Font, Pixels, Rectangle, Size, Theme};
use std::{
    io,
    sync::{Arc, Mutex},
};
const VIEWPORT: Size = Size::new(760.0, 160.0);
#[derive(Default)]
struct BrowserFixture {
    work_browser: crate::state::WorkBrowserState,
    work_browser_controller: WorkBrowserController,
}
fn visible_work_browser_app() -> BrowserFixture {
    BrowserFixture::default()
}
fn placement() -> domain::Placement {
    domain::Placement {
        project_screen: true,
        work_mode: true,
        browser_resource: true,
        resources_visible: false,
        system_settings_open: false,
        chat_fullscreen: false,
        chat_half_fullscreen: false,
        window_position: (0.0, 0.0),
        window_size: (1400.0, 900.0),
        sidebar_collapsed: false,
        sidebar_width: 292.0,
        sidebar_rail_width: 54.0,
        terminal_visible: false,
        terminal_fullscreen: false,
        terminal_height: 240.0,
        split_ratio: 0.6,
        resource_fullscreen: false,
        resource_half_fullscreen: false,
        top_bar_height: 28.0,
        status_bar_height: 30.0,
    }
}
fn view(app: &BrowserFixture) -> iced::Element<'_, domain::Message> {
    domain::view(&app.work_browser, Default::default())
}
fn update(app: &mut BrowserFixture, message: domain::Message) -> Option<String> {
    domain::update(&mut app.work_browser, &mut app.work_browser_controller, &placement(), message)
}
fn sync_lifecycle(app: &mut BrowserFixture) -> Option<String> {
    domain::sync_lifecycle(&mut app.work_browser, &mut app.work_browser_controller, &placement())
}
#[derive(Default)]
struct NavigationProcess {
    events: Arc<Mutex<Vec<WorkBrowserEvent>>>,
    commands: Arc<Mutex<Vec<String>>>,
}

impl BrowserProcess for NavigationProcess {
    fn write_protocol(&mut self, bytes: &[u8]) -> io::Result<()> {
        self.commands.lock().unwrap().push(String::from_utf8_lossy(bytes).into_owned());
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

#[test]
fn work_browser_content_starts_below_resource_header_and_toolbar() {
    assert_eq!(domain::BROWSER_CONTENT_TOP_INSET, 134.0);
    assert_eq!(
        domain::BROWSER_CONTENT_TOP_INSET,
        domain::RESOURCE_PANEL_HEADER_HEIGHT
            + domain::BROWSER_TOOLBAR_MAX_HEIGHT
            + domain::BROWSER_TOOLBAR_CONTENT_GAP
    );
}

#[test]
fn work_browser_defaults_to_blank_surface() {
    let mut app = BrowserFixture::default();

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
            |_| Ok(Box::new(NavigationProcess::default())),
        )
        .expect("controlled browser should start");

    let _ = update(&mut app, ViewMessage::WorkBrowserClose);

    assert!(app.work_browser.address.is_empty());
    assert!(app.work_browser.error.is_none());
    assert!(app.work_browser.closed);
    assert!(!app.work_browser_controller.has_running_child());

    let _ = sync_lifecycle(&mut app);
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
            |_| Ok(Box::new(NavigationProcess::default())),
        )
        .expect("controlled browser should start");

    let _ = update(&mut app, ViewMessage::WorkBrowserNavigate);

    assert!(!app.work_browser.closed);
    assert!(app.work_browser.error.is_none());
}

#[test]
fn work_browser_runtime_tick_updates_address_from_webview_navigation() {
    let mut app = BrowserFixture::default();
    let events = Arc::new(Mutex::new(Vec::new()));
    app.work_browser_controller
        .ensure_started_with(
            "https://example.com/",
            WebviewBounds { x: 0, y: 0, width: 800, height: 600 },
            |_| {
                Ok(Box::new(NavigationProcess {
                    events: Arc::clone(&events),
                    ..Default::default()
                }))
            },
        )
        .expect("controlled browser should start");
    events.lock().expect("navigation events lock").push(WorkBrowserEvent::NavigationChanged(
        "https://example.com/docs/next?tab=api#response".to_string(),
    ));

    let _ = std::hint::black_box(update(&mut app, ViewMessage::WorkBrowserRuntimeTick));

    assert_eq!(app.work_browser.address, "https://example.com/docs/next?tab=api#response");
}

#[tokio::test(flavor = "multi_thread")]
async fn work_browser_controller_toolbar_uses_compact_height() {
    let renderer =
        <iced::Renderer as Headless>::new(Font::DEFAULT, Pixels(16.0), Some("tiny-skia"))
            .await
            .expect("tiny-skia headless renderer should be available");
    let app = BrowserFixture::default();
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
    let mut renderer =
        <iced::Renderer as Headless>::new(Font::DEFAULT, Pixels(16.0), Some("tiny-skia"))
            .await
            .expect("tiny-skia headless renderer should be available");

    for theme in [Theme::Light, Theme::Dark] {
        for closed in [false, true] {
            let mut app = BrowserFixture::default();
            app.work_browser.closed = closed;
            if !closed {
                app.work_browser.address = "https://example.com/文档".to_string();
            }
            let mut element = view(&app);
            let limits = layout::Limits::new(Size::ZERO, VIEWPORT);
            let mut tree = Tree::new(element.as_widget());
            let node = element.as_widget_mut().layout(&mut tree, &renderer, &limits);
            let toolbar = Layout::new(&node).children().next().expect("browser toolbar");
            let controls = toolbar.children().next().expect("browser toolbar controls");
            let close_bounds = controls.children().last().expect("browser close control").bounds();
            let cursor = mouse::Cursor::Available(close_bounds.center());
            let viewport = Rectangle::with_size(VIEWPORT);
            element.as_widget().draw(
                &tree,
                &mut renderer,
                &theme,
                &renderer::Style::default(),
                Layout::new(&node),
                cursor,
                &viewport,
            );
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
                    &viewport,
                );
            }

            assert_eq!(
                messages
                    .into_iter()
                    .any(|message| matches!(message, ViewMessage::WorkBrowserClose)),
                !closed,
                "close action availability should match browser state (closed={closed}, theme={theme:?})"
            );
        }
    }
}

#[test]
fn work_browser_controller_toolbar_external_action_normalizes_safe_url() {
    assert!(matches!(
        external_browser_message(" example.com "),
        Some(ViewMessage::OpenUrlExternal(url)) if url == "https://example.com"
    ));
    assert!(external_browser_message("file:///tmp/private").is_none());
}

#[tokio::test(flavor = "multi_thread")]
async fn work_browser_controller_toolbar_draws_dark_chinese_error_without_panic() {
    let mut renderer =
        <iced::Renderer as Headless>::new(Font::DEFAULT, Pixels(16.0), Some("tiny-skia"))
            .await
            .expect("tiny-skia headless renderer should be available");
    let mut app = BrowserFixture::default();
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
