//! 显式原生窗口验收：不执行 App 启动任务，不连接网关，不保存配置。
use crate::app::components::system_settings_cron::date_time;
use crate::app::message::SettingsMessage;
use crate::app::{App, Message as AppMessage};
use chrono::{NaiveDate, NaiveTime};
use iced::widget::{container, text};
use iced::{Element, Length, Task, Theme, window};

#[derive(Debug, Clone)]
enum Message {
    Ui(AppMessage),
    Frame,
    Opened(window::Id),
    Captured(window::Screenshot),
}
struct Harness {
    app: App,
    window: window::Id,
    case: usize,
    frames: usize,
    root: std::path::PathBuf,
}
impl Harness {
    fn setup(&mut self) -> Task<Message> {
        self.app.cron_settings = Default::default();
        self.app.cron_settings.add_draft.at = "2024-02-29T09:30:00Z".into();
        let mut picker = date_time::DateTimePicker::new(
            false,
            NaiveDate::from_ymd_opt(2024, 2, 29).unwrap(),
            NaiveTime::from_hms_opt(9, 30, 0).unwrap(),
        );
        match self.case % 4 {
            1 => {
                picker.select(NaiveDate::from_ymd_opt(2024, 12, 31).unwrap());
                picker.navigate(1);
                picker.select(NaiveDate::from_ymd_opt(2025, 1, 1).unwrap());
            }
            3 => {
                picker.navigate(12);
                picker.select(NaiveDate::from_ymd_opt(2025, 2, 28).unwrap());
            }
            _ => {}
        }
        self.app.cron_settings.date_time_picker = Some(picker);
        if self.case % 4 == 2 {
            let _ = crate::app::message::settings::update(
                &mut self.app,
                SettingsMessage::CronPickerCancel,
            );
            assert!(self.app.cron_settings.date_time_picker.is_none());
            assert_eq!(self.app.cron_settings.add_draft.at, "2024-02-29T09:30:00Z");
        }
        window::resize(
            self.window,
            if self.case % 4 == 3 {
                iced::Size::new(360., 420.)
            } else {
                iced::Size::new(700., 680.)
            },
        )
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
            Message::Frame => {
                self.frames += 1;
                if self.frames == 45 {
                    return window::screenshot(self.window).map(Message::Captured);
                }
            }
            Message::Captured(frame) => {
                let name = format!(
                    "{}-{}.png",
                    if self.case < 4 { "light" } else { "dark" },
                    ["leap-selected", "cross-year", "cancel", "narrow"][self.case % 4]
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
                self.case += 1;
                if self.case == 8 {
                    eprintln!(
                        "PASS: 8 native picker scenarios, light/dark, Chinese title, selection, cross-year, cancel, narrow; no glyph panic"
                    );
                    return iced::exit();
                }
                self.frames = 0;
                return self.setup();
            }
        }
        Task::none()
    }
    fn view(&self, _: window::Id) -> Element<'_, Message> {
        let base = container(
            text(format!("定时任务草稿时间：{}", self.app.cron_settings.add_draft.at)).size(16),
        )
        .center_x(Length::Fill)
        .center_y(Length::Fill)
        .into();
        crate::app::components::system_settings_cron::view_overlays(&self.app, base)
            .map(Message::Ui)
    }
    fn theme(&self) -> Theme {
        if self.case < 4 { Theme::Light } else { Theme::Dark }
    }
}
pub fn run() {
    let root = std::path::PathBuf::from(
        std::env::var("CRON_PICKER_WINDOW_OUTPUT").expect("explicit evidence directory"),
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
                Harness { app, window: id, case: 0, frames: 0, root: root.clone() },
                task.map(Message::Opened),
            )
        },
        Harness::update,
        Harness::view,
    )
    .title("日期时间选择器实际渲染验收")
    .theme(|s: &Harness, _| s.theme())
    .subscription(|_| window::frames().map(|_| Message::Frame))
    .settings(iced::Settings {
        fonts: crate::fonts::load_all(),
        default_font: iced::Font::with_name("Noto Sans CJK SC"),
        ..Default::default()
    })
    .run()
    .unwrap();
}
