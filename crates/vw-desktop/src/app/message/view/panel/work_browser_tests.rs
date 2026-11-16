use std::io;
use std::sync::{Arc, Mutex};

use crate::app::state::{WorkResourceMode, WorkspaceMode};
use crate::app::webview_runtime::{BrowserProcess, WebviewBounds, WorkBrowserEvent};
use crate::app::{App, Message, Screen};

#[test]
fn work_browser_reserves_status_bar_at_bottom() {
    assert_eq!(super::BROWSER_BOTTOM_INSET, crate::app::impls::STATUS_BAR_HEIGHT + 10.0);
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

fn visible_work_browser_app() -> App {
    let mut app = App::new().0;
    app.navigation_tabs.screen = Screen::Project;
    app.navigation_mode.workspace_mode = WorkspaceMode::Work;
    app.navigation_workspace.work_resource_mode = WorkResourceMode::Browser;
    app.window_size = (1_400.0, 900.0);
    app
}

#[test]
fn terminal_fullscreen_keeps_native_browser_above_dock_and_hides_it_when_expanded() {
    use crate::app::message::TerminalMessage;

    let mut app = visible_work_browser_app();
    app.navigation_workspace.work_resources_visible = true;
    app.work_browser.closed = false;
    app.work_browser.address = "https://example.com/".into();
    app.terminal.set_visible(false);
    app.terminal.height = 220.0;
    let process = NavigationProcess::default();
    let commands = Arc::clone(&process.commands);
    app.work_browser_controller
        .ensure_started_with(
            &app.work_browser.address,
            WebviewBounds { x: 0, y: 0, width: 800, height: 600 },
            |_| Ok(Box::new(process)),
        )
        .expect("controlled browser");

    let take_commands = || std::mem::take(&mut *commands.lock().unwrap());
    let resize_height = |commands: &[String]| {
        commands
            .iter()
            .find_map(|command| {
                let command = command.strip_prefix("resize ")?;
                command.split_whitespace().nth(1)?.parse::<u32>().ok()
            })
            .expect("browser should resize")
    };

    super::sync_lifecycle_result(&mut app).unwrap();
    let hidden_terminal_height = resize_height(&take_commands());
    app.terminal.set_visible(true);
    super::sync_lifecycle_result(&mut app).unwrap();
    assert_eq!(resize_height(&take_commands()), hidden_terminal_height - 220);

    let _ = app.update(Message::Terminal(TerminalMessage::ToggleFullscreen));
    assert_eq!(take_commands(), ["hide\n"]);
    let _ = app.update(Message::Terminal(TerminalMessage::ToggleFullscreen));
    assert_eq!(take_commands(), ["show\n"]);
    assert_eq!(app.work_browser.address, "https://example.com/");

    app.window_size.1 = 300.0;
    assert!(super::sync_lifecycle_result(&mut app).is_err());
    assert_eq!(take_commands(), ["hide\n"]);
}
