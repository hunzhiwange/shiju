//! Explicit archive actions; retry replies are bound to the current composer generation.
use super::voice_draft::Phase;
use crate::app::voice::preferences::Preferences;
use crate::app::voice::{
    recordings::{Entry, Store},
    recordings_playback::Replay,
};
use crate::app::{App, Message};
use iced::Task;
use std::time::Duration;
use vw_gateway_client::GatewayEndpoint;
#[derive(Default)]
pub(crate) struct State {
    pub entries: Vec<Entry>,
    pub result: Option<String>,
    pub problem: Option<String>,
    pub replay: Option<Replay>,
    pub busy: bool,
    initialized: bool,
    generation: u64,
    scope: Option<super::voice_draft::Scope>,
    endpoint: Option<GatewayEndpoint>,
    cancel: Option<tokio::sync::watch::Sender<bool>>,
    native_cancel: Option<crate::app::voice::local::Cancellation>,
}
#[derive(Clone)]
pub enum Event {
    Refresh,
    Play(String),
    Stop,
    Delete(String),
    Clear,
    Retry(String),
    Apply,
    Dismiss,
    Tick(u64),
    Retried(u64, Result<String, String>),
}
impl std::fmt::Debug for Event {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("VoiceRecordingsEvent").field(&std::mem::discriminant(self)).finish()
    }
}
fn message(event: Event) -> Message {
    Message::Voice(super::voice::Event::Recordings(event))
}
fn tick(generation: u64) -> Task<Message> {
    super::after(Duration::from_millis(100), message(Event::Tick(generation)))
}
impl State {
    pub(crate) fn stop(&mut self) {
        if let Some(cancel) = self.native_cancel.take() {
            cancel.cancel();
        }
        self.replay.take();
        self.cancel.take();
        self.busy = false;
        self.result = None;
        self.generation = self.generation.wrapping_add(1);
        self.scope = None;
        self.endpoint = None;
    }
    fn refresh(&mut self) -> Result<(), String> {
        self.entries = Store::local()?.list()?;
        self.problem = None;
        Ok(())
    }
}
pub(crate) fn invalidate(app: &mut App) {
    let current = super::voice::scope(app);
    let endpoint = crate::app::gateway_client_endpoint();
    let state = &mut app.voice.recordings;
    if !state.initialized {
        state.initialized = true;
        if let Err(e) = state.refresh() {
            state.problem = Some(e);
        }
    }
    let auth_changed = state.endpoint.as_ref().is_some_and(|old| {
        old.base_url() != endpoint.base_url()
            || old.tenant_scope() != endpoint.tenant_scope()
            || old.auth.as_ref().and_then(|a| a.skey.as_ref())
                != endpoint.auth.as_ref().and_then(|a| a.skey.as_ref())
    });
    if state.scope.is_some() && (state.scope != current || auth_changed) {
        state.stop();
    }
    if !app.show_system_settings
        || app.system_settings_tab != crate::app::components::system_settings::SystemTab::Voice
    {
        state.stop();
    }
}
pub(crate) fn update(app: &mut App, event: Event) -> Task<Message> {
    invalidate(app);
    match handle(app, event) {
        Ok(task) => task,
        Err(e) => app.show_error_toast(e),
    }
}
fn handle(app: &mut App, event: Event) -> Result<Task<Message>, String> {
    match event {
        Event::Refresh => {
            app.voice.recordings.refresh()?;
        }
        Event::Stop => {
            app.voice.recordings.stop();
        }
        Event::Dismiss => {
            app.voice.recordings.result = None;
        }
        Event::Delete(id) => {
            app.voice.recordings.stop();
            Store::local()?.delete(&id)?;
            app.voice.recordings.refresh()?;
        }
        Event::Clear => {
            app.voice.recordings.stop();
            Store::local()?.clear()?;
            app.voice.recordings.refresh()?;
        }
        Event::Play(id) => {
            if app.voice.conversation.is_some()
                || matches!(app.voice.draft.phase, Phase::Recording | Phase::Transcribing)
                || app.voice.settings.preview.is_some()
            {
                return Err("请先结束当前语音活动再回放录音。".into());
            }
            app.voice.recordings.stop();
            let wav = Store::local()?.read(&id)?;
            app.voice.recordings.replay = Some(Replay::start(wav)?);
            return Ok(tick(app.voice.recordings.generation));
        }
        Event::Tick(generation) => {
            if generation != app.voice.recordings.generation {
                return Ok(Task::none());
            }
            if let Some(result) = app.voice.recordings.replay.as_ref().and_then(Replay::result) {
                app.voice.recordings.replay.take();
                result?;
            } else if app.voice.recordings.replay.is_some() {
                return Ok(tick(generation));
            }
        }
        Event::Retry(id) => {
            if app.voice.conversation.is_some()
                || matches!(app.voice.draft.phase, Phase::Recording | Phase::Transcribing)
            {
                return Err("请先结束当前语音活动再重试识别。".into());
            }
            let scope = super::voice::scope(app).ok_or("请先选择回填的聊天会话。")?;
            let endpoint = crate::app::gateway_client_endpoint();
            let preferences = Preferences::load()?;
            app.voice.recordings.stop();
            let wav = Store::local()?.read(&id)?;
            let state = &mut app.voice.recordings;
            state.scope = Some(scope.clone());
            state.endpoint = Some(endpoint.clone());
            state.busy = true;
            let generation = state.generation;
            let (tx, mut cancel) = tokio::sync::watch::channel(false);
            state.cancel = Some(tx);
            let native_cancel = crate::app::voice::local::Cancellation::default();
            state.native_cancel = Some(native_cancel.clone());
            return Ok(Task::perform(
                async move {
                    tokio::select! {
                        biased;
                        _=cancel.changed()=>Err("已取消重新识别。".into()),
                        result=tokio::time::timeout(Duration::from_secs(90),retry(preferences,endpoint,scope.session,generation,wav,native_cancel))=>result.unwrap_or_else(|_|Err("重新识别超时。".into())),
                    }
                },
                move |result| message(Event::Retried(generation, result)),
            ));
        }
        Event::Retried(generation, result) => {
            let state = &mut app.voice.recordings;
            if state.generation != generation || !state.busy {
                return Ok(Task::none());
            }
            state.busy = false;
            state.cancel = None;
            if let Some(cancel) = state.native_cancel.take() {
                cancel.cancel();
            }
            let text = result?;
            if text.trim().is_empty() {
                return Err("没有识别到文字。".into());
            }
            state.result = Some(text);
        }
        Event::Apply => {
            if matches!(app.voice.draft.phase, Phase::Recording | Phase::Transcribing)
                || app.voice.conversation.is_some()
            {
                return Err("请先结束当前语音活动再回填。".into());
            }
            if let Some(text) = app.voice.recordings.result.take() {
                super::voice::paste(app, text, false);
            }
            app.voice.recordings.stop();
        }
    }
    Ok(Task::none())
}
async fn retry(
    preferences: Preferences,
    endpoint: GatewayEndpoint,
    session: String,
    generation: u64,
    wav: Vec<u8>,
    cancel: crate::app::voice::local::Cancellation,
) -> Result<String, String> {
    super::voice::dictation::retry(preferences, endpoint, session, generation, wav, cancel).await
}
#[cfg(test)]
#[path = "voice_recordings_tests.rs"]
mod tests;
