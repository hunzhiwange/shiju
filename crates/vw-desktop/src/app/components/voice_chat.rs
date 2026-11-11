//! Conversation overlay uses the shared modal layout and theme palette.
use super::system_settings_common::{
    primary_action_btn_style, rounded_action_btn_style, settings_close_button,
    settings_modal_header, settings_modal_layout, settings_modal_overlay_with_escape,
};
use crate::app::{
    App, Message,
    message::voice::Event,
    voice::conversation::{Phase, Snapshot},
};
use iced::{
    Element, Length,
    widget::{button, column, container, row, text},
};

fn message(event: Event) -> Message {
    Message::Voice(event)
}
fn phase_label(phase: Phase) -> &'static str {
    match phase {
        Phase::Connecting => "正在连接",
        Phase::Listening => "正在聆听",
        Phase::Speaking => "正在回答",
        Phase::Muted => "麦克风已静音",
        Phase::Ending => "通话已结束",
        Phase::Error => "通话发生错误",
    }
}
pub(crate) fn with_voice_chat<'a>(
    app: &'a App,
    base: Element<'a, Message>,
) -> Element<'a, Message> {
    let Some(chat) = &app.voice.conversation else {
        return base;
    };
    overlay(base, chat.snapshot())
}
fn overlay<'a>(base: Element<'a, Message>, state: Snapshot) -> Element<'a, Message> {
    let close = message(Event::ChatEnd);
    let header =
        settings_modal_header(text("语音聊天").size(20), settings_close_button(close.clone()));
    let mut body = column![
        text(phase_label(state.phase)).size(18),
        text(format!(
            "麦克风：{} · 远端发送：{} · 播放：{}",
            if state.microphone { "采集中" } else { "已停止" },
            if state.sending { "开启" } else { "暂停" },
            if state.playing { "开启" } else { "停止" }
        ))
        .size(13),
        text("扬声器回答期间暂停上传麦克风；点击打断或按 Command+Shift+Space 后继续说话。")
            .size(13),
    ]
    .spacing(12);
    if let Some(error) = state.error {
        body = body.push(text(error).size(14));
    }
    for caption in state.captions {
        let role = match caption.role {
            vw_gateway_client::vw_api_types::voice::TranscriptRole::User => "你",
            vw_gateway_client::vw_api_types::voice::TranscriptRole::Assistant => "ShiJu",
        };
        body = body.push(column![text(role).size(12), text(caption.text).size(15)].spacing(4));
    }
    let active = !matches!(state.phase, Phase::Error | Phase::Ending | Phase::Connecting);
    let footer = row![
        button(text(if state.phase == Phase::Muted { "取消静音" } else { "静音" }))
            .padding([8, 14])
            .style(rounded_action_btn_style)
            .on_press_maybe(active.then(|| message(Event::ChatMute))),
        button(text("打断回答")).padding([8, 14]).style(rounded_action_btn_style).on_press_maybe(
            (active && (state.playing || state.phase == Phase::Speaking))
                .then(|| message(Event::ChatInterrupt))
        ),
        button(text("结束通话"))
            .padding([8, 14])
            .style(primary_action_btn_style)
            .on_press(close.clone()),
    ]
    .spacing(8);
    let card = settings_modal_layout(header, body, Some(footer.into()))
        .width(Length::Fill)
        .max_width(640)
        .height(Length::Fill)
        .max_height(560);
    settings_modal_overlay_with_escape(
        base,
        close,
        container(card).padding(12).max_width(664).max_height(584),
    )
}
#[cfg(test)]
#[path = "voice_chat_tests.rs"]
mod tests;
