//! Transactional voice form. Every asynchronous result is scoped to one edit generation.
use crate::app::voice::{
    capture::{InputDevice, input_devices},
    preferences::{Edit, Preferences},
    preview::Preview,
};
use crate::app::{App, Message};
use iced::Task;
use std::time::Duration;
use vw_config_types::voice::{RecognitionLanguage, VoiceConfig, VoiceName};
use vw_gateway_client::{GatewayClient, GatewayEndpoint};

#[derive(Default)]
pub(crate) struct State {
    pub edit: Option<Edit>,
    pub devices: Vec<InputDevice>,
    pub dictionary: String,
    pub busy: bool,
    pub preview: Option<Preview>,
    generation: u64,
    endpoint: Option<GatewayEndpoint>,
}
#[derive(Debug, Clone)]
pub struct Loaded {
    pub config: VoiceConfig,
    pub local: Preferences,
    pub devices: Vec<InputDevice>,
    pub warning: Option<String>,
}
#[derive(Clone)]
pub enum Event {
    Open,
    Cancel,
    Save,
    Preview,
    StopPreview,
    Tick(u64),
    Loaded(u64, Result<Loaded, String>),
    Saved(u64, Result<(), String>),
    Microphone(Option<String>),
    Provider(String),
    Language(RecognitionLanguage),
    Voice(VoiceName),
    Dictation(bool),
    Realtime(bool),
    Dictionary(String),
    AddWord,
    RemoveWord(usize),
}
// Settings messages may appear in diagnostic traces; never format dictionary/context payloads.
impl std::fmt::Debug for Event {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("VoiceSettingsEvent").field(&std::mem::discriminant(self)).finish()
    }
}
fn message(event: Event) -> Message {
    Message::Settings(super::SettingsMessage::Voice(event))
}
fn same_endpoint(a: &GatewayEndpoint, b: &GatewayEndpoint) -> bool {
    a.base_url() == b.base_url()
        && a.tenant_scope() == b.tenant_scope()
        && a.auth.as_ref().and_then(|a| a.skey.as_ref())
            == b.auth.as_ref().and_then(|a| a.skey.as_ref())
}
pub(crate) fn invalidate(app: &mut App) {
    let current = crate::app::gateway_client_endpoint();
    let state = &mut app.voice.settings;
    if state.endpoint.as_ref().is_some_and(|old| !same_endpoint(old, &current))
        || ((!app.show_system_settings
            || app.system_settings_tab
                != crate::app::components::system_settings::SystemTab::Voice)
            && state.endpoint.is_some())
    {
        state.preview.take();
        state.edit = None;
        state.endpoint = None;
        state.busy = false;
        state.generation = state.generation.wrapping_add(1);
    }
}
fn tick(generation: u64) -> Task<Message> {
    crate::app::message::after(Duration::from_millis(100), message(Event::Tick(generation)))
}
pub(crate) fn update(app: &mut App, event: Event) -> Task<Message> {
    invalidate(app);
    match event {
        Event::Open => {
            if app.voice.settings.busy {
                return Task::none();
            }
            let endpoint = crate::app::gateway_client_endpoint();
            let state = &mut app.voice.settings;
            state.preview.take();
            state.edit = None;
            state.busy = true;
            state.generation = state.generation.wrapping_add(1);
            state.endpoint = Some(endpoint.clone());
            let generation = state.generation;
            Task::perform(
                async move {
                    let config = GatewayClient::new(endpoint)?
                        .voice_config::<VoiceConfig>()
                        .await
                        .map_err(|e| e.to_string())?;
                    let (local, devices, warning) = tokio::task::spawn_blocking(|| {
                        let (local, warning) = match Preferences::load() {
                            Ok(local) => (local, None),
                            Err(error) => (
                                Preferences::default(),
                                Some(format!("{error} 保存将重置本机偏好。")),
                            ),
                        };
                        let devices = input_devices().map_err(|_| "无法列出麦克风。".to_owned())?;
                        Ok::<_, String>((local, devices, warning))
                    })
                    .await
                    .map_err(|_| "无法读取本机语音设备。")??;
                    Ok(Loaded { config, local, devices, warning })
                },
                move |result| message(Event::Loaded(generation, result)),
            )
        }
        Event::Loaded(generation, result) => {
            let state = &mut app.voice.settings;
            if generation != state.generation || state.endpoint.is_none() {
                return Task::none();
            }
            state.busy = false;
            match result {
                Ok(loaded) => {
                    state.edit = Some(Edit::new(loaded.config, loaded.local));
                    state.devices = loaded.devices;
                    state.dictionary.clear();
                    if let Some(warning) = loaded.warning {
                        app.show_error_toast(warning)
                    } else {
                        Task::none()
                    }
                }
                Err(error) => app.show_error_toast(error),
            }
        }
        Event::Cancel => {
            let state = &mut app.voice.settings;
            if state.busy {
                return Task::none();
            }
            state.preview.take();
            if let Some(edit) = state.edit.take() {
                let _ = edit.cancel();
            }
            state.endpoint = None;
            state.generation = state.generation.wrapping_add(1);
            Task::none()
        }
        Event::Save => {
            let state = &mut app.voice.settings;
            if state.busy {
                return Task::none();
            }
            let (Some(edit), Some(endpoint)) = (&state.edit, state.endpoint.clone()) else {
                return Task::none();
            };
            let patch = match edit.patch() {
                Ok(p) => p,
                Err(e) => return app.show_error_toast(e),
            };
            let local = edit.local.clone();
            let generation = state.generation;
            state.preview.take();
            state.busy = true;
            Task::perform(
                async move {
                    let path = Preferences::path()?;
                    GatewayClient::new(endpoint)?.voice_settings_patch(&patch).await?;
                    tokio::task::spawn_blocking(move || local.write(&path))
                        .await
                        .map_err(|_| "网关设置已保存，本机设备保存失败。")?
                        .map_err(|_| "网关设置已保存，本机设备保存失败，请重试。".to_owned())
                },
                move |result| message(Event::Saved(generation, result)),
            )
        }
        Event::Saved(generation, result) => {
            let state = &mut app.voice.settings;
            if generation != state.generation || state.endpoint.is_none() {
                return Task::none();
            }
            state.busy = false;
            match result {
                Ok(()) => {
                    state.edit = None;
                    state.endpoint = None;
                    app.show_toast(
                        crate::app::state::ToastKind::Success,
                        "语音设置已保存，新音色将在下次语音聊天生效。",
                    )
                }
                Err(e) => app.show_error_toast(e),
            }
        }
        Event::Preview => {
            let state = &mut app.voice.settings;
            if state.busy || state.preview.is_some() {
                return Task::none();
            }
            if app.voice.conversation.is_some() {
                return app.show_error_toast("请先结束语音聊天再试听。");
            }
            let (Some(edit), Some(endpoint)) = (&state.edit, state.endpoint.clone()) else {
                return Task::none();
            };
            match Preview::start(endpoint, edit.config.voice) {
                Ok(preview) => {
                    state.preview = Some(preview);
                    tick(state.generation)
                }
                Err(e) => app.show_error_toast(e),
            }
        }
        Event::StopPreview => {
            app.voice.settings.preview.take();
            Task::none()
        }
        Event::Tick(generation) => {
            let state = &mut app.voice.settings;
            if generation != state.generation {
                return Task::none();
            }
            if let Some(result) = state.preview.as_ref().and_then(Preview::result) {
                state.preview.take();
                return match result {
                    Ok(()) => Task::none(),
                    Err(e) => app.show_error_toast(e),
                };
            }
            if state.preview.is_some() { tick(generation) } else { Task::none() }
        }
        event => {
            let state = &mut app.voice.settings;
            if state.busy {
                return Task::none();
            }
            let Some(edit) = &mut state.edit else {
                return Task::none();
            };
            match event {
                Event::Microphone(value) => edit.local.microphone = value,
                Event::Provider(value) => {
                    edit.config.provider_id =
                        (!value.trim().is_empty()).then(|| value.trim().to_owned())
                }
                Event::Language(value) => edit.config.language = value,
                Event::Voice(value) => {
                    state.preview.take();
                    edit.config.voice = value;
                }
                Event::Dictation(value) => edit.config.dictation_enabled = value,
                Event::Realtime(value) => edit.config.realtime_enabled = value,
                Event::Dictionary(value) => state.dictionary = value,
                Event::AddWord => {
                    let word = state.dictionary.trim().to_owned();
                    if edit.config.dictionary.contains(&word) {
                        return Task::none();
                    }
                    let mut config = edit.config.clone();
                    config.dictionary.push(word);
                    if let Err(e) = config.validate() {
                        return app.show_error_toast(e);
                    }
                    edit.config = config;
                    state.dictionary.clear();
                }
                Event::RemoveWord(index) if index < edit.config.dictionary.len() => {
                    edit.config.dictionary.remove(index);
                }
                _ => {}
            }
            Task::none()
        }
    }
}
#[cfg(test)]
#[path = "voice_tests.rs"]
mod tests;
