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
use vw_gateway_client::vw_api_types::worktree::WorktreeDeletionPreflight;

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
        let state = &mut self.app.worktree_settings;
        *state = Default::default();
        state.saved = Some(Default::default());
        state.default_root = "/项目/中文路径/工作树".into();
        state.managed.groups = vec![crate::app::state::ManagedWorktreeGroup {
            project: serde_json::from_value(serde_json::json!({
                "id":"fixture", "name":"中文项目与长标题", "directory":"/项目/中文路径",
                "display_path":"/项目/中文路径", "status":"ready", "created_at_ms":1,"updated_at_ms":2,
                "git":{"is_repo":true,"has_uncommitted_changes":false}
            })).unwrap(),
            items: vec![serde_json::from_value(serde_json::json!({
                "worktree":{"id":"checkout", "project_id":"fixture","name":"中文工作树", "branch":"feature",
                    "directory":"/工作树/很长的中文路径用于验证换行和字形整形/原目录", "status":"error", "created_at_ms":1,"updated_at_ms":2},
                "title":"中文聊天标题，用于检查很长的动态文本是否安全渲染", "sessions":[],"protection_reasons":[], "recovered_from":null
            })).unwrap()], collapsed:false, error:None,
        }];
        state.recovery.restored =
            Some(("fixture".into(), "unavailable".into(), "/项目/已恢复的中文路径".into()));

        match self.case % 5 {
            1 => {
                state.active_tab = WorktreesTab::Environment;
                state.environment.loaded = true;
                state.environment.selected = Some(serde_json::from_value(serde_json::json!({
                    "id":"fixture", "name":"中文项目与很长很长的环境初始化标题", "directory":"/项目/中文路径",
                    "display_path":"/项目/中文路径", "status":"ready", "created_at_ms":1,"updated_at_ms":2,
                    "git":{"is_repo":true,"has_uncommitted_changes":false}
                })).unwrap());
                state.environment.editor = iced::widget::text_editor::Content::with_text(
                    "echo 中文环境初始化\necho ready",
                );
            }
            2 => {
                state.managed.preflight_target =
                    Some(("中文项目".into(), "长标题工作树".repeat(8)));
                state.managed.preflight = Some(WorktreeDeletionPreflight {
                    fingerprint: "fixture".into(),
                    expires_at_ms: u64::MAX,
                    confirmation_required: true,
                    has_changes: true,
                    has_ignored_files: true,
                    protection_reasons: vec![],
                });
                state.recovery.error =
                    Some("二进制修改将保存到恢复快照；忽略文件需要额外确认".into());
            }
            3 => {
                state.draft = Some(vw_config_types::worktree::WorktreeSettings::default().into());
            }
            _ => {}
        }
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
        let page = iced::widget::scrollable(container(worktrees::view(&self.app)).padding([0, 7]))
            .id("worktree-window-page")
            .direction(iced::widget::scrollable::Direction::Vertical(
                iced::widget::scrollable::Scrollbar::new().width(4).scroller_width(4),
            ))
            .height(Length::Fill);
        let base = container(page).padding(14).center_x(Length::Fill).center_y(Length::Fill).into();
        iced::widget::keyed::Column::with_children([(
            self.case,
            worktrees::view_overlays(&self.app, base).map(Message::Ui),
        )])
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
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
