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
    if app.voice.local_conversation.is_some() {
        return base;
    }
    let Some(chat) = &app.voice.conversation else {
        return base;
    };
    overlay_screen(base, chat.snapshot(), Some(&app.voice.screen))
}
#[cfg(test)]
fn overlay<'a>(base: Element<'a, Message>, state: Snapshot) -> Element<'a, Message> {
    overlay_screen(base, state, None)
}
pub(crate) fn overlay_screen<'a>(
    base: Element<'a, Message>,
    state: Snapshot,
    screen: Option<&'a crate::app::voice::screen_context::State>,
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
        body = body.push(screen_controls(screen, active, mentions_screen, false));
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

fn screen_controls(
    screen: &crate::app::voice::screen_context::State,
    active: bool,
    mentions_screen: bool,
    local: bool,
) -> Element<'static, Message> {
    let mut body = column![].spacing(12);
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
    if !screen.enabled {
        body = body.push(action("屏幕录制授权", ScreenEvent::Permission));
    }
    if screen.enabled {
        body = body.push(text("只分享确认窗口的一帧，最大 1024 像素并按大小缩小；不持续采集，采集端不另存截图。已发送内容可能被网关和模型保留。").size(12));
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
        if local {
            body = body.push(text("确认后发送给当前聊天模型并朗读回复；采集端不另存截图，图像由网关和模型按聊天策略处理、保存。").size(12));
        }
    }
    body.into()
}

/// Inline local controls leave chat approvals and questions unobstructed.
pub(crate) fn local_inline(app: &App) -> Option<Element<'_, Message>> {
    use crate::app::message::voice::local_chat::{Event as LocalEvent, Stage};
    let chat = app.voice.local_conversation.as_ref()?;
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
    let moon = iced::widget::tooltip(
        button(super::voice_moon::view(chat.muted))
            .padding(0)
            .style(|_, _| button::Style::default())
            .on_press_maybe(action.map(message)),
        text(label),
        iced::widget::tooltip::Position::Top,
    );
    let controls = row![
        button(text(if chat.muted { "取消静音" } else { "静音" }))
            .padding([6, 12])
            .style(rounded_action_btn_style)
            .on_press(message(Event::ChatMute)),
        button(text("打断本轮"))
            .padding([6, 12])
            .style(rounded_action_btn_style)
            .on_press(message(Event::ChatInterrupt)),
        iced::widget::tooltip(
            button(text("×").size(20))
                .width(32)
                .height(32)
                .padding(2)
                .style(super::system_settings_common::round_icon_btn_style)
                .on_press(message(Event::ChatEnd)),
            text("结束语音聊天"),
            iced::widget::tooltip::Position::Top
        ),
    ]
    .spacing(8)
    .align_y(iced::Alignment::Center);
    let mut body = column![moon, text(label).size(12), controls]
        .spacing(8)
        .align_x(iced::Alignment::Center)
        .width(Length::Fill);
    if let Some(notice) = &chat.notice {
        body = body.push(text(notice).size(12));
    }
    // Preserve explicit screen consent and confirmation without obscuring chat.
    let mentions_screen = chat.captions.iter().any(|(assistant, caption)| {
        !assistant && (caption.contains("屏幕") || caption.to_lowercase().contains("screen"))
    });
    body = body.push(screen_controls(
        &app.voice.screen,
        chat.stage == Stage::Ready,
        mentions_screen,
        true,
    ));
    Some(
        container(body)
            .padding(iced::Padding { top: 0.0, right: 7.0, bottom: 20.0, left: 7.0 })
            .into(),
    )
}
