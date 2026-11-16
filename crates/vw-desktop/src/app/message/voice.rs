//! Dictation and conversation routing. Local audio stays on the client; screen sharing is explicit.
use super::voice_draft::{Composer, Scope};
#[cfg(test)]
use super::voice_draft::Phase;
use crate::app::{App, Message};
#[cfg(test)]
use gateway_endpoint::GatewayEndpoint;
use iced::Task;
#[cfg(test)]
use iced::widget::text_editor;
#[cfg(test)]
use std::sync::Arc;
#[cfg(test)]
use std::sync::Mutex;

#[path = "voice_screen.rs"]
mod screen;
pub use screen::ScreenEvent;

#[path = "voice_dictation.rs"]
pub(super) mod dictation;

#[path = "voice_local_chat.rs"]
pub(crate) mod local_chat;

pub use desktop_voice_ui::message::voice::State;

pub use desktop_voice_ui::components::voice_onboarding::OnboardingEvent;
pub use desktop_voice_ui::message::voice::Event;
pub(super) fn scope(app: &App) -> Option<Scope> {
    let endpoint = crate::app::gateway_client_endpoint();
    Some(Scope {
        endpoint: endpoint.base_url(),
        tenant: serde_json::to_string(endpoint.tenant_scope()).ok()?,
        project: app.project_path.clone(),
        composer: match &app.active_session_id {
            Some(id) => Composer::Session(id.clone()),
            None => Composer::Draft(app.session_view_cache.navigation_generation),
        },
    })
}
pub(crate) fn end_conversation(app: &mut App) {
    local_chat::end(app);
    desktop_voice_ui::message::voice::end_cloud(&mut app.voice);
}
pub(crate) fn cancel(app: &mut App) {
    desktop_voice_ui::message::voice::clear_pending(&mut app.voice);
    end_conversation(app);
    desktop_voice_ui::message::voice::cancel_dictation(&mut app.voice);
}
/// Called before and after dispatch so a switch away and back cannot revive old work.
pub(crate) fn invalidate(app: &mut App) {
    desktop_voice_ui::message::voice::invalidate_screen(&mut app.voice);
    super::settings::voice::invalidate(app);
    super::voice_recordings::invalidate(app);
    let current = scope(app);
    let endpoint = crate::app::gateway_client_endpoint();
    if desktop_voice_ui::message::voice::conversation_changed(&app.voice, &current, &endpoint) {
        end_conversation(app);
    }
    if app.voice.draft.scope.is_none() {
        return;
    }
    if desktop_voice_ui::message::voice::draft_changed(&app.voice, &current, &endpoint) {
        cancel(app);
    } else if app.voice.draft.scope.is_some() {
        let text = app
            .current_session_runtime_ref()
            .map(|runtime| runtime.input_editor.text())
            .unwrap_or_default();
        app.voice.draft.observe(&text);
    }
}
fn fail(app: &mut App, message: String) -> Task<Message> {
    desktop_voice_ui::message::voice::fail(&mut app.voice);
    app.show_toast(crate::app::state::ToastKind::Error, message)
}
fn capture_message(event: desktop_voice_ui::message::voice::CaptureEvent) -> Message {
    use desktop_voice_ui::message::voice::CaptureEvent;
    Message::Voice(match event {
        CaptureEvent::Started(g, r) => Event::Started(g, r),
        CaptureEvent::Tick(g) => Event::Tick(g),
        CaptureEvent::Transcribed(g, r) => Event::Transcribed(g, r),
    })
}
fn capture_step(
    app: &mut App,
    step: desktop_voice_ui::message::voice::CaptureStep,
) -> Task<Message> {
    use desktop_voice_ui::message::voice::CaptureStep;
    match step {
        CaptureStep::Idle => Task::none(),
        CaptureStep::Task(task) => task.map(capture_message),
        CaptureStep::Finish => update(app, Event::Finish),
        CaptureStep::Error(error) => fail(app, error),
        CaptureStep::Completed(g, r) => update(app, Event::Completed(g, r)),
    }
}
pub(super) fn paste(app: &mut App, text: String, restore_cursor: bool) {
    let cursor = app.voice.cursor;
    let editor = &mut app.current_session_runtime_mut().input_editor;
    desktop_voice_ui::message::voice::paste(
        editor,
        if restore_cursor { cursor } else { None },
        text,
    );
    if app.active_session_id.is_none() {
        app.input_editor = app.current_session_runtime_mut().input_editor.clone();
    }
}
fn check_start(app: &mut App, chat: bool) -> Option<Task<Message>> {
    use desktop_voice_ui::message::voice::StartCheck;
    match desktop_voice_ui::message::voice::check_start(&mut app.voice, chat) {
        StartCheck::Ready => None,
        StartCheck::Idle => Some(Task::none()),
        StartCheck::Error(error) => Some(app.show_error_toast(error)),
        StartCheck::Info(info) => Some(app.show_toast(crate::app::state::ToastKind::Info, info)),
    }
}
pub(crate) fn update(app: &mut App, event: Event) -> Task<Message> {
    invalidate(app);
    match event {
        Event::Local(event) => local_chat::update(app, event),
        Event::Screen(event) => screen::update(app, event),
        Event::Onboarding(event) => {
            match desktop_voice_ui::message::voice::onboarding(&mut app.voice, event) {
                Ok(Some(text)) => iced::clipboard::write(text),
                Ok(None) => Task::none(),
                Err(error) => app.show_error_toast(error),
            }
        }
        Event::HotkeyTick => super::voice_hotkeys::tick(app),
        Event::Recordings(event) => super::voice_recordings::update(app, event),
        Event::Transcribed(generation, result) => {
            let step = desktop_voice_ui::message::voice::capture_event(
                &mut app.voice,
                desktop_voice_ui::message::voice::CaptureEvent::Transcribed(generation, result),
            );
            capture_step(app, step)
        }
        Event::ChatStart => {
            if let Some(task) = check_start(app, true) {
                return task;
            }
            let preferences = match crate::app::voice::preferences::load() {
                Ok(p) => p,
                Err(error) => return app.show_error_toast(error),
            };
            if preferences.engine == crate::app::voice::preferences::VoiceEngine::Local {
                return local_chat::start(app, preferences);
            }
            let Some(scope) = scope(app) else {
                return fail(app, "请先选择聊天会话。".into());
            };
            let endpoint = crate::app::gateway_client_endpoint();
            if scope.session_id().is_none() {
                return fail(app, "云端语音聊天需要已建立的聊天会话。".into());
            }
            match desktop_voice_ui::message::voice::start_cloud(
                &mut app.voice,
                scope,
                endpoint,
                || crate::app::voice::preferences::load().map(|p| p.microphone),
            ) {
                Ok(()) => Task::none(),
                Err(error) => app.show_toast(crate::app::state::ToastKind::Error, error),
            }
        }
        Event::ChatMute => {
            if app.voice.local_conversation.is_some() {
                return local_chat::mute(app);
            }
            if let Some(chat) = &app.voice.conversation {
                chat.mute();
            }
            Task::none()
        }
        Event::ChatInterrupt => {
            if app.voice.local_conversation.is_some() {
                return local_chat::interrupt(app);
            }
            if let Some(chat) = &app.voice.conversation {
                chat.interrupt();
            }
            Task::none()
        }
        Event::ChatEnd => {
            end_conversation(app);
            local_chat::tick(app)
        }
        Event::ChatTick => {
            if app.voice.local_conversation.is_some() || app.voice.local_stop.is_some() {
                return local_chat::tick(app);
            }
            if let Some(error) = desktop_voice_ui::message::voice::cloud_error(&mut app.voice) {
                return app.show_error_toast(error);
            }
            Task::none()
        }
        Event::Cancel => {
            cancel(app);
            Task::none()
        }
        Event::Start => {
            if let Some(task) = check_start(app, false) {
                return task;
            }
            let preferences = match crate::app::voice::preferences::load() {
                Ok(preferences) => preferences,
                Err(error) => return fail(app, error),
            };
            let Some(scope) = scope(app) else {
                return fail(app, "无法确定当前听写草稿。".into());
            };
            if preferences.engine == crate::app::voice::preferences::VoiceEngine::Cloud
                && scope.session_id().is_none()
            {
                return fail(app, "云端听写需要已建立的聊天会话；本机听写可用于新草稿。".into());
            }
            let runtime = app.current_session_runtime_mut();
            let cursor = runtime.input_editor.cursor();
            let text = runtime.input_editor.text();
            desktop_voice_ui::message::voice::start_capture(
                &mut app.voice,
                preferences,
                scope,
                crate::app::gateway_client_endpoint(),
                cursor,
                text,
            )
            .map(capture_message)
        }
        Event::Started(generation, result) => {
            let step = desktop_voice_ui::message::voice::capture_event(
                &mut app.voice,
                desktop_voice_ui::message::voice::CaptureEvent::Started(generation, result),
            );
            capture_step(app, step)
        }
        Event::Tick(generation) => {
            let step = desktop_voice_ui::message::voice::capture_event(
                &mut app.voice,
                desktop_voice_ui::message::voice::CaptureEvent::Tick(generation),
            );
            capture_step(app, step)
        }
        Event::Finish => {
            let step = desktop_voice_ui::message::voice::finish_capture(&mut app.voice);
            capture_step(app, step)
        }
        Event::Completed(generation, result) => {
            let current_scope = scope(app);
            let current = app
                .current_session_runtime_ref()
                .map(|r| r.input_editor.text())
                .unwrap_or_default();
            let result = desktop_voice_ui::message::voice::complete(
                &mut app.voice,
                generation,
                result,
                current_scope,
                &current,
                crate::app::voice::preferences::path(),
                crate::app::voice::recordings::path(),
            );
            if let Some(text) = result.paste {
                paste(app, text, true);
            }
            if let Some(error) = result.error {
                return app.show_error_toast(error);
            }
            if result.edited {
                return app.show_toast(
                    crate::app::state::ToastKind::Info,
                    "草稿已编辑，听写结果已保留；点击应用听写插入。",
                );
            }
            Task::none()
        }
        Event::Apply => {
            if let Some(text) = app.voice.draft.pending.take() {
                paste(app, text, false);
            }
            cancel(app);
            Task::none()
        }
    }
}
#[cfg(test)]
#[path = "voice_composer_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "voice_hotkey_lifecycle_tests.rs"]
mod hotkey_lifecycle_tests;

#[cfg(test)]
#[path = "voice_new_draft_tests.rs"]
mod new_draft_tests;

#[cfg(test)]
#[path = "voice_ui_tests.rs"]
mod ui_tests;
