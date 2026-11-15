//! Consent and lifecycle policy for one local screen frame.
use desktop_voice::screen_context::{self, Ticket};
use base64::{Engine as _, engine::general_purpose::STANDARD};
use iced::Task;
use vw_api_voice::{ScreenPayload, VoiceCounter};
pub enum Outcome {
    None, Error(String), Permission, AttachmentChat,
    Offer(Task<ScreenEvent>), Task(Task<ScreenEvent>),
    Share(Ticket, Owner, ScreenPayload),
}
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Owner {
    Cloud(VoiceCounter),
    Local(u128),
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
pub fn update(screen: &mut screen_context::State, event: ScreenEvent, active: bool, enable_allowed: bool, current_owner: Option<Owner>) -> Outcome {
    match event {
        ScreenEvent::Enable(enabled) => {
            let enabled = enabled && enable_allowed;
            screen.set_enabled(enabled);
        }
        ScreenEvent::Cancel => screen.cancel(),
        ScreenEvent::Permission => {
            if !active {
                return Outcome::None;
            }
            return Outcome::Permission;
        }
        ScreenEvent::Offer => {
            if !screen.enabled || !active {
                return Outcome::None;
            }
            screen.cancel();
            screen.selecting = true;
            let epoch = screen.epoch;
            return Outcome::Offer(Task::perform(async { tokio::time::sleep(std::time::Duration::from_secs(3)).await }, move |_| ScreenEvent::Inspect(epoch)));
        }
        ScreenEvent::Inspect(epoch) => {
            if epoch != screen.epoch
                || !screen.enabled
                || !active
            {
                return Outcome::None;
            }
            screen.selecting = false;
            match screen_context::foreground() {
                Ok(target) => screen.offered = Some(target),
                Err(error) => return Outcome::Error(error),
            }
        }
        ScreenEvent::Confirm => {
            let Some(frame_owner) = current_owner else {
                screen.cancel();
                return Outcome::Error("请先完成当前轮次并选择支持图片输入的模型，再分享屏幕。".into());
            };
            if !screen_context::permitted() {
                screen.cancel();
                return Outcome::Error("屏幕权限不可用，未采集或发送。请授权后重新分享。".into());
            }
            let Some(target) = screen.offered.take() else {
                return Outcome::None;
            };
            let Some(ticket) = screen.begin(target, screen_context::permitted()) else {
                return Outcome::None;
            };
            screen.generation = match frame_owner {
                Owner::Cloud(generation) => Some(generation),
                Owner::Local(_) => None,
            };
            let capture = ticket.clone();
            return Outcome::Task(Task::perform(
                async move {
                    tokio::task::spawn_blocking(move || screen_context::capture(&capture))
                        .await
                        .map_err(|_| "无法采集窗口。".to_owned())
                        .and_then(|r| r)
                },
                move |result| {
                    ScreenEvent::Ready(
                        ticket.clone(),
                        frame_owner,
                        result,
                    )
                },
            ));
        }
        ScreenEvent::Ready(ticket, frame_owner, result) => {
            return receive(screen, current_owner, ticket, frame_owner, result, screen_context::permitted());
        }
        ScreenEvent::AttachmentChat => {
            return Outcome::AttachmentChat;
        }
    }
    Outcome::None
}

/// Permission is queried at dispatch, after the OS capture boundary has also checked it.
pub fn receive(
    screen: &mut screen_context::State,
    current_owner: Option<Owner>,
    ticket: Ticket,
    frame_owner: Owner,
    result: Result<Vec<u8>, String>,
    permitted: bool,
) -> Outcome {
    if current_owner != Some(frame_owner) {
        if screen.owns(&ticket) {
            screen.cancel();
        }
        return Outcome::None;
    }
    if let Err(error) = &result {
        if screen.enabled && screen.owns(&ticket) {
            screen.cancel();
            return Outcome::Error(error.clone());
        }
        return Outcome::None;
    }
    if !permitted {
        if screen.owns(&ticket) {
            screen.cancel();
        }
        return Outcome::None;
    }
    let Some(Ok(bytes)) = screen.accept(&ticket, result, permitted) else {
        return Outcome::None;
    };
    let payload = ScreenPayload { png: STANDARD.encode(bytes) };
    if payload.bytes().is_err() {
        screen.cancel();
        return Outcome::Error("屏幕帧格式无效，未发送。".into());
    }
    Outcome::Share(ticket, frame_owner, payload)
}

#[cfg(test)]
#[path = "voice_screen_tests.rs"]
mod tests;
