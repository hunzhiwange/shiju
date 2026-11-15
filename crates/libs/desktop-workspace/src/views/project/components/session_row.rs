//! Shared session rows for the code and work sidebars.

use std::sync::LazyLock;
use std::time::Duration;

use chrono::{Local, TimeZone};
use iced::widget::tooltip::{Position, Tooltip};
use iced::widget::{Space, button, column, container, hover, mouse_area, row, stack, svg, text};
use iced::{Alignment, Background, Border, Color, Element, Length, Theme, border};
use session_info::{Info};

use desktop_ui_controls::system_settings_common::{
    settings_muted_text_style, settings_panel_style,
};
use desktop_ui_controls::single_line_label::SingleLineLabel;
use desktop_i18n::I18n;
use super::sessions_panel::events::ProjectMessage;
use super::sessions_panel::Message;

use super::super::row_styles::{is_dark_theme, session_row_highlight_color, tooltip_bubble};
use super::super::utils::mix_color;

const ACTION_SIZE: f32 = 22.0;
const ACTION_GAP: f32 = 2.0;
const ROW_PADDING: f32 = 8.0;
const STATUS_SIZE: f32 = 14.0;
const RUNNING_RING_FRAMES: usize = 16;

// Rotate the arc inside the SVG so its bounds stay fixed across frames.
static RUNNING_RING_ICONS: LazyLock<[svg::Handle; RUNNING_RING_FRAMES]> = LazyLock::new(|| {
    std::array::from_fn(|frame| {
        let angle = frame as f32 * 360.0 / RUNNING_RING_FRAMES as f32;
        svg::Handle::from_memory(
            format!(
                r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 16 16">
                  <g fill="none" stroke="black" stroke-width="1.75">
                    <circle cx="8" cy="8" r="6.25" opacity="0.23"/>
                    <circle cx="8" cy="8" r="6.25" stroke-linecap="round"
                      stroke-dasharray="22 40" transform="rotate({angle} 8 8)"/>
                  </g>
                </svg>"#,
            )
            .into_bytes(),
        )
    })
});
const PIN: &[u8] =
    include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../assets/icons/bootstrap/pin.svg"));
const PINNED: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../../assets/icons/bootstrap/pin-fill.svg"
));
const ARCHIVE: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../../assets/icons/bootstrap/archive.svg"
));

pub struct SessionRowDetails {
    pub title: String,
    pub directory: String,
    pub updated: u64,
    pub project: String,
    pub project_id: String,
    pub created: u64,
    pub changes: Option<(i64, i64)>,
    pub pending: bool,
}

impl SessionRowDetails {
    pub fn new(session: &Info, project: String) -> Self {
        Self {
            title: session.title.clone(),
            directory: session.directory.clone(),
            updated: session.time.updated,
            project,
            project_id: session.project_id.clone(),
            created: session.time.created,
            changes: session.summary.as_ref().map(|summary| (summary.additions, summary.deletions)),
            pending: session.is_first_response_pending(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionRowStatus {
    Idle,
    Running,
    Queued(usize),
    Updated,
}

impl SessionRowStatus {
    fn indicator_width(self) -> f32 {
        match self {
            Self::Idle => 0.0,
            Self::Running | Self::Queued(_) | Self::Updated => STATUS_SIZE,
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

fn running_indicator(frame: usize, breathe: f32) -> Element<'static, Message> {
    let ring = svg(RUNNING_RING_ICONS[frame % RUNNING_RING_FRAMES].clone())
        .width(STATUS_SIZE)
        .height(STATUS_SIZE)
        .style(|theme: &Theme, _| svg::Style {
            color: Some(if is_dark_theme(theme) {
                Color::from_rgb8(150, 150, 150)
            } else {
                Color::from_rgb8(133, 133, 133)
            }),
        });
    let diameter = 5.0 + 2.0 * breathe;
    let dot = container(Space::new()).width(diameter).height(diameter).style(move |_| {
        container::background(running_status_color(breathe)).border(border::rounded(diameter / 2.0))
    });
    stack![ring, container(dot).center(STATUS_SIZE)].into()
}

fn status_icon(id: &str, status: SessionRowStatus, frame: usize) -> Element<'static, Message> {
    let session_phase =
        id.bytes().fold(0u32, |acc, byte| acc.wrapping_mul(16777619).wrapping_add(byte as u32));
    let now = runtime_time::now_ms() as f64 / 1000.0;
    let phase =
        (now / 1.8 + f64::from(session_phase) / f64::from(u32::MAX)) * std::f64::consts::TAU;
    let breathe = ((phase.sin() + 1.0) * 0.5) as f32;
    let indicator: Element<'static, Message> = match status {
        SessionRowStatus::Running => running_indicator(frame, breathe),
        SessionRowStatus::Queued(_) => {
            text("●").size(8.5).color(queued_status_color(breathe)).into()
        }
        SessionRowStatus::Updated => {
            text("●").size(11).color(Color::from_rgb8(46, 184, 114)).into()
        }
        SessionRowStatus::Idle => Space::new().into(),
    };
    container(indicator).width(status.indicator_width()).align_x(Alignment::End).into()
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

pub fn agent_description(
    i18n: I18n,
    agent: Option<&str>,
    label: Option<&str>,
    acp: Option<&str>,
) -> String {
    let name = label.filter(|label| !label.trim().is_empty()).or(agent
        .filter(|agent| *agent != "main" && !agent.trim().is_empty()));
    let engine = match acp {
        Some("") => "ACP".to_owned(),
        Some(backend) => format!("ACP · {backend}"),
        None => i18n.__("识矩原生").to_owned(),
    };
    name.map_or_else(|| engine.clone(), |name| format!("{name} · {engine}"))
}

fn details_card(
    i18n: I18n,
    workspace_kind: Option<bool>,
    agent: String,
    title: String,
    details: &SessionRowDetails,
    status: SessionRowStatus,
    relative: &str,
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
    let relative = relative.to_owned();
    let workspace = match workspace_kind {
        Some(true) => format!("{} · {}", i18n.__("主工作区"), details.directory),
        Some(false) => format!("Git worktree · {}", details.directory),
        None => details.directory.clone(),
    };
    let mut content = column![
        text(title)
            .size(14)
            .font(iced::Font { weight: iced::font::Weight::Semibold, ..Default::default() })
            .width(Length::Fill)
            .wrapping(text::Wrapping::WordOrGlyph),
        detail_line(i18n.__("项目"), details.project.clone()),
        detail_line(i18n.__("工作区"), workspace),
        detail_line(i18n.__("智能体"), agent),
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

pub fn session_row(
    input: &Input,
    id: &str,
    details: &SessionRowDetails,
    status: SessionRowStatus,
    open: Message,
) -> Element<'static, Message> {
    let i18n = I18n::new(input.desktop_locale);
    let active = input.active;
    let title = input.title.clone();
    let key = (details.project_id.clone(), id.to_owned());
    let pinned = !details.pending && input.pinned;
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
    content = content.push(SingleLineLabel::new(title.clone(), 13.0).font(iced::Font {
        weight: if active { iced::font::Weight::Bold } else { iced::font::Weight::Medium },
        ..Default::default()
    }));
    if !details.pending {
        content = content.push(Space::new().width(2.0 * ACTION_SIZE + ACTION_GAP));
    }
    let mut right_padding = ROW_PADDING;
    if status != SessionRowStatus::Idle {
        content = content.push(status_icon(id, status, input.status_animation_frame));
        right_padding += status.indicator_width() + 4.0;
    }
    let base = button(content)
        .width(Length::Fill)
        .height(30)
        .padding([0.0, ROW_PADDING])
        .on_press(open)
        .style(move |theme, status| session_row_button_style(theme, active, status));
    if details.pending || input.menu_open {
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
    let workspace_kind = input.workspace_kind;
    let agent = input.agent.clone();
    let tooltip = Tooltip::new(
        hover(base, top),
        details_card(i18n, workspace_kind, agent, title, details, status, &input.relative),
        Position::Right,
    )
    .delay(Duration::from_millis(280))
    .padding(0)
    .gap(14);
    mouse_area(tooltip)
        .on_enter(Message::SessionDetailsHovered {
            session_id: id.to_owned(),
            project_id: details.project_id.clone(),
            directory: details.directory.clone(),
        })
        .into()
}



#[derive(Clone)]
pub struct Input {
 pub desktop_locale: config_ui::DesktopLocale,
 pub active: bool,
 pub title: String,
 pub pinned: bool,
 pub status_animation_frame: usize,
 pub menu_open: bool,
 pub workspace_kind: Option<bool>,
 pub agent: String,
 pub relative: String,
}

impl SessionRowDetails { pub fn title(&self) -> &str { &self.title } pub fn project_id(&self) -> &str { &self.project_id } }

#[cfg(test)]
#[path = "session_row_tests.rs"]
mod session_row_tests;

impl SessionRowStatus {
 pub fn from_activity(requesting: bool, queued: usize, unseen: bool) -> Self {
  match (requesting, queued, unseen) {
   (true, _, _) => Self::Running,
   (_, count, _) if count > 0 => Self::Queued(count),
   (_, _, true) => Self::Updated,
   _ => Self::Idle,
  }
 }
}

pub fn running_draft_session(id: &str, directory: &str, query: Option<&str>, started: u64, locale: config_ui::DesktopLocale) -> Info {
        let title = query.and_then(session_text::initial_title_from_content)
            .map(|line| super::super::utils::truncate_display_width(line, 120))
            .unwrap_or_else(|| I18n::new(locale).__("新会话").to_owned());
        let mut info = Info {
            id: id.to_owned(),
            slug: String::new(),
            project_id: String::new(),
            directory: directory.to_owned(),
            parent_id: None,
            summary: None,
            share: None,
            title,
            version: String::new(),
            time: session_info::TimeInfo { created: started, updated: started, compacting: None, archived: None },
            permission: None,
            fork_execution_started: None,
            revert: None,
        };
        info.mark_first_response_pending();

 info
}
