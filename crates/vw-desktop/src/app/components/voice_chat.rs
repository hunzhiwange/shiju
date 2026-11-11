//! Conversation overlay uses the shared modal layout and theme palette.
use super::system_settings_common::{
    primary_action_btn_style, rounded_action_btn_style, settings_checkbox_style,
    settings_close_button, settings_modal_header, settings_modal_layout,
    settings_modal_overlay_with_escape,
};
use crate::app::{
    App, Message,
    message::voice::{Event, ScreenEvent},
    voice::conversation::{Phase, Snapshot},
};
use iced::{
    Element, Length,
    widget::{button, checkbox, column, container, row, text},
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
    overlay_screen(base, chat.snapshot(), Some(&app.voice.screen))
}
#[cfg(test)]
fn overlay<'a>(base: Element<'a, Message>, state: Snapshot) -> Element<'a, Message> {
    overlay_screen(base, state, None)
}
fn overlay_screen<'a>(
    base: Element<'a, Message>,
    state: Snapshot,
    screen: Option<&crate::app::voice::screen_context::State>,
) -> Element<'a, Message> {
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
    let mentions_screen = state.captions.iter().any(|c| {
        c.role == vw_gateway_client::vw_api_types::voice::TranscriptRole::User
            && (c.text.contains("屏幕") || c.text.to_lowercase().contains("screen"))
    });
    for caption in state.captions {
        let role = match caption.role {
            vw_gateway_client::vw_api_types::voice::TranscriptRole::User => "你",
            vw_gateway_client::vw_api_types::voice::TranscriptRole::Assistant => "ShiJu",
        };
        body = body.push(column![text(role).size(12), text(caption.text).size(15)].spacing(4));
    }
    let active = !matches!(state.phase, Phase::Error | Phase::Ending | Phase::Connecting);
    if let Some(screen) = screen {
        let action = |label: &'static str, event| {
            button(text(label))
                .padding([8, 14])
                .style(rounded_action_btn_style)
                .on_press(message(Event::Screen(event)))
        };
        body = body.push(
            checkbox(screen.enabled)
                .label("本次通话允许单帧屏幕分享（默认关闭）")
                .on_toggle(|v| message(Event::Screen(ScreenEvent::Enable(v))))
                .style(settings_checkbox_style),
        );
        body = body.push(text("只分享确认窗口的一帧，最大 1024 像素并按大小缩小；不持续采集、不在本机保存。已发送内容可能被远端保留。").size(12));
        body = body.push(
            row![
                button(text(if mentions_screen {
                    "你提到了屏幕，选择分享窗口"
                } else {
                    "选择前台窗口"
                }))
                .padding([8, 14])
                .style(rounded_action_btn_style)
                .on_press_maybe(
                    (active && screen.enabled).then(|| message(Event::Screen(ScreenEvent::Offer)))
                ),
                action("屏幕录制授权", ScreenEvent::Permission),
                action("取消分享", ScreenEvent::Cancel),
            ]
            .spacing(8)
            .wrap(),
        );
        if screen.selecting {
            body = body.push(text("正在选择：三秒内切换到目标应用，随后返回确认。").size(13));
        }
        if screen.busy() {
            body = body.push(text("正在采集已确认窗口的单帧；可以取消。").size(13));
        }
        if let Some(target) = &screen.offered {
            body =
                body.push(text(format!("即将分享：{}。请确认窗口内容。", target.label)).size(13));
            body = body.push(action("确认采集并分享这一帧", ScreenEvent::Confirm));
        }
        body = body.push(action("改用当前聊天附件分享", ScreenEvent::AttachmentChat));
    }
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
        .max_height(760);
    settings_modal_overlay_with_escape(
        base,
        close,
        container(card).padding(12).max_width(664).max_height(784),
    )
}
#[cfg(test)]
#[path = "voice_chat_tests.rs"]
mod tests;
