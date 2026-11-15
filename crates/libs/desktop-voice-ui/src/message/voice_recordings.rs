use desktop_voice::recordings_playback::Replay;
use voice_recordings::{self as recordings, Entry};
use gateway_endpoint::GatewayEndpoint;
#[derive(Default)]
pub struct State {
    pub entries: Vec<Entry>,
    pub result: Option<String>,
    pub problem: Option<String>,
    pub replay: Option<Replay>,
    pub busy: bool,
    pub initialized: bool,
    pub generation: u64,
    pub scope: Option<super::voice_draft::Scope>,
    pub endpoint: Option<GatewayEndpoint>,
    pub cancel: Option<tokio::sync::watch::Sender<bool>>,
    pub native_cancel: Option<desktop_voice::local::Cancellation>,
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
impl State {
    pub fn stop(&mut self) {
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
    pub fn refresh(&mut self, path: Result<std::path::PathBuf, String>) -> Result<(), String> {
        self.entries = recordings::Store::open(&path?)?.list()?;
        self.problem = None;
        Ok(())
    }
}

use iced::Task;
use std::time::Duration;
use desktop_voice::preferences::Preferences;
pub enum Outcome { None, Task(Task<Event>), Apply(String) }
fn tick(generation: u64) -> Task<Event> {
    Task::perform(async { tokio::time::sleep(Duration::from_millis(100)).await }, move |_| Event::Tick(generation))
}
pub fn invalidate(state: &mut State, current: Option<super::voice_draft::Scope>, endpoint: &GatewayEndpoint, visible: bool, path: Result<std::path::PathBuf, String>) {
    if !state.initialized {
        state.initialized = true;
        if let Err(e) = state.refresh(path) {
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
    if !visible {
        state.stop();
    }
}
pub fn update(state: &mut State, event: Event, chat_active: bool, draft_busy: bool, preview_active: bool, scope: Option<super::voice_draft::Scope>, endpoint: GatewayEndpoint, path: Result<std::path::PathBuf, String>, preferences_path: Result<std::path::PathBuf, String>) -> Result<Outcome, String> {
    match event {
        Event::Refresh => {
            state.refresh(path.clone())?;
        }
        Event::Stop => {
            state.stop();
        }
        Event::Dismiss => {
            state.result = None;
        }
        Event::Delete(id) => {
            state.stop();
            recordings::Store::open(&path.clone()?)?.delete(&id)?;
            state.refresh(path.clone())?;
        }
        Event::Clear => {
            state.stop();
            recordings::Store::open(&path.clone()?)?.clear()?;
            state.refresh(path.clone())?;
        }
        Event::Play(id) => {
            if chat_active
                || draft_busy
                || preview_active
            {
                return Err("请先结束当前语音活动再回放录音。".into());
            }
            state.stop();
            let wav = recordings::Store::open(&path.clone()?)?.read(&id)?;
            state.replay = Some(Replay::start(wav)?);
            return Ok(Outcome::Task(tick(state.generation)));
        }
        Event::Tick(generation) => {
            if generation != state.generation {
                return Ok(Outcome::None);
            }
            if let Some(result) = state.replay.as_ref().and_then(Replay::result) {
                state.replay.take();
                result?;
            } else if state.replay.is_some() {
                return Ok(Outcome::Task(tick(generation)));
            }
        }
        Event::Retry(id) => {
            if chat_active
                || draft_busy
            {
                return Err("请先结束当前语音活动再重试识别。".into());
            }
            let scope = scope.ok_or("请先选择回填的聊天会话。")?;
            let preferences = Preferences::read(&preferences_path?)?;
            state.stop();
            let wav = recordings::Store::open(&path.clone()?)?.read(&id)?;
            state.scope = Some(scope.clone());
            state.endpoint = Some(endpoint.clone());
            state.busy = true;
            let generation = state.generation;
            let (tx, mut cancel) = tokio::sync::watch::channel(false);
            state.cancel = Some(tx);
            let native_cancel = desktop_voice::local::Cancellation::default();
            state.native_cancel = Some(native_cancel.clone());
            return Ok(Outcome::Task(Task::perform(
                async move {
                    tokio::select! {
                        biased;
                        _=cancel.changed()=>Err("已取消重新识别。".into()),
                        result=tokio::time::timeout(Duration::from_secs(90),retry(preferences,endpoint,scope.session_id().map(str::to_owned),generation,wav,native_cancel))=>result.unwrap_or_else(|_|Err("重新识别超时。".into())),
                    }
                },
                move |result| Event::Retried(generation, result),
            )));
        }
        Event::Retried(generation, result) => {
            if state.generation != generation || !state.busy {
                return Ok(Outcome::None);
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
            if draft_busy
                || chat_active
            {
                return Err("请先结束当前语音活动再回填。".into());
            }
            if let Some(text) = state.result.take() {
                return Ok(Outcome::Apply(text));
            }
            state.stop();
        }
    }
    Ok(Outcome::None)
}
async fn retry(
    preferences: Preferences,
    endpoint: GatewayEndpoint,
    session: Option<String>,
    generation: u64,
    wav: Vec<u8>,
    cancel: desktop_voice::local::Cancellation,
) -> Result<String, String> {
    super::voice_dictation::retry(preferences, endpoint, session, generation, wav, cancel).await
}

#[cfg(test)]
#[path = "voice_recordings_tests.rs"]
mod tests;
