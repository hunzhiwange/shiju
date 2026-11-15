//! 显式 macOS 原生窗口验收：真实释放、重新创建、托盘菜单动作和窗口可见性。
//! cargo test -p vw-desktop --test tray-window --no-run，随后将可执行文件放入 .app 启动。
#[path = "../src/app/mod.rs"]
mod app;
#[path = "../src/apps/mod.rs"]
mod apps;
#[path = "../src/desktop_update_helper.rs"]
mod desktop_update_helper;
#[path = "../src/fonts.rs"]
mod fonts;

#[cfg(target_os = "macos")]
mod macos {
    use super::app::App;
    use iced::{Element, Task, window};
    use objc2::MainThreadMarker;
    use objc2_app_kit::{NSApplication, NSMenu};
    use std::time::{Duration, Instant};
    use tray_icon::{
        Icon, TrayIcon, TrayIconBuilder,
        menu::{ContextMenu, Menu, MenuEvent, MenuItem},
    };

    #[derive(Debug, Clone)]
    enum Message {
        Opened(window::Id),
        Tick,
        Ui(super::app::Message),
    }

    struct Harness {
        window: window::Id,
        app: App,
        step: u8,
        started: Instant,
        menu: Option<Menu>,
        tray: Option<TrayIcon>,
    }

    impl Harness {
        fn update(&mut self, message: Message) -> Task<Message> {
            if matches!(message, Message::Ui(_)) { return Task::none(); }
            if let Message::Opened(id) = message {
                self.window = id;
                return Task::none();
            }
            assert!(
                self.started.elapsed() < Duration::from_secs(20),
                "window restoration timed out at step {}",
                self.step
            );
            let mtm = MainThreadMarker::new().expect("native UI thread");
            let app = NSApplication::sharedApplication(mtm);
            match self.step {
                0 => {
                    if !app.windows().iter().any(|window| window.isVisible()) {
                        return Task::none();
                    }
                    let menu = Menu::new();
                    menu.append(&MenuItem::with_id("show", "显示主窗口", true, None)).unwrap();
                    let rgba = image::load_from_memory(include_bytes!(
                        "../../../assets/logo/icons.iconset/icon_32x32.png"
                    ))
                    .unwrap()
                    .into_rgba8();
                    let (w, h) = rgba.dimensions();
                    self.tray = Some(
                        TrayIconBuilder::new()
                            .with_menu(Box::new(menu.clone()))
                            .with_icon(Icon::from_rgba(rgba.into_raw(), w, h).unwrap())
                            .build()
                            .unwrap(),
                    );
                    self.menu = Some(menu);
                    assert!(app.windows().iter().any(|window| window.isVisible()));
                    self.step = 1;
                    self.app.release_main_window().map(Message::Ui)
                }
                1 => {
                    assert!(self.app.main_window_id.is_none());
                    assert!(!app.windows().iter().any(|window| window.isVisible()));
                    // 当前轻量后台会释放窗口；验收真实 App 重建入口。
                    // 此处不声称覆盖 Dock Apple Event 的投递。
                    self.step = 2;
                    self.reopen()
                }
                2 => {
                    if !app.windows().iter().any(|window| window.isVisible()) {
                        return Task::none();
                    }
                    assert!(app.windows().iter().any(|window| window.isVisible()));
                    eprintln!("PASS: released window recreated");
                    self.step = 3;
                    self.app.release_main_window().map(Message::Ui)
                }
                3 => {
                    assert!(self.app.main_window_id.is_none());
                    assert!(!app.windows().iter().any(|window| window.isVisible()));
                    // 真正执行 NSMenuItem 的 action，经过 muda 事件队列，不直接伪造恢复消息。
                    let raw = self.menu.as_ref().unwrap().ns_menu().cast::<NSMenu>();
                    // SAFETY: Menu 持有 NSMenu，且本回调运行在 UI 主线程。
                    unsafe {
                        (&*raw).performActionForItemAtIndex(0);
                    }
                    self.step = 4;
                    Task::none()
                }
                4 => {
                    if MenuEvent::receiver().try_iter().any(|event| event.id.as_ref() == "show") {
                        self.step = 5;
                        self.reopen()
                    } else {
                        Task::none()
                    }
                }
                5 => {
                    assert!(!app.isHidden());
                    assert!(
                        app.windows()
                            .iter()
                            .any(|window| window.isVisible() && !window.isMiniaturized())
                    );
                    eprintln!("PASS: tray menu restored a visible window");
                    self.step = 6;
                    window::minimize(self.window, true)
                }
                6 => {
                    self.step = 7;
                    self.reopen()
                }
                _ => {
                    assert!(!app.isHidden());
                    assert!(
                        app.windows()
                            .iter()
                            .any(|window| window.isVisible() && !window.isMiniaturized())
                    );
                    eprintln!("PASS: minimized window restored; 3 native window scenarios passed");
                    self.tray.take();
                    iced::exit()
                }
            }
        }
        fn reopen(&mut self) -> Task<Message> {
            let task = self.app.reopen_main_window();
            self.window = self.app.main_window_id.expect("reopen assigns a window");
            task.map(Message::Ui)
        }

        fn view(&self, _: window::Id) -> Element<'_, Message> {
            iced::widget::container(
                iced::widget::text("关闭后恢复窗口验收：重建 / 菜单栏 / 最小化").size(20),
            )
            .center(iced::Length::Fill)
            .into()
        }
    }

    pub fn run() {
        iced::daemon(
            || {
                let (id, task) = window::open(window::Settings {
                    exit_on_close_request: false,
                    size: iced::Size::new(640.0, 300.0),
                    ..Default::default()
                });
                let (mut app, _) = App::new();
                app.main_window_id = Some(id);
                (
                    Harness {
                        app,
                        window: id,
                        step: 0,
                        started: Instant::now(),
                        menu: None,
                        tray: None,
                    },
                    task.map(Message::Opened),
                )
            },
            Harness::update,
            Harness::view,
        )
        .title("ShiJu window restoration verification")
        .subscription(|_| iced::time::every(Duration::from_millis(500)).map(|_| Message::Tick))
        .theme(|_: &Harness, _: window::Id| iced::Theme::Dark)
        .run()
        .unwrap();
    }
}

#[cfg(target_os = "macos")]
fn main() {
    macos::run();
}
#[cfg(not(target_os = "macos"))]
fn main() {
    panic!("tray-window requires macOS");
}
