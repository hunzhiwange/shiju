//! UI-only consent and lifecycle dispatch for a single local frame.
use super::{Event, end_conversation};
use crate::app::{
    App, Message,
    voice::screen_context::{self, Ticket},
};
use base64::{Engine as _, engine::general_purpose::STANDARD};
use iced::Task;
use vw_gateway_client::vw_api_types::voice::{ScreenPayload, VoiceCounter};

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Owner {
    Cloud(VoiceCounter),
    Local(u128),
}
fn owner(app: &App) -> Option<Owner> {
    if app.voice.local_conversation.is_some() {
        super::local_chat::screen_call(app).ok().map(Owner::Local)
    } else {
        app.voice.conversation.as_ref()?.snapshot().generation.map(Owner::Cloud)
    }
}

#[derive(Clone)]
pub enum ScreenEvent {
    Enable(bool),
    Offer,
    Inspect(u64),
    Confirm,
    Cancel,
    Permission,
    Ready(Ticket, Owner, Result<Vec<u8>, String>),
    AttachmentChat,
}
pub(super) fn update(app: &mut App, event: ScreenEvent) -> Task<Message> {
    match event {
        ScreenEvent::Enable(enabled) => {
            let enabled = enabled
                && (app.voice.local_conversation.is_some()
                    || app.voice.conversation.as_ref().is_some_and(|c| {
                        matches!(
                            c.snapshot().phase,
                            crate::app::voice::conversation::Phase::Listening
                                | crate::app::voice::conversation::Phase::Speaking
                                | crate::app::voice::conversation::Phase::Muted
                        )
                    }));
            app.voice.screen.set_enabled(enabled);
        }
        ScreenEvent::Cancel => app.voice.screen.cancel(),
        ScreenEvent::Permission => {
            if !app.voice.chat_active() {
                return Task::none();
            }
            if app.voice.local_conversation.is_some()
                && let Err(error) = super::local_chat::screen_call(app)
            {
                return app.show_error_toast(error);
            }
            use crate::app::voice::permissions::{Permission, SystemPermissions, VoicePermissions};
            if let Err(error) = VoicePermissions::new(SystemPermissions)
                .request_from_user_action(Permission::ScreenRecording)
            {
                return app.show_error_toast(error.to_string());
            }
        }
        ScreenEvent::Offer => {
            if !app.voice.screen.enabled || !app.voice.chat_active() {
                return Task::none();
            }
            app.voice.screen.cancel();
            app.voice.screen.selecting = true;
            let epoch = app.voice.screen.epoch;
            return Task::batch([
                super::super::after(std::time::Duration::from_secs(3), Message::Voice(Event::Screen(ScreenEvent::Inspect(epoch)))),
                app.show_info_toast("请在三秒内切换到要分享的应用，然后返回这里确认窗口。此时只选择目标，不采集屏幕。"),
            ]);
        }
        ScreenEvent::Inspect(epoch) => {
            if epoch != app.voice.screen.epoch
                || !app.voice.screen.enabled
                || !app.voice.chat_active()
            {
                return Task::none();
            }
            app.voice.screen.selecting = false;
            match screen_context::foreground() {
                Ok(target) => app.voice.screen.offered = Some(target),
                Err(error) => return app.show_error_toast(error),
            }
        }
        ScreenEvent::Confirm => {
            let Some(frame_owner) = owner(app) else {
                app.voice.screen.cancel();
                return app
                    .show_error_toast("请先完成当前轮次并选择支持图片输入的模型，再分享屏幕。");
            };
            if !screen_context::permitted() {
                app.voice.screen.cancel();
                return app.show_error_toast("屏幕权限不可用，未采集或发送。请授权后重新分享。");
            }
            let Some(target) = app.voice.screen.offered.take() else {
                return Task::none();
            };
            let Some(ticket) = app.voice.screen.begin(target, screen_context::permitted()) else {
                return Task::none();
            };
            app.voice.screen.generation = match frame_owner {
                Owner::Cloud(generation) => Some(generation),
                Owner::Local(_) => None,
            };
            let capture = ticket.clone();
            return Task::perform(
                async move {
                    tokio::task::spawn_blocking(move || screen_context::capture(&capture))
                        .await
                        .map_err(|_| "无法采集窗口。".to_owned())
                        .and_then(|r| r)
                },
                move |result| {
                    Message::Voice(Event::Screen(ScreenEvent::Ready(
                        ticket.clone(),
                        frame_owner,
                        result,
                    )))
                },
            );
        }
        ScreenEvent::Ready(ticket, frame_owner, result) => {
            return receive(app, ticket, frame_owner, result, screen_context::permitted());
        }
        ScreenEvent::AttachmentChat => {
            end_conversation(app);
            return app
                .show_info_toast("已返回当前聊天。请使用附件按钮选择要分享的图像，并确认发送。");
        }
    }
    Task::none()
}

/// Permission is queried at dispatch, after the OS capture boundary has also checked it.
pub(super) fn receive(
    app: &mut App,
    ticket: Ticket,
    frame_owner: Owner,
    result: Result<Vec<u8>, String>,
    permitted: bool,
) -> Task<Message> {
    if owner(app) != Some(frame_owner) {
        if app.voice.screen.owns(&ticket) {
            app.voice.screen.cancel();
        }
        return Task::none();
    }
    if let Err(error) = &result {
        if app.voice.screen.enabled && app.voice.screen.owns(&ticket) {
            app.voice.screen.cancel();
            return app.show_error_toast(error.clone());
        }
        return Task::none();
    }
    if !permitted {
        if app.voice.screen.owns(&ticket) {
            app.voice.screen.cancel();
        }
        return Task::none();
    }
    let Some(Ok(bytes)) = app.voice.screen.accept(&ticket, result, permitted) else {
        return Task::none();
    };
    let payload = ScreenPayload { png: STANDARD.encode(bytes) };
    if payload.bytes().is_err() {
        app.voice.screen.cancel();
        return app.show_error_toast("屏幕帧格式无效，未发送。");
    }
    match frame_owner {
        Owner::Local(call) => super::local_chat::share_screen(app, call, payload),
        Owner::Cloud(generation) => {
            if let Some(chat) = &app.voice.conversation {
                if let Err(error) = chat.share(ticket, generation, payload) {
                    return app.show_error_toast(error);
                }
                return app.show_info_toast(
                    "单帧已交给当前语音通道；没有持续采集。远端可能保留已分享内容。",
                );
            }
            Task::none()
        }
    }
}

#[cfg(test)]
#[path = "voice_screen_tests.rs"]
mod tests;
