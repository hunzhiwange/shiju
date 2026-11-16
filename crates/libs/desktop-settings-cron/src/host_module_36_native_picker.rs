#![allow(dead_code)]
//! 显式原生窗口验收：不执行 App 启动任务，不连接网关，不保存配置。
use crate::message::{Message as AppMessage, SettingsMessage};
use crate::state::CronSettingsState;
use crate::system_settings_cron::{self as domain, date_time};
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
    state: CronSettingsState,
    app_theme: Theme,
    window: window::Id,
    case: usize,
    frames: usize,
    root: std::path::PathBuf,
}
impl Harness {
    fn data(&self) -> domain::ViewData<'_> {
        domain::ViewData {
            cron_settings: &self.state,
            desktop_locale: config_ui::DesktopLocale::SimplifiedChinese,
            app_theme: &self.app_theme,
            project_path: None,
            recent_projects: &[],
            agents: Vec::new(),
            providers: Vec::new(),
            delivery: Default::default(),
            scheduler_error: None,
            last_copied_code_hash: None,
            help_content: |title, body, _, _| iced::widget::column![text(title), text(body)].into(),
        }
    }
    fn apply(&mut self, event: SettingsMessage) {
        use crate::update::messages::SettingsMessage as Update;
        let event = match event {
            SettingsMessage::CronPickerOpen { editing } => Update::CronPickerOpen { editing },
            SettingsMessage::CronPickerNavigate(months) => Update::CronPickerNavigate(months),
            SettingsMessage::CronPickerDate(date) => Update::CronPickerDate(date),
            SettingsMessage::CronPickerTime { hour, minute } => {
                Update::CronPickerTime { hour, minute }
            }
            SettingsMessage::CronPickerCancel => Update::CronPickerCancel,
            SettingsMessage::CronPickerConfirm => Update::CronPickerConfirm,
            _ => panic!("native date-time fixture received a non-picker event"),
        };
        let config = desktop_config::ConfigAccess::new(
            gateway_endpoint::GatewayEndpoint::new("127.0.0.1", 9),
            None,
        );
        drop(crate::update::cron::update(
            &mut crate::update::cron::Inputs {
                cron_settings: &mut self.state,
                desktop_locale: config_ui::DesktopLocale::SimplifiedChinese,
                delivery: Default::default(),
                client: Err("native fixture unavailable".into()),
                config: &config,
                notices: &mut Vec::new(),
            },
            event,
        ));
    }

    fn setup(&mut self) -> Task<Message> {
        self.state = Default::default();
        self.state.add_draft.at = "2024-02-29T09:30:00Z".into();
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
        self.state.date_time_picker = Some(picker);
        if self.case % 4 == 2 {
            let _ = self.apply(SettingsMessage::CronPickerCancel);
            assert!(self.state.date_time_picker.is_none());
            assert_eq!(self.state.add_draft.at, "2024-02-29T09:30:00Z");
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
                let _ = self.apply(message);
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
        let base =
            container(text(format!("定时任务草稿时间：{}", self.state.add_draft.at)).size(16))
                .center_x(Length::Fill)
                .center_y(Length::Fill)
                .into();
        domain::view_overlays(&self.data(), base, |message| message).map(Message::Ui)
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
            let state = CronSettingsState::default();
            (
                Harness {
                    state,
                    app_theme: Theme::Light,
                    window: id,
                    case: 0,
                    frames: 0,
                    root: root.clone(),
                },
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
        fonts: desktop_assets::fonts::load_all(),
        default_font: iced::Font::with_name("Noto Sans CJK SC"),
        ..Default::default()
    })
    .run()
    .unwrap();
}
