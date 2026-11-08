//! Shared session rows for the code and work sidebars.

use std::time::Duration;

use chrono::{Local, TimeZone};
use iced::widget::tooltip::{Position, Tooltip};
use iced::widget::{Space, button, column, container, hover, row, svg, text};
use iced::{Alignment, Background, Border, Color, Element, Length, Theme};
use vw_shared::session::info::Info;

use crate::app::components::status_animation::spinner_frame;
use crate::app::components::system_settings_common::{
    settings_muted_text_style, settings_panel_style,
};
use crate::app::components::widgets::SingleLineLabel;
use crate::app::i18n::I18n;
use crate::app::message::ProjectMessage;
use crate::app::{App, Message};

use super::super::styles::{is_dark_theme, session_row_highlight_color, tooltip_bubble};
use super::super::utils::mix_color;

const ACTION_SIZE: f32 = 22.0;
const ACTION_GAP: f32 = 2.0;
const ROW_PADDING: f32 = 8.0;
const PIN: &[u8] =
    include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../assets/icons/bootstrap/pin.svg"));
const PINNED: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../assets/icons/bootstrap/pin-fill.svg"
));
const ARCHIVE: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../assets/icons/bootstrap/archive.svg"
));

pub(in crate::app::views::project) struct SessionRowDetails {
    title: String,
    directory: String,
    pub updated: u64,
    project: String,
    project_id: String,
    created: u64,
    changes: Option<(i64, i64)>,
}

impl SessionRowDetails {
    pub(in crate::app::views::project) fn new(session: &Info, project: String) -> Self {
        Self {
            title: session.title.clone(),
            directory: session.directory.clone(),
            updated: session.time.updated,
            project,
            project_id: session.project_id.clone(),
            created: session.time.created,
            changes: session.summary.as_ref().map(|summary| (summary.additions, summary.deletions)),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::app::views::project) enum SessionRowStatus {
    Idle,
    Running,
    Queued(usize),
    Updated,
}

impl SessionRowStatus {
    pub(in crate::app::views::project) fn for_session(app: &App, id: &str) -> Self {
        match app.session_runtime_states.get(id) {
            Some(runtime) if runtime.is_requesting => Self::Running,
            Some(runtime) if !runtime.queue.is_empty() => Self::Queued(runtime.queue.len()),
            Some(runtime) if runtime.has_unseen_success => Self::Updated,
            _ => Self::Idle,
        }
    }

    fn indicator_width(self) -> f32 {
        match self {
            Self::Idle => 0.0,
            Self::Queued(_) | Self::Updated => 14.0,
            Self::Running => 28.0,
        }
    }
}

pub(super) fn session_row_button_style(
    theme: &Theme,
    active: bool,
    status: button::Status,
) -> button::Style {
    let highlight = session_row_highlight_color(theme).scale_alpha(if is_dark_theme(theme) {
        0.72
    } else {
        0.58
    });
    let background = match status {
        button::Status::Pressed => {
            theme.extended_palette().background.strong.color.scale_alpha(0.42)
        }
        button::Status::Hovered => highlight,
        _ if active => highlight,
        _ => Color::TRANSPARENT,
    };
    button::Style {
        background: Some(Background::Color(background)),
        text_color: theme.palette().text,
        border: Border { radius: 8.0.into(), ..Default::default() },
        ..Default::default()
    }
}

pub(super) fn running_status_color(breathe: f32) -> Color {
    mix_color(Color::from_rgb8(245, 158, 11), Color::from_rgb8(239, 68, 68), breathe)
}

pub(super) fn queued_status_color(breathe: f32) -> Color {
    running_status_color(breathe * 0.75)
}

fn status_icon(id: &str, status: SessionRowStatus, frame: usize) -> Element<'static, Message> {
    let session_phase =
        id.bytes().fold(0u32, |acc, byte| acc.wrapping_mul(16777619).wrapping_add(byte as u32));
    let now = crate::app::time::now_ms() as f64 / 1000.0;
    let phase =
        (now / 1.8 + f64::from(session_phase) / f64::from(u32::MAX)) * std::f64::consts::TAU;
    let breathe = ((phase.sin() + 1.0) * 0.5) as f32;
    let indicator = match status {
        SessionRowStatus::Running => row![
            text(spinner_frame(frame))
                .size(11)
                .line_height(text::LineHeight::Relative(1.0))
                .color(Color::from_rgb8(239, 68, 68)),
            text("●").size(12).color(running_status_color(breathe)),
        ],
        SessionRowStatus::Queued(_) => {
            row![text("●").size(8.5).color(queued_status_color(breathe))]
        }
        SessionRowStatus::Updated => row![text("●").size(11).color(Color::from_rgb8(46, 184, 114))],
        SessionRowStatus::Idle => row![],
    };
    container(indicator.spacing(3).align_y(Alignment::Center))
        .width(status.indicator_width())
        .align_x(Alignment::End)
        .into()
}

fn action(icon: &'static [u8], label: &str, message: Message) -> Element<'static, Message> {
    let button = button(
        container(
            svg(svg::Handle::from_memory(icon))
                .width(13)
                .height(13)
                .style(|theme: &Theme, _| svg::Style { color: Some(theme.palette().text) }),
        )
        .center(ACTION_SIZE),
    )
    .padding(0)
    .on_press(message)
    .style(|theme, status| session_row_button_style(theme, false, status));
    Tooltip::new(button, tooltip_bubble(label.to_owned()), Position::Top).gap(4).into()
}

fn details_style(theme: &Theme) -> container::Style {
    let mut style = settings_panel_style(theme);
    style.background = Some(theme.palette().background.into());
    style.text_color = Some(theme.palette().text);
    style.border.radius = 12.0.into();
    style
}

fn detail_line(label: &'static str, value: String) -> Element<'static, Message> {
    row![
        text(label).size(11).style(settings_muted_text_style),
        text(value).size(12).width(Length::Fill).wrapping(text::Wrapping::WordOrGlyph),
    ]
    .spacing(9)
    .into()
}

fn local_time(timestamp: u64) -> String {
    i64::try_from(timestamp)
        .ok()
        .filter(|timestamp| *timestamp > 0)
        .and_then(|timestamp| Local.timestamp_millis_opt(timestamp).single())
        .map_or_else(|| "—".to_owned(), |time| time.format("%Y-%m-%d %H:%M").to_string())
}

fn details_card(
    i18n: I18n,
    title: String,
    details: &SessionRowDetails,
    status: SessionRowStatus,
) -> Element<'static, Message> {
    let changes: Element<'static, Message> = if let Some((added, deleted)) = details.changes {
        row![
            text(format!("+{added}")).size(12).style(|theme: &Theme| text::Style {
                color: Some(if is_dark_theme(theme) {
                    Color::from_rgb8(74, 222, 128)
                } else {
                    Color::from_rgb8(21, 128, 61)
                }),
            }),
            text(format!("-{deleted}")).size(12).style(|theme: &Theme| text::Style {
                color: Some(if is_dark_theme(theme) {
                    Color::from_rgb8(252, 129, 129)
                } else {
                    Color::from_rgb8(185, 28, 28)
                }),
            }),
        ]
        .spacing(6)
        .into()
    } else {
        text("—").size(12).style(settings_muted_text_style).into()
    };
    let relative = crate::app::i18n::chat::relative_time(
        i18n,
        crate::app::components::chat_panel::utils::relative_time_bucket(
            details.updated,
            crate::app::time::now_ms(),
        ),
    );
    let mut content = column![
        text(title)
            .size(14)
            .font(iced::Font { weight: iced::font::Weight::Semibold, ..Default::default() })
            .width(Length::Fill)
            .wrapping(text::Wrapping::WordOrGlyph),
        detail_line(i18n.__("项目"), details.project.clone()),
        detail_line(i18n.__("工作区"), details.directory.clone()),
        detail_line(i18n.__("创建时间"), local_time(details.created)),
        detail_line(i18n.__("更新时间"), format!("{} · {relative}", local_time(details.updated))),
        row![text(i18n.__("变更行数")).size(11).style(settings_muted_text_style), changes]
            .spacing(9),
    ]
    .spacing(6);
    let status_label = match status {
        SessionRowStatus::Running => Some(i18n.__("运行中").to_owned()),
        SessionRowStatus::Queued(count) => i18n.format("排队 %u", &[count.into()]).ok(),
        SessionRowStatus::Updated => Some(i18n.__("有更新").to_owned()),
        SessionRowStatus::Idle => None,
    };
    if let Some(label) = status_label {
        content = content.push(detail_line(i18n.__("状态"), label));
    }
    container(content).width(320).padding(12).style(details_style).into()
}

pub(in crate::app::views::project) fn session_row(
    app: &App,
    id: &str,
    details: &SessionRowDetails,
    status: SessionRowStatus,
    open: Message,
) -> Element<'static, Message> {
    let i18n = I18n::new(app.desktop_locale);
    let active = app.active_session_id.as_deref() == Some(id);
    let title = if details.title.trim().is_empty() {
        i18n.__("未命名任务").to_owned()
    } else {
        crate::app::i18n::other::session_title(app, id, &details.title)
    };
    let key = (details.project_id.clone(), id.to_owned());
    let pinned = app.session_ui_projection.pinned.contains(&key);
    let action_width = 2.0 * ACTION_SIZE + ACTION_GAP;
    let mut content =
        row![].spacing(4).width(Length::Fill).height(Length::Fill).align_y(Alignment::Center);
    if pinned {
        content = content.push(
            svg(svg::Handle::from_memory(PINNED))
                .width(12)
                .height(12)
                .style(|theme: &Theme, _| svg::Style { color: Some(theme.palette().text) }),
        );
    }
    content = content
        .push(SingleLineLabel::new(title.clone(), 13.0).font(iced::Font {
            weight: if active { iced::font::Weight::Bold } else { iced::font::Weight::Medium },
            ..Default::default()
        }))
        .push(Space::new().width(action_width));
    let mut right_padding = ROW_PADDING;
    if status != SessionRowStatus::Idle {
        content = content.push(status_icon(id, status, app.status_animation_frame));
        right_padding += status.indicator_width() + 4.0;
    }
    let base = button(content)
        .width(Length::Fill)
        .height(30)
        .padding([0.0, ROW_PADDING])
        .on_press(open)
        .style(move |theme, status| session_row_button_style(theme, active, status));
    if app.session_menu_id.is_some() {
        return base.into();
    }
    let pin_message = Message::Project(ProjectMessage::SessionPinPressed(key.0, key.1, !pinned));
    let actions = row![
        action(
            if pinned { PINNED } else { PIN },
            i18n.__(if pinned { "取消置顶" } else { "置顶" }),
            pin_message
        ),
        action(
            ARCHIVE,
            i18n.__("归档"),
            Message::Project(ProjectMessage::SessionArchivePressed(id.to_owned()))
        ),
    ]
    .spacing(ACTION_GAP);
    let top = container(actions)
        .width(Length::Fill)
        .height(Length::Fill)
        .align_x(Alignment::End)
        .align_y(Alignment::Center)
        .padding(iced::Padding::default().right(right_padding));
    Tooltip::new(hover(base, top), details_card(i18n, title, details, status), Position::Right)
        .delay(Duration::from_millis(280))
        .padding(0)
        .gap(14)
        .into()
}
