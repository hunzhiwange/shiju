//! 显式 macOS 原生窗口验收：真实隐藏、Dock 重开、托盘菜单动作和窗口可见性。
//! cargo test -p vw-desktop --test tray-window --no-run，随后将可执行文件放入 .app 启动。
#[cfg(target_os = "macos")]
#[path = "../src/app/desktop_lifecycle/window.rs"]
mod lifecycle_window;

#[cfg(target_os = "macos")]
mod macos {
    use super::lifecycle_window;
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
    }

    struct Harness {
        window: window::Id,
        step: u8,
        started: Instant,
        menu: Option<Menu>,
        tray: Option<TrayIcon>,
    }

    impl Harness {
        fn update(&mut self, message: Message) -> Task<Message> {
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
                    lifecycle_window::hide_main_window(self.window)
                }
                1 => {
                    assert!(
                        app.isHidden(),
                        "close must hide the app without discarding Dock restoration"
                    );
                    let executable = std::env::current_exe().unwrap();
                    let bundle = executable.parent().unwrap().parent().unwrap().parent().unwrap();
                    assert_eq!(bundle.extension().unwrap(), "app");
                    // 向真实应用发送 reopen Apple Event，等价于从 Dock 打开。
                    let status = std::process::Command::new("/usr/bin/open")
                        .arg("-a")
                        .arg(bundle)
                        .status()
                        .unwrap();
                    assert!(status.success());
                    self.step = 2;
                    Task::none()
                }
                2 => {
                    if app.isHidden() {
                        return Task::none();
                    }
                    assert!(app.windows().iter().any(|window| window.isVisible()));
                    eprintln!("PASS: Dock reopened the hidden window");
                    self.step = 3;
                    lifecycle_window::hide_main_window(self.window)
                }
                3 => {
                    assert!(app.isHidden());
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
                        lifecycle_window::show_main_window(self.window)
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
                        .chain(lifecycle_window::hide_main_window(self.window))
                }
                6 => {
                    self.step = 7;
                    lifecycle_window::show_main_window(self.window)
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
        fn view(&self, _: window::Id) -> Element<'_, Message> {
            iced::widget::container(
                iced::widget::text("关闭后恢复窗口验收：Dock / 菜单栏 / 最小化").size(20),
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
                (
                    Harness {
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
