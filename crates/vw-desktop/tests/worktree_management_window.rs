//! Explicit native production-component rendering acceptance.
#![allow(dead_code, unused_imports, clippy::all, clippy::pedantic)]
#[path = "../src/app/mod.rs"]
mod app;
#[path = "../src/apps/mod.rs"]
mod apps;
#[path = "../src/desktop_update_helper.rs"]
mod desktop_update_helper;
#[path = "../src/fonts.rs"]
mod fonts;
// 显式原生窗口验收：不执行 App 启动任务，不连接网关，不保存配置。
use crate::app::components::system_settings_worktrees as worktrees;
use crate::app::message::SettingsMessage;
use crate::app::state::WorktreesTab;
use crate::app::{App, Message as AppMessage};
use iced::widget::{container, text};
use iced::{Element, Length, Task, Theme, window};
use vw_api_worktree::WorktreeDeletionPreflight;

#[derive(Debug, Clone)]
enum Message {
    Ui(AppMessage),
    Opened(window::Id),
    Captured(window::Screenshot),
}
struct Harness {
    app: App,
    window: window::Id,
    case: usize,
    root: std::path::PathBuf,
}
impl Harness {
    fn setup(&mut self) -> Task<Message> {
        desktop_settings_workspace::acceptance::setup(&mut self.app.worktree_settings, self.case);
        let resize = window::resize(
            self.window,
            if self.case % 5 == 3 {
                iced::Size::new(360., 420.)
            } else {
                iced::Size::new(700., 680.)
            },
        );
        let id = self.window;
        let capture = Task::perform(
            async {
                tokio::time::sleep(std::time::Duration::from_millis(
                    std::env::var("WORKTREE_WINDOW_CAPTURE_MS")
                        .ok()
                        .and_then(|v| v.parse::<u64>().ok())
                        .unwrap_or(500)
                        .max(500),
                ))
                .await;
            },
            |_| (),
        )
        .then(move |_| window::screenshot(id))
        .map(Message::Captured);
        let scroll = if self.case % 5 == 4 {
            Task::perform(
                async {
                    tokio::time::sleep(std::time::Duration::from_millis(100)).await;
                },
                |_| (),
            )
            .then(|_| {
                iced::widget::operation::snap_to(
                    "worktree-window-page",
                    iced::widget::scrollable::RelativeOffset::END,
                )
            })
        } else {
            Task::none()
        };
        let focus =
            if self.case % 5 == 3 { iced::widget::operation::focus_next() } else { Task::none() };
        // Frame messages rebuild layout and invalidate the editor render buffer's
        // weak references. Capture the stable window without another App message.
        Task::batch([resize, scroll, focus, capture])
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::Ui(AppMessage::Settings(message)) => {
                let _ = crate::app::message::settings::update(&mut self.app, message);
            }
            Message::Ui(_) => {}
            Message::Opened(id) => {
                self.window = id;
                return self.setup();
            }
            Message::Captured(frame) => {
                let name = format!(
                    "{}-{}.png",
                    if self.case < 5 { "light" } else { "dark" },
                    ["general", "environment", "delete", "narrow-policy", "recovery"]
                        [self.case % 5]
                );
                image::save_buffer(
                    self.root.join(&name),
                    &frame.rgba,
                    frame.size.width,
                    frame.size.height,
                    image::ColorType::Rgba8,
                )
                .unwrap();
                eprintln!("CAPTURE {}", self.root.join(name).display());
                if self.case % 5 == 1 {
                    assert!(
                        self.app
                            .worktree_settings
                            .environment
                            .editor
                            .text()
                            .contains("中文环境初始化")
                    );
                    // Fixed fixture bounds exclude the input border and field labels.
                    let scale = frame.size.width as f32 / 700.0;
                    let mut ink = 0;
                    for y in (265.0 * scale) as usize..(300.0 * scale) as usize {
                        for x in (60.0 * scale) as usize..(600.0 * scale) as usize {
                            let offset = (y * frame.size.width as usize + x) * 4;
                            let rgb = &frame.rgba[offset..offset + 3];
                            if if self.case < 5 {
                                rgb.iter().all(|v| *v < 160)
                            } else {
                                rgb.iter().all(|v| *v > 160)
                            } {
                                ink += 1;
                            }
                        }
                    }
                    assert!(ink > 20, "environment command text was not painted: {ink}");
                    eprintln!("EDITOR_INK {ink}");
                }
                if std::env::var_os("WORKTREE_WINDOW_SINGLE_CASE").is_some() {
                    return iced::exit();
                }
                self.case += 1;
                if self.case == 10 {
                    eprintln!(
                        "PASS: 10 native worktree scenarios, light/dark, Chinese paths, environment, deletion, narrow policy; no glyph panic"
                    );
                    return iced::exit();
                }
                return self.setup();
            }
        }
        Task::none()
    }
    fn view(&self, _: window::Id) -> Element<'_, Message> {
        desktop_settings_workspace::acceptance::view(
            &self.app.worktree_settings,
            desktop_i18n::I18n::new(self.app.general_settings.desktop_locale),
            desktop_settings_workspace::environment::can_add(&crate::app::gateway_client_endpoint()),
            self.case,
        ).map(|event| Message::Ui(AppMessage::Settings(SettingsMessage::Worktrees(event))))
    }

    fn theme(&self) -> Theme {
        if self.case < 5 { Theme::Light } else { Theme::Dark }
    }
}
fn main() {
    let root = std::path::PathBuf::from(
        std::env::var("WORKTREE_WINDOW_OUTPUT").expect("explicit evidence directory"),
    );
    std::fs::create_dir_all(&root).unwrap();
    iced::daemon(
        move || {
            let (id, task) = window::open(window::Settings {
                size: iced::Size::new(700., 680.),
                ..Default::default()
            });
            let app = App::new().0;
            (
                Harness {
                    app,
                    window: id,
                    case: std::env::var("WORKTREE_WINDOW_SINGLE_CASE")
                        .ok()
                        .and_then(|v| v.parse::<usize>().ok())
                        .filter(|v| *v < 10)
                        .unwrap_or(0),
                    root: root.clone(),
                },
                task.map(Message::Opened),
            )
        },
        Harness::update,
        Harness::view,
    )
    .title("工作树实际渲染验收")
    .theme(|s: &Harness, _| s.theme())
    .settings(iced::Settings {
        fonts: crate::fonts::load_all(),
        default_font: iced::Font::with_name("Noto Sans CJK SC"),
        ..Default::default()
    })
    .run()
    .unwrap();
}
