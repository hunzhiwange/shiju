//! Real windows use the production page and production message reducer/tasks.
use crate::acceptance::fixture;
use crate::app::components::system_settings_archived_chats::page;
use crate::app::message::settings::{
    SettingsMessage,
    archived_chats::{self, ArchivedChatsMessage as Archive},
};
use crate::app::state::{ArchivedChatsSort, ArchivedChatsState};
use crate::app::{App, Message as AppMessage};
use iced::{Element, Task, Theme, window};
use std::path::PathBuf;
mod gateway;

// Execute the production async task graph, including automatic reloads. No fake
// completion messages are injected. Toast expiry does not affect archive state.
async fn send(app: &mut App, message: Archive) {
    use iced::futures::StreamExt;
    let mut tasks = vec![archived_chats::update(app, message)];
    while let Some(task) = tasks.pop() {
        if let Some(mut stream) = iced_runtime::task::into_stream(task) {
            while let Some(action) = stream.next().await {
                if let iced_runtime::Action::Output(AppMessage::Settings(
                    SettingsMessage::ArchivedChats(message),
                )) = action
                {
                    tasks.push(archived_chats::update(app, message));
                }
            }
        }
    }
}

async fn roundtrip(app: &mut App) -> (ArchivedChatsState, ArchivedChatsState) {
    send(app, Archive::Refresh).await;
    assert!(app.archived_chats.error.is_some(), "injected list failure");
    let error = app.archived_chats.clone();
    send(app, Archive::Refresh).await;
    assert_eq!(app.archived_chats.sessions.len(), 1178);
    let row = app.archived_chats.sessions[0].clone();
    // Persist archive through the real HTTP client before listing and restoring.
    let client = crate::app::config::gateway_client().unwrap();
    let _: vw_shared::session::info::Info = client
        .session_update(
            &row.id,
            Some(&row.directory),
            &vw_gateway_client::GatewaySessionPatchBody {
                title: None,
                time: Some(vw_gateway_client::GatewaySessionPatchTime { archived: Some(10) }),
            },
        )
        .await
        .unwrap();
    send(app, Archive::Refresh).await;
    send(app, Archive::Restore(row.id.clone())).await;
    assert!(
        app.archived_chats
            .sessions
            .iter()
            .any(|row| row.id == "chat-0000" && row.time.archived.is_some())
    );
    assert!(app.archived_chats.operating_ids.is_empty());
    send(app, Archive::Restore(row.id)).await;
    send(app, Archive::Refresh).await;
    assert!(
        app.archived_chats
            .sessions
            .iter()
            .find(|row| row.id == "chat-0000")
            .unwrap()
            .time
            .archived
            .is_none()
    );
    assert_eq!(
        app.archived_chats.groups().iter().map(|group| group.rows.len()).sum::<usize>(),
        1177
    );
    send(app, Archive::DeleteRequested("chat-0001".into())).await;
    send(app, Archive::DeleteConfirmed).await;
    send(app, Archive::Refresh).await;
    assert!(
        !app.archived_chats
            .sessions
            .iter()
            .any(|row| ["chat-0001", "chat-0002"].contains(&row.id.as_str()))
    );
    send(app, Archive::ProjectDeleteRequested("batch".into())).await;
    send(app, Archive::DeleteConfirmed).await;
    let batch = app.archived_chats.project_delete.as_ref().unwrap();
    assert_eq!((batch.succeeded, batch.failed.len()), (1, 1));
    let partial = app.archived_chats.clone();
    send(app, Archive::ProjectDeleteRetry).await;
    assert_eq!(app.archived_chats.project_delete.as_ref().unwrap().targets.len(), 1);
    send(app, Archive::DeleteConfirmed).await;
    send(app, Archive::Refresh).await;
    assert_eq!(app.archived_chats.sessions.len(), 1174);
    assert!(!app.archived_chats.groups().iter().any(|group| group.project.id == "batch"));
    eprintln!(
        "PASS: 3 HTTP roundtrips: archive/restore/reload, descendant delete/reload, partial batch/retry/reload"
    );
    (error, partial)
}

const CASES: [&str; 15] = [
    "list",
    "sort-closed",
    "project-closed",
    "actions",
    "filtered",
    "created",
    "title",
    "delete",
    "deleting",
    "loading",
    "error",
    "empty",
    "restoring",
    "partial",
    "reloaded",
];
#[derive(Clone, Debug)]
enum Message {
    Ui(AppMessage),
    Frame,
    Opened(window::Id),
    Capture(window::Screenshot),
}
struct Harness {
    app: App,
    window: window::Id,
    case: usize,
    frames: usize,
    root: PathBuf,
    error: ArchivedChatsState,
    partial: ArchivedChatsState,
    reloaded: ArchivedChatsState,
}
impl Harness {
    fn setup(&mut self) {
        self.app.archived_chats = ArchivedChatsState {
            sessions: fixture(),
            identity: Some(archived_chats::Identity::from_endpoint(
                &crate::app::gateway_client_endpoint(),
            )),
            project_names: [
                ("batch".into(), "同名项目".into()),
                ("duplicate".into(), "同名项目".into()),
            ]
            .into(),
            ..Default::default()
        };
        let identity = self.app.archived_chats.identity.clone().unwrap();
        let state = &mut self.app.archived_chats;
        let message = match self.case % CASES.len() {
            3 => Some(Archive::ProjectActionsToggled("batch".into())),
            4 => Some(Archive::ProjectSelected(Some("duplicate".into()))),
            5 => Some(Archive::SortSelected(ArchivedChatsSort::Created)),
            6 => Some(Archive::SortSelected(ArchivedChatsSort::Title)),
            7 | 8 => Some(Archive::DeleteRequested("chat-0000".into())),
            9 => Some(Archive::Refresh),
            10 => {
                *state = self.error.clone();
                None
            }
            11 => {
                state.sessions.clear();
                None
            }
            12 => Some(Archive::Restore("chat-0000".into())),
            13 => {
                *state = self.partial.clone();
                None
            }
            14 => {
                *state = self.reloaded.clone();
                None
            }
            _ => None,
        };
        if let Some(message) = message {
            archived_chats::apply(state, message, &identity);
        }
        if self.case % CASES.len() == 8 {
            archived_chats::apply(state, Archive::DeleteConfirmed, &identity);
        }
    }
    fn theme(&self) -> Theme {
        if self.case / CASES.len() % 2 == 0 { Theme::Light } else { Theme::Dark }
    }
    fn width(&self) -> f32 {
        if self.case < CASES.len() * 2 { 860.0 } else { 360.0 }
    }
    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::Ui(AppMessage::Settings(SettingsMessage::ArchivedChats(message))) => {
                return archived_chats::update(&mut self.app, message).map(Message::Ui);
            }
            Message::Ui(_) => {}
            Message::Opened(id) => self.window = id,
            Message::Frame => {
                self.frames += 1;
                if self.frames == 8 {
                    return window::screenshot(self.window).map(Message::Capture);
                }
            }
            Message::Capture(screenshot) => {
                assert!(screenshot.rgba.chunks_exact(4).any(|p| p[0] != p[1]));
                let name = format!(
                    "{}-{}-{}.png",
                    if self.case / CASES.len() % 2 == 0 { "light" } else { "dark" },
                    self.width(),
                    CASES[self.case % CASES.len()]
                );
                image::save_buffer(
                    self.root.join(name),
                    &screenshot.rgba,
                    screenshot.size.width,
                    screenshot.size.height,
                    image::ColorType::Rgba8,
                )
                .unwrap();
                self.case += 1;
                if self.case == CASES.len() * 4 {
                    eprintln!(
                        "PASS: 60 native archive scenarios; Chinese/ASCII/emoji shaped without panic"
                    );
                    return iced::exit();
                }
                self.frames = 0;
                self.setup();
                return window::resize(self.window, iced::Size::new(self.width(), 650.0));
            }
        }
        Task::none()
    }
    fn view(&self, _: window::Id) -> Element<'_, Message> {
        let content: Element<'_, AppMessage> = iced::widget::container(page(
            &self.app.archived_chats,
            vw_config_types::ui::DesktopLocale::SimplifiedChinese,
        ))
        .padding(16)
        .width(iced::Length::Fill)
        .height(iced::Length::Fill)
        .into();
        content.map(Message::Ui)
    }
}
pub(crate) fn run() {
    let root = PathBuf::from(
        std::env::var_os("SHIJU_ARCHIVED_WINDOW_OUTPUT").expect("explicit evidence output"),
    );
    assert!(root.is_absolute());
    std::fs::create_dir_all(&root).unwrap();
    let server = gateway::Gateway::start();
    crate::app::config::install_window_test_bootstrap(server.port);
    let runtime = tokio::runtime::Runtime::new().unwrap();
    let (mut app, _) = {
        let _guard = runtime.enter();
        App::new()
    };
    let (error, partial) = runtime.block_on(roundtrip(&mut app));
    std::fs::write(root.join("http-requests.txt"), server.requests.lock().unwrap().join("\n"))
        .unwrap();
    let reloaded = app.archived_chats.clone();
    iced::daemon(
        move || {
            let (window, task) = window::open(window::Settings {
                size: iced::Size::new(860.0, 650.0),
                ..Default::default()
            });
            let mut harness =
                Harness { app, window, case: 0, frames: 0, root, error, partial, reloaded };
            harness.setup();
            (harness, task.map(Message::Opened))
        },
        Harness::update,
        Harness::view,
    )
    .title("归档聊天隔离验收")
    .theme(|state: &Harness, _| state.theme())
    .subscription(|_| window::frames().map(|_| Message::Frame))
    .settings(iced::Settings {
        fonts: crate::fonts::load_all(),
        default_font: iced::Font::with_name("PingFang SC"),
        ..Default::default()
    })
    .run()
    .unwrap();
    drop(server);
}
