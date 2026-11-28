//! Conversation overlay uses the shared modal layout and theme palette.
use desktop_i18n::I18n;
use desktop_ui_controls::system_settings_common::{
    primary_action_btn_style, round_icon_btn_style, rounded_action_btn_style,
    settings_close_button, settings_modal_header, settings_modal_layout,
    settings_modal_overlay_with_escape,
};
use desktop_ui_controls::tooltip::tooltip_dark_style;
use desktop_voice::conversation::{Phase, Snapshot};
use iced::{
    Alignment, Element, Length,
    widget::{button, column, container, row, stack, svg, text, tooltip},
};
use std::time::Duration;

#[path = "voice_chat_screen.rs"]
mod screen_ui;

const MIC_ICON: &[u8] =
    include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../assets/icons/feather/mic.svg"));
const MIC_OFF_ICON: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../../assets/icons/feather/mic-off.svg"
));
const INTERRUPT_ICON: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../../assets/icons/feather/stop-circle.svg"
));
const CLOSE_ICON: &[u8] =
    include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../assets/icons/feather/x.svg"));
const RECORDING_ICON: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../../assets/icons/bootstrap/square-fill.svg"
));

const CONTROL_SIZE: f32 = 32.0;

pub fn phase_label(phase: Phase) -> &'static str {
    match phase {
        Phase::Connecting => "正在连接",
        Phase::Listening => "正在聆听",
        Phase::Speaking => "正在回答",
        Phase::Muted => "麦克风已静音",
        Phase::Ending => "通话已结束",
        Phase::Error => "通话发生错误",
    }
}
pub fn overlay_screen<'a, Message: Clone + 'static>(
    i18n: I18n,
    base: Element<'a, Message>,
    map: fn(Event) -> Message,
    state: Snapshot,
    screen: Option<&'a desktop_voice::screen_context::State>,
) -> Element<'a, Message> {
    let close = map(Event::ChatEnd);
    let header = settings_modal_header(
        text(i18n.__("语音聊天")).size(20),
        settings_close_button(close.clone()),
    );
    let mut body = column![
        text(i18n.__(phase_label(state.phase))).size(18),
        text(
            i18n.format(
                "麦克风：%s · 远端发送：%s · 播放：%s",
                &[
                    i18n.__(if state.microphone { "采集中" } else { "已停止" }).into(),
                    i18n.__(if state.sending { "开启" } else { "暂停" }).into(),
                    i18n.__(if state.playing { "开启" } else { "停止" }).into()
                ]
            )
            .expect("valid voice status")
        )
        .size(13),
        text(
            i18n.__("扬声器回答期间暂停上传麦克风；点击打断或按 Command+Shift+Space 后继续说话。")
        )
        .size(13),
    ]
    .spacing(12);
    if let Some(error) = state.error {
        body = body.push(text(crate::feedback::render(i18n, error)).size(14));
    }
    for caption in state.captions {
        let role = match caption.role {
            vw_api_voice::TranscriptRole::User => "你",
            vw_api_voice::TranscriptRole::Assistant => "ShiJu",
        };
        body = body
            .push(column![text(i18n.__(role)).size(12), text(caption.text).size(15)].spacing(4));
    }
    let active = !matches!(state.phase, Phase::Error | Phase::Ending | Phase::Connecting);
    let mut footer = row![
        button(text(i18n.__(if state.phase == Phase::Muted { "取消静音" } else { "静音" })))
            .padding([8, 14])
            .style(rounded_action_btn_style)
            .on_press_maybe(active.then(|| map(Event::ChatMute))),
        button(text(i18n.__("打断回答")))
            .padding([8, 14])
            .style(rounded_action_btn_style)
            .on_press_maybe(
                (active && (state.playing || state.phase == Phase::Speaking))
                    .then(|| map(Event::ChatInterrupt))
            ),
        button(text(i18n.__("结束通话")))
            .padding([8, 14])
            .style(primary_action_btn_style)
            .on_press(close.clone()),
    ]
    .spacing(8)
    .align_y(Alignment::Center);
    if let Some(screen) = screen {
        footer = footer.push(screen_ui::settings_control(i18n, map, screen, active));
        if let Some(activity) = screen_ui::activity(i18n, map, screen) {
            body = body.push(activity);
        }
    }
    let card = settings_modal_layout(header, body, Some(footer.into()))
        .width(Length::Fill)
        .max_width(640)
        .height(Length::Fill)
        .max_height(760);
    settings_modal_overlay_with_escape(
        base,
        close,
        container(card).padding(12).max_width(664).max_height(784),
    )
}
fn voice_tooltip<'a, Message: 'a>(
    control: impl Into<Element<'a, Message>>,
    label: &'a str,
    position: tooltip::Position,
) -> Element<'a, Message> {
    tooltip(
        control,
        container(text(label).size(12).wrapping(text::Wrapping::WordOrGlyph)).max_width(260),
        position,
    )
    .gap(8)
    .padding(8)
    .delay(Duration::from_millis(280))
    .style(tooltip_dark_style)
    .into()
}

fn voice_icon(bytes: &'static [u8], size: f32) -> svg::Svg<'static> {
    svg(svg::Handle::from_memory(bytes))
        .width(size)
        .height(size)
        .style(|theme: &iced::Theme, _| svg::Style { color: Some(theme.palette().text) })
}

fn voice_action(
    id: &'static str,
    bytes: &'static [u8],
    label: &'static str,
    event: Event,
) -> Element<'static, Event> {
    let icon = container(voice_icon(bytes, 17.0)).center(Length::Fill).id(id);
    voice_tooltip(
        button(icon)
            .width(CONTROL_SIZE)
            .height(CONTROL_SIZE)
            .padding(0)
            .style(round_icon_btn_style)
            .on_press(event),
        label,
        tooltip::Position::Bottom,
    )
}

/// Inline local controls leave chat approvals and questions unobstructed.
pub fn local_inline<'a>(
    i18n: I18n,
    chat: LocalView<'a>,
    screen: &desktop_voice::screen_context::State,
) -> Element<'a, Event> {
    let map = std::convert::identity::<Event>;
    let label = if chat.muted {
        "麦克风已静音"
    } else {
        match chat.stage {
            Stage::Ready => "点击开始录音",
            Stage::Preparing => "正在准备，可取消",
            Stage::Recording => "点击停止并发送",
            Stage::Recognizing => "正在本机识别",
            Stage::WaitingModel => "正在等待回复",
            Stage::Speaking => "正在朗读",
        }
    };
    let action = match chat.stage {
        Stage::Ready if !chat.muted => Some(Event::Local(LocalEvent::Begin)),
        Stage::Recording => Some(Event::Local(LocalEvent::Finish)),
        Stage::Preparing => Some(Event::ChatInterrupt),
        _ => None,
    };
    let mut orb = super::voice_moon::view(chat.muted);
    if chat.stage == Stage::Recording {
        orb = stack![
            orb,
            container(
                voice_icon(RECORDING_ICON, 15.0)
                    .style(|_, _| svg::Style { color: Some(iced::Color::from_rgb8(17, 26, 51)) })
            )
            .center(Length::Fill),
        ]
        .into();
    }
    let moon = voice_tooltip(
        button(orb).padding(0).style(|_, _| button::Style::default()).on_press_maybe(action),
        i18n.__(label),
        tooltip::Position::Right,
    );
    let controls = row![
        voice_action(
            "voice-chat-mute",
            if chat.muted { MIC_OFF_ICON } else { MIC_ICON },
            i18n.__(if chat.muted { "取消静音" } else { "静音" }),
            Event::ChatMute,
        ),
        voice_action(
            "voice-chat-interrupt",
            INTERRUPT_ICON,
            i18n.__("打断本轮"),
            Event::ChatInterrupt,
        ),
        voice_action("voice-chat-end", CLOSE_ICON, i18n.__("结束语音聊天"), Event::ChatEnd),
        screen_ui::settings_control(i18n, map, screen, chat.stage == Stage::Ready),
    ]
    .spacing(8)
    .align_y(Alignment::Center);
    let mut body =
        column![moon, controls].spacing(8).align_x(Alignment::Center).width(Length::Fill);
    if let Some(notice) = chat.notice {
        body = body.push(text(crate::feedback::render(i18n, notice)).size(12));
    }
    if let Some(activity) = screen_ui::activity(i18n, map, screen) {
        body = body.push(activity);
    }
    container(body)
        .width(Length::Fill)
        .padding(iced::Padding { top: 0.0, right: 7.0, bottom: 16.0, left: 7.0 })
        .into()
}

#[derive(Clone)]
pub enum Event {
    ChatEnd,
    ChatMute,
    ChatInterrupt,
    Local(LocalEvent),
    Screen(ScreenEvent),
}
#[derive(Clone)]
pub enum LocalEvent {
    Begin,
    Finish,
}
#[derive(Clone)]
pub enum ScreenEvent {
    Enable(bool),
    Permission,
    Offer,
    Cancel,
    Confirm,
    AttachmentChat,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Stage {
    Ready,
    Preparing,
    Recording,
    Recognizing,
    WaitingModel,
    Speaking,
}
pub struct LocalView<'a> {
    pub stage: Stage,
    pub muted: bool,
    pub notice: Option<&'a str>,
}

#[cfg(test)]
#[path = "voice_chat_tests.rs"]
mod tests;

use crate::message::voice::Event as VoiceEvent;
use crate::message::voice_screen::ScreenEvent as VoiceScreenEvent;
pub fn map(event: self::Event) -> crate::message::voice::Event {
    use self::{Event as Ui, LocalEvent as Local, ScreenEvent as Screen};
    match event {
        Ui::ChatEnd => VoiceEvent::ChatEnd,
        Ui::ChatMute => VoiceEvent::ChatMute,
        Ui::ChatInterrupt => VoiceEvent::ChatInterrupt,
        Ui::Local(event) => VoiceEvent::Local(match event {
            Local::Begin => crate::message::voice_local_chat::Event::Begin,
            Local::Finish => crate::message::voice_local_chat::Event::Finish,
        }),
        Ui::Screen(event) => VoiceEvent::Screen(match event {
            Screen::Enable(value) => VoiceScreenEvent::Enable(value),
            Screen::Permission => VoiceScreenEvent::Permission,
            Screen::Offer => VoiceScreenEvent::Offer,
            Screen::Cancel => VoiceScreenEvent::Cancel,
            Screen::Confirm => VoiceScreenEvent::Confirm,
            Screen::AttachmentChat => VoiceScreenEvent::AttachmentChat,
        }),
    }
}
