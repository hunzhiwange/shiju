//! Companion dock, composer and compact voice controls.
use desktop_assets::assets::{self, Icon};
use desktop_i18n::I18n;
use desktop_ui_controls::overlays::BelowOverlay;
use desktop_ui_controls::system_settings_common::theme::is_dark_theme;
use desktop_ui_controls::system_settings_common::{
    SETTINGS_CONTROL_PADDING, SETTINGS_CONTROL_TEXT_SIZE, SettingsSelectLabel,
    primary_action_btn_style, rounded_action_btn_style, settings_muted_text_style,
    settings_panel_style, settings_text_input, settings_text_input_style,
};
use iced::widget::scrollable::{Direction, Scrollbar};
use iced::widget::{scrollable, stack, text_input, tooltip};
use iced::{Degrees, Gradient, Shadow, Vector, gradient};

use super::*;
use crate::pet_view::{ACTIONS_WIDTH, DOCK_HEIGHT, DockMode, PetVoice, VoiceOrbAction};

pub fn composer_id() -> iced::widget::Id {
    iced::widget::Id::new("task-pet-composer")
}

pub fn project_search_id() -> iced::widget::Id {
    iced::widget::Id::new("task-pet-project-search")
}

pub(super) fn toolbar<'a>(app: PetView<'a>) -> Element<'a, Message> {
    let i18n = I18n::new(app.desktop_locale);
    let dock = app.dock;
    let size = dock.mode.size();
    let notice = !app.task_pet_items.is_empty();
    let morphing = dock.width != size.width || dock.height != size.height;
    let split = !morphing
        && matches!(dock.mode, DockMode::Actions | DockMode::VoiceConnecting | DockMode::Voice);
    let content = match dock.mode {
        // Mount the composer immediately so the focus operation can find it.
        DockMode::Composer => composer(i18n, app),
        DockMode::Voice if !morphing => voice_controls(app, i18n),
        DockMode::Idle if dock.height <= 22.0 => Space::new().into(),
        _ => actions(app, i18n, split),
    };
    let pill = container(content)
        .width(dock.width)
        .height(dock.height)
        .center_x(dock.width)
        .center_y(dock.height)
        .clip(true)
        .style(move |theme: &Theme| {
            if split {
                container::Style::default()
            } else if notice && dock.height <= 13.0 {
                surface(theme, dock.height / 2.0, true)
            } else {
                surface(theme, dock.height / 2.0, false)
            }
        });
    let pill: Element<'a, Message> = if dock.mode == DockMode::Idle {
        mouse_area(pill).on_press(Message::TaskPetDockHover(true)).into()
    } else {
        pill.into()
    };
    container(pill).center_x(Length::Fill).center_y(DOCK_HEIGHT).into()
}

fn actions<'a>(app: PetView<'a>, i18n: I18n, split: bool) -> Element<'a, Message> {
    let dock = app.dock;
    let width = (dock.width.min(ACTIONS_WIDTH) - 4.0) / 3.0;
    let opacity = ((dock.height - 20.0) / 24.0).clamp(0.0, 1.0);
    let middle_width = if split { DOCK_HEIGHT } else { width };
    let connecting = matches!(dock.mode, DockMode::VoiceConnecting | DockMode::Voice);
    let middle = if connecting {
        let stop = if dock
            .voice
            .is_some_and(|voice| matches!(voice.orb_action, Some(VoiceOrbAction::Interrupt)))
        {
            VoiceOrbAction::Interrupt
        } else {
            VoiceOrbAction::End
        };
        action(
            voice_dot(opacity),
            i18n.__("正在连接"),
            Some(Message::TaskPetVoiceOrb(stop)),
            middle_width,
        )
    } else {
        action(
            line_icon(&WAVE, 18.0, opacity),
            i18n.__("语音聊天"),
            Some(Message::TaskPetStartVoice),
            middle_width,
        )
    };
    if split {
        return row![
            capsule(new_chat_button(i18n, DOCK_HEIGHT, opacity)),
            capsule(middle),
            capsule(reminders(app, i18n, DOCK_HEIGHT, opacity)),
        ]
        .spacing((ACTIONS_WIDTH - 3.0 * DOCK_HEIGHT) / 2.0)
        .align_y(Alignment::Center)
        .into();
    }
    row![
        new_chat_button(i18n, width, opacity),
        divider(),
        middle,
        divider(),
        reminders(app, i18n, width, opacity),
    ]
    .spacing(1)
    .align_y(Alignment::Center)
    .into()
}

fn new_chat_button(i18n: I18n, size: f32, opacity: f32) -> Element<'static, Message> {
    action(
        icon(Icon::Pencil, 16.0, opacity),
        i18n.__("新建聊天"),
        Some(Message::TaskPetWriteMessage),
        size,
    )
}

fn voice_dot(opacity: f32) -> Element<'static, Message> {
    container(Space::new())
        .width(10)
        .height(10)
        .style(move |theme: &Theme| {
            container::Style::default()
                .background(theme.palette().text.scale_alpha(opacity))
                .border(Border::default().rounded(5))
        })
        .into()
}

fn composer<'a>(i18n: I18n, app: PetView<'a>) -> Element<'a, Message> {
    let draft = app.dock.draft;
    let input = text_input(i18n.__("开始新聊天"), draft)
        .id(composer_id())
        .on_input(Message::TaskPetComposerChanged)
        .on_submit_maybe((!draft.trim().is_empty()).then_some(Message::TaskPetComposerSubmit))
        .width(Length::Fill)
        .padding(SETTINGS_CONTROL_PADDING)
        .size(SETTINGS_CONTROL_TEXT_SIZE)
        .style(|theme, status| {
            let mut style = settings_text_input_style(theme, status);
            style.background = Background::Color(Color::TRANSPARENT);
            style.border = Border::default();
            style
        });
    let send = button(centered(glyph(Icon::ArrowUp, 14.0)))
        .on_press_maybe((!draft.trim().is_empty()).then_some(Message::TaskPetComposerSubmit))
        .width(28)
        .height(28)
        .padding(0)
        .style(|theme, status| {
            let mut style = primary_action_btn_style(theme, status);
            style.border.radius = 14.0.into();
            style
        });
    let project = project_select(i18n, app);
    container(row![project, input, send].align_y(Alignment::Center).spacing(4))
        .padding([0, 8])
        .width(Length::Fill)
        .into()
}

fn project_select<'a>(i18n: I18n, app: PetView<'a>) -> Element<'a, Message> {
    let name_for = |path: &str| {
        app.project_paths
            .iter()
            .position(|candidate| candidate == path)
            .and_then(|index| app.project_labels.get(index))
            .filter(|label| !label.is_empty())
            .cloned()
            .unwrap_or_else(|| {
                std::path::Path::new(path).file_name().and_then(|name| name.to_str())
                    .unwrap_or(path).to_owned()
            })
    };
    let label = app.dock.project_path.map(name_for)
        .unwrap_or_else(|| i18n.__("选择项目").to_owned());
    let trigger = button(
        row![SettingsSelectLabel::new(label), glyph(Icon::ChevronDown, 10.0)]
            .spacing(4)
            .align_y(Alignment::Center),
    )
    .on_press(Message::TaskPetProjectMenuToggle)
    .width(112)
    .height(30)
    .padding([0, 7])
    .style(|theme, status| {
        let mut style = rounded_action_btn_style(theme, status);
        style.border.radius = 15.0.into();
        style
    });
    let Some(query) = app.dock.project_query else {
        return trigger.into();
    };
    let search = settings_text_input(i18n.__("搜索项目"), query)
        .id(project_search_id())
        .on_input(Message::TaskPetProjectQueryChanged)
        .width(Length::Fill);
    let query = query.trim().to_lowercase();
    let selected = app.dock.project_path;
    let current_unlisted = selected.filter(|path| !app.project_paths.iter().any(|p| p == *path));
    let mut choices: Vec<Element<'a, Message>> = Vec::new();
    for path in current_unlisted.into_iter().chain(app.project_paths.iter().map(String::as_str)) {
        let label = name_for(path);
        if !label.to_lowercase().contains(&query) && !path.to_lowercase().contains(&query) {
            continue;
        }
        let is_selected = selected == Some(path);
        let choice = button(
            column![
                row![
                    SettingsSelectLabel::new(label),
                    if is_selected { glyph(Icon::Check, 12.0) } else { Space::new().width(12).into() },
                ]
                .spacing(6)
                .align_y(Alignment::Center),
                text(path).size(11).wrapping(iced::widget::text::Wrapping::None)
                    .style(settings_muted_text_style),
            ]
            .spacing(2),
        )
        .width(Length::Fill)
        .padding([5, 8])
        .clip(true)
        .style(rounded_action_btn_style)
        .on_press(Message::TaskPetProjectSelected(path.to_owned()));
        choices.push(choice.into());
    }
    if choices.is_empty() {
        let label = if query.is_empty() { "暂无项目" } else { "未找到匹配的项目" };
        choices.push(
            container(text(i18n.__(label)).size(12).style(settings_muted_text_style))
                .padding([6, 8])
                .into(),
        );
    }
    let menu = container(column![
        search,
        scrollable(iced::widget::Column::with_children(choices).spacing(4))
            .width(Length::Fill)
            .height(Length::Shrink)
            .direction(Direction::Vertical(Scrollbar::new().width(4).scroller_width(4).spacing(3))),
    ]
    .spacing(6))
        .width(280)
        .max_height(210)
        .padding(7)
        .style(settings_panel_style);
    BelowOverlay::new(trigger, menu)
        .show(true)
        .gap(6.0)
        .within_window()
        .dismiss_on_escape()
        .on_close(Message::TaskPetProjectMenuClose)
        .into()
}

fn voice_controls<'a>(app: PetView<'a>, i18n: I18n) -> Element<'a, Message> {
    let Some(voice) = app.dock.voice else {
        return Space::new().into();
    };
    let record_label = match voice.orb_action {
        Some(VoiceOrbAction::Begin) => "点击开始录音",
        Some(VoiceOrbAction::Finish) => "点击停止并发送",
        Some(VoiceOrbAction::Interrupt) => "打断本轮",
        Some(VoiceOrbAction::End) => "结束语音聊天",
        None => "语音聊天",
    };
    let mic = action(
        line_icon(if voice.muted { &MIC_OFF } else { &MIC }, 17.0, 1.0),
        i18n.__(if voice.muted { "取消静音" } else { "静音" }),
        voice.microphone_enabled.then_some(Message::TaskPetVoiceMic),
        34.0,
    );
    let speaker = action(
        line_icon(&SPEAKER, 18.0, 1.0),
        i18n.__("打断回答"),
        voice.can_interrupt.then_some(Message::TaskPetVoiceOrb(VoiceOrbAction::Interrupt)),
        34.0,
    );
    let orb = action(
        voice_orb(voice),
        i18n.__(record_label),
        voice.orb_action.map(Message::TaskPetVoiceOrb),
        42.0,
    );
    let end = action(
        glyph(Icon::X, 14.0),
        i18n.__("结束语音聊天"),
        Some(Message::TaskPetVoiceOrb(VoiceOrbAction::End)),
        28.0,
    );
    row![
        capsule(new_chat_button(i18n, DOCK_HEIGHT, 1.0)),
        capsule(row![mic, divider(), orb, divider(), speaker, end].align_y(Alignment::Center)),
        capsule(reminders(app, i18n, DOCK_HEIGHT, 1.0)),
    ]
    .spacing(8)
    .align_y(Alignment::Center)
    .into()
}

fn reminders(app: PetView<'_>, i18n: I18n, width: f32, opacity: f32) -> Element<'static, Message> {
    let count = app.task_pet_items.len();
    let open = app.dock.reminders_open;
    let icon =
        if open { icon(Icon::ChevronUp, 16.0, opacity) } else { line_icon(&BELL, 18.0, opacity) };
    let content = if count > 0 && !open && opacity > 0.75 {
        let badge =
            container(text(if count > 99 { "99+".into() } else { count.to_string() }).size(9))
                .padding([0, 3])
                .center_y(13)
                .align_x(Alignment::Center)
                .style(move |_| {
                    container::Style::default()
                        .background(Color::from_rgb8(22, 167, 100).scale_alpha(opacity))
                        .color(Color::WHITE.scale_alpha(opacity))
                        .border(Border::default().rounded(7))
                });
        stack![
            centered(icon),
            container(badge)
                .align_x(Alignment::End)
                .width(Length::Fill)
                .padding(iced::Padding::default().top(2).right(2)),
        ]
        .into()
    } else {
        centered(icon)
    };
    action(
        content,
        i18n.__(if open { "收起" } else { "会话提醒" }),
        (count > 0 || open).then_some(Message::TaskPetToggleReminders),
        width,
    )
}

fn glyph(symbol: Icon, size: f32) -> Element<'static, Message> {
    icon(symbol, size, 1.0)
}

fn icon(symbol: Icon, size: f32, opacity: f32) -> Element<'static, Message> {
    vector(assets::get_icon(symbol), size, opacity)
}

fn action(
    icon: Element<'static, Message>,
    label: &'static str,
    intent: Option<Message>,
    width: f32,
) -> Element<'static, Message> {
    let height = width.min(DOCK_HEIGHT);
    tooltip(
        button(centered(icon))
            .on_press_maybe(intent)
            .width(width)
            .height(height)
            .padding(0)
            .style(move |theme: &Theme, status| {
                let mut style = rounded_action_btn_style(theme, status);
                style.border = Border::default().rounded(height / 2.0);
                style.shadow = Shadow::default();
                style.background = match status {
                    button::Status::Hovered => Some(theme.palette().text.scale_alpha(0.10).into()),
                    button::Status::Pressed => Some(theme.palette().text.scale_alpha(0.16).into()),
                    _ => None,
                };
                style
            }),
        text(label).size(11),
        tooltip::Position::Top,
    )
    .style(container::rounded_box)
    .delay(iced::time::Duration::from_millis(450))
    .padding(5)
    .into()
}

fn centered(content: Element<'static, Message>) -> Element<'static, Message> {
    container(content).center_x(Length::Fill).center_y(Length::Fill).into()
}

fn divider() -> Element<'static, Message> {
    container(Space::new())
        .width(1)
        .height(18)
        .style(|theme: &Theme| {
            container::Style::default().background(theme.palette().text.scale_alpha(0.13))
        })
        .into()
}

fn capsule<'a>(content: impl Into<Element<'a, Message>>) -> Element<'a, Message> {
    container(content).center_y(DOCK_HEIGHT).style(|theme| surface(theme, DOCK_HEIGHT / 2.0, false)).into()
}

pub(super) fn surface(theme: &Theme, radius: f32, notice: bool) -> container::Style {
    let dark = is_dark_theme(theme);
    let (top, bottom, edge) = if notice {
        (
            Color::from_rgb8(63, 224, 143),
            Color::from_rgb8(14, 158, 87),
            Color::from_rgb8(94, 235, 160),
        )
    } else if dark {
        (
            Color::from_rgba8(57, 65, 81, 0.98),
            Color::from_rgba8(32, 38, 50, 0.98),
            Color::from_rgba8(147, 166, 199, 0.32),
        )
    } else {
        (
            Color::from_rgba8(253, 255, 255, 0.99),
            Color::from_rgba8(221, 232, 248, 0.98),
            Color::from_rgba8(106, 129, 160, 0.46),
        )
    };
    container::Style {
        background: Some(Background::Gradient(Gradient::Linear(
            gradient::Linear::new(Degrees(180.0)).add_stop(0.0, top).add_stop(1.0, bottom),
        ))),
        text_color: Some(theme.palette().text),
        border: Border { color: edge, width: 1.0, radius: radius.into() },
        shadow: Shadow {
            color: Color::BLACK.scale_alpha(if dark { 0.28 } else { 0.15 }),
            offset: Vector::new(0.0, 2.0),
            blur_radius: 5.0,
        },
        ..Default::default()
    }
}

fn voice_orb(voice: PetVoice) -> Element<'static, Message> {
    let (size, top, bottom) = if voice.connecting {
        (11.0, Color::WHITE, Color::from_rgb8(195, 216, 251))
    } else {
        (29.0, Color::from_rgb8(56, 111, 236), Color::from_rgb8(199, 234, 255))
    };
    container(Space::new())
        .width(size)
        .height(size)
        .style(move |_| {
            container::Style::default()
                .background(Gradient::Linear(
                    gradient::Linear::new(Degrees(140.0)).add_stop(0.0, top).add_stop(1.0, bottom),
                ))
                .border(Border::default().rounded(size / 2.0))
        })
        .into()
}

type LineIcon = Lazy<svg::Handle>;
static WAVE: LineIcon = Lazy::new(|| stroke("<path d='M3 10v4m4-7v10m5-13v16m5-13v10m4-7v4'/>"));
static BELL: LineIcon =
    Lazy::new(|| stroke("<path d='M5 10a7 7 0 0 1 14 0v4l2 3H3l2-3zm4 10a3 3 0 0 0 6 0'/>"));
static MIC: LineIcon = Lazy::new(|| {
    stroke(
        "<rect x='9' y='3' width='6' height='12' rx='3'/><path d='M5 10v2a7 7 0 0 0 14 0v-2m-7 9v3'/>",
    )
});
static MIC_OFF: LineIcon = Lazy::new(|| {
    stroke(
        "<path d='M9 9v3a3 3 0 0 0 4 3m2-5V6a3 3 0 0 0-5-2m-5 7v1a7 7 0 0 0 12 5m2-5v-1m-7 8v3M3 3l18 18'/>",
    )
});
static SPEAKER: LineIcon = Lazy::new(|| {
    stroke("<path d='M3 9h4l5-4v14l-5-4H3zm13-1a6 6 0 0 1 0 8m3-11a10 10 0 0 1 0 14'/>")
});

pub(super) fn stroke(body: &str) -> svg::Handle {
    svg::Handle::from_memory(format!(
        "<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 24 24'><g fill='none' stroke='white' stroke-width='1.8' stroke-linecap='round' stroke-linejoin='round'>{body}</g></svg>"
    ).into_bytes())
}

fn line_icon(icon: &LineIcon, size: f32, opacity: f32) -> Element<'static, Message> {
    vector((**icon).clone(), size, opacity)
}

pub(super) fn vector(handle: svg::Handle, size: f32, opacity: f32) -> Element<'static, Message> {
    svg::Svg::new(handle)
        .width(size)
        .height(size)
        .opacity(opacity)
        .style(|theme: &Theme, _| svg::Style { color: Some(theme.palette().text) })
        .into()
}
