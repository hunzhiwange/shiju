use std::io;
use std::sync::{Arc, Mutex};

use super::{
    BrowserProcess, WebviewBounds, WorkBrowserController, WorkBrowserEvent, WorkBrowserLayout,
    WorkBrowserSurface, parse_webview_event, work_browser_bounds, work_browser_surface_visible,
};
use crate::app::Screen;
use crate::app::state::{WorkResourceMode, WorkspaceMode};

#[derive(Default)]
struct ProcessProbe {
    protocol: Vec<u8>,
    events: Vec<WorkBrowserEvent>,
    running: bool,
    terminations: usize,
}

struct MemoryProcess {
    probe: Arc<Mutex<ProcessProbe>>,
}

impl BrowserProcess for MemoryProcess {
    fn write_protocol(&mut self, bytes: &[u8]) -> io::Result<()> {
        self.probe.lock().expect("process probe lock").protocol.extend_from_slice(bytes);
        Ok(())
    }

    fn take_events(&mut self) -> Vec<WorkBrowserEvent> {
        std::mem::take(&mut self.probe.lock().expect("process probe lock").events)
    }

    fn is_running(&mut self) -> io::Result<bool> {
        Ok(self.probe.lock().expect("process probe lock").running)
    }

    fn terminate(&mut self) -> io::Result<()> {
        let mut probe = self.probe.lock().expect("process probe lock");
        probe.running = false;
        probe.terminations += 1;
        Ok(())
    }
}

#[test]
fn work_browser_controller_keeps_single_child_and_writes_navigation_commands() {
    let probe = Arc::new(Mutex::new(ProcessProbe { running: true, ..ProcessProbe::default() }));
    let mut spawn_count = 0;
    let mut controller = WorkBrowserController::default();
    let bounds = WebviewBounds { x: 740, y: 180, width: 620, height: 540 };

    for _ in 0..2 {
        controller
            .ensure_started_with("https://example.com/", bounds, |launch| {
                spawn_count += 1;
                assert_eq!(launch.target, "https://example.com/");
                assert_eq!(launch.bounds, bounds);
                Ok(Box::new(MemoryProcess { probe: Arc::clone(&probe) }))
            })
            .expect("controlled browser should start");
    }

    controller.navigate("https://example.com/docs").expect("navigation command");
    controller.back().expect("back command");
    controller.forward().expect("forward command");
    controller.reload().expect("reload command");
    controller
        .sync_bounds(WebviewBounds { x: 760, y: 190, width: 600, height: 520 })
        .expect("bounds commands");
    controller.set_always_on_top(false).expect("lower browser window");
    controller.set_always_on_top(true).expect("raise browser window");
    controller.set_visible(true).expect("show command");
    controller.set_visible(false).expect("hide command");
    controller.close().expect("close controlled browser");

    let probe = probe.lock().expect("process probe lock");
    assert_eq!(spawn_count, 1, "only one controlled child may be active");
    assert_eq!(
        String::from_utf8_lossy(&probe.protocol),
        concat!(
            "navigate https://example.com/docs\n",
            "back\n",
            "forward\n",
            "reload\n",
            "move 760 190\n",
            "resize 600 520\n",
            "lower\n",
            "raise\n",
            "show\n",
            "hide\n",
            "close\n",
        )
    );
    assert_eq!(probe.terminations, 1);
}

#[test]
fn work_browser_controller_uses_latest_safe_navigation_event() {
    let probe = Arc::new(Mutex::new(ProcessProbe { running: true, ..ProcessProbe::default() }));
    let mut controller = WorkBrowserController::default();
    let bounds = WebviewBounds { x: 0, y: 0, width: 800, height: 600 };
    controller
        .ensure_started_with("https://example.com/", bounds, |_| {
            Ok(Box::new(MemoryProcess { probe: Arc::clone(&probe) }))
        })
        .expect("controlled browser should start");
    probe.lock().expect("process probe lock").events.extend([
        WorkBrowserEvent::NavigationChanged("https://example.com/first".to_string()),
        WorkBrowserEvent::NavigationChanged("javascript:alert(1)".to_string()),
        WorkBrowserEvent::NavigationChanged("https://example.com/latest#section".to_string()),
    ]);

    assert_eq!(
        controller.take_latest_navigation_url().as_deref(),
        Some("https://example.com/latest#section")
    );
    assert_eq!(controller.take_latest_navigation_url(), None);
}

#[test]
fn work_browser_event_protocol_parses_json_encoded_navigation() {
    assert_eq!(
        parse_webview_event(r#"navigation "https://example.com/path?q=a%20b#part""#),
        Some(WorkBrowserEvent::NavigationChanged(
            "https://example.com/path?q=a%20b#part".to_string()
        ))
    );
    assert_eq!(parse_webview_event("WebView data path: /tmp/webview"), None);
    assert_eq!(parse_webview_event("navigation not-json"), None);
}

#[test]
fn work_browser_bounds_follow_main_window_and_work_split() {
    let layout = WorkBrowserLayout {
        window_x: 100.0,
        window_y: 200.0,
        window_width: 1_400.0,
        window_height: 900.0,
        work_sidebar_width: 298.0,
        work_divider_width: 6.0,
        top_bar_height: 28.0,
        resource_header_height: 134.0,
        horizontal_inset: 12.0,
        bottom_inset: 40.0,
        split_ratio: 0.6,
        resource_fullscreen: false,
        resource_half_fullscreen: false,
    };

    assert_eq!(
        work_browser_bounds(layout),
        Some(WebviewBounds { x: 1_074, y: 362, width: 414, height: 698 })
    );

    let half_fullscreen =
        work_browser_bounds(WorkBrowserLayout { resource_half_fullscreen: true, ..layout });
    assert_eq!(half_fullscreen, Some(WebviewBounds { x: 410, y: 362, width: 1_078, height: 698 }));

    let fullscreen = work_browser_bounds(WorkBrowserLayout { resource_fullscreen: true, ..layout });
    assert_eq!(fullscreen, Some(WebviewBounds { x: 112, y: 362, width: 1_376, height: 698 }));
}

#[test]
fn work_browser_hides_outside_work_browser_surface() {
    let visible = WorkBrowserSurface {
        screen: Screen::Project,
        workspace_mode: WorkspaceMode::Work,
        resource_mode: WorkResourceMode::Browser,
        resources_visible: true,
        system_settings_open: false,
        chat_fullscreen: false,
        chat_half_fullscreen: false,
    };
    assert!(work_browser_surface_visible(visible));

    for hidden in [
        WorkBrowserSurface { screen: Screen::Home, ..visible },
        WorkBrowserSurface { workspace_mode: WorkspaceMode::Code, ..visible },
        WorkBrowserSurface { resource_mode: WorkResourceMode::Overview, ..visible },
        WorkBrowserSurface { resources_visible: false, ..visible },
        WorkBrowserSurface { system_settings_open: true, ..visible },
        WorkBrowserSurface { chat_fullscreen: true, ..visible },
        WorkBrowserSurface { chat_half_fullscreen: true, ..visible },
    ] {
        assert!(!work_browser_surface_visible(hidden));
    }
}

#[test]
fn work_browser_controller_reports_launch_failure_without_child() {
    let mut controller = WorkBrowserController::default();
    let bounds = WebviewBounds { x: 0, y: 0, width: 800, height: 600 };
    let error = controller
        .ensure_started_with("https://example.com/", bounds, |_| {
            Err("未找到 vw-webview 辅助程序".to_string())
        })
        .expect_err("launch failure must remain explicit");

    assert!(error.contains("vw-webview"));
    assert!(!controller.has_running_child());
}
