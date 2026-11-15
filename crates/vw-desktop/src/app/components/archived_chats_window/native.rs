//! Real windows use the production page and production message reducer/tasks.
use desktop_settings_sessions::acceptance::{gateway, roundtrip, Scenarios};
use crate::app::message::settings::{
    SettingsMessage,
    archived_chats::{self, ArchivedChatsMessage as Archive},
};
use crate::app::{App, Message as AppMessage};
use iced::{Element, Task, window};
use std::path::PathBuf;

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
    frames: usize,
    root: PathBuf,
    scenarios: Scenarios,
}
impl Harness {
    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::Ui(AppMessage::Settings(SettingsMessage::ArchivedChats(message))) => {
                let task = archived_chats::update(&mut self.app, message);
                self.scenarios.state = self.app.archived_chats.clone();
                return task.map(Message::Ui);
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
                let name = self.scenarios.capture_name();
                image::save_buffer(
                    self.root.join(name),
                    &screenshot.rgba,
                    screenshot.size.width,
                    screenshot.size.height,
                    image::ColorType::Rgba8,
                )
                .unwrap();
                self.scenarios.case += 1;
                if self.scenarios.finished() {
                    eprintln!(
                        "PASS: 60 native archive scenarios; Chinese/ASCII/emoji shaped without panic"
                    );
                    return iced::exit();
                }
                self.frames = 0;
                self.scenarios.setup();
                self.app.archived_chats = self.scenarios.state.clone();
                return window::resize(self.window, iced::Size::new(self.scenarios.width(), 650.0));
            }
        }
        Task::none()
    }
    fn view(&self, _: window::Id) -> Element<'_, Message> {
        self.scenarios.view().map(|message| {
            Message::Ui(AppMessage::Settings(SettingsMessage::ArchivedChats(message)))
        })
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
    let mut state = std::mem::take(&mut app.archived_chats);
    let client = crate::app::config::gateway_client().unwrap();
    let (error, partial) = runtime.block_on(roundtrip(&mut state, client, async |state, message| {
        std::mem::swap(state, &mut app.archived_chats);
        send(&mut app, message).await;
        std::mem::swap(state, &mut app.archived_chats);
    }));
    app.archived_chats = state;
    std::fs::write(root.join("http-requests.txt"), server.requests.lock().unwrap().join("\n"))
        .unwrap();
    let reloaded = app.archived_chats.clone();
    iced::daemon(
        move || {
            let (window, task) = window::open(window::Settings {
                size: iced::Size::new(860.0, 650.0),
                ..Default::default()
            });
            let mut harness = Harness {
                scenarios: Scenarios {
                    state: Default::default(),
                    case: 0,
                    error,
                    partial,
                    reloaded,
                    identity: archived_chats::Identity::from_endpoint(
                        &crate::app::gateway_client_endpoint(),
                    ),
                },
                app,
                window,
                frames: 0,
                root,
            };
            harness.scenarios.setup();
            harness.app.archived_chats = harness.scenarios.state.clone();
            (harness, task.map(Message::Opened))
        },
        Harness::update,
        Harness::view,
    )
    .title("归档聊天隔离验收")
    .theme(|state: &Harness, _| state.scenarios.theme())
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
