//! Transactional voice form. Every asynchronous result is scoped to one edit generation.
use crate::app::voice::{
    capture::{InputDevice, input_devices},
    preferences::{Edit, Preferences, VoiceEngine},
    preview::Preview,
};
use crate::app::{App, Message};
use iced::Task;
use std::time::Duration;
use vw_config_types::voice::{RecognitionLanguage, VoiceConfig, VoiceName};
use vw_gateway_client::{GatewayClient, GatewayEndpoint};

#[path = "voice_io.rs"]
mod io;
pub(crate) use io::{LocalResources, resources as local_resources};

#[derive(Default)]
pub(crate) struct State {
    pub edit: Option<Edit>,
    pub devices: Vec<InputDevice>,
    pub dictionary: String,
    pub busy: bool,
    pub recording_hotkey: Option<bool>,
    pub preview: Option<Preview>,
    pub resources: LocalResources,
    pub cloud_ready: bool,
    pub cloud_error: Option<String>,
    generation: u64,
    endpoint: Option<GatewayEndpoint>,
}
#[derive(Debug, Clone)]
pub struct Loaded {
    pub config: VoiceConfig,
    pub local: Preferences,
    pub devices: Vec<InputDevice>,
    pub warning: Option<String>,
    pub resources: LocalResources,
    pub cloud_ready: bool,
    pub cloud_error: Option<String>,
}
#[derive(Clone)]
pub enum Event {
    Open,
    Engine(VoiceEngine),
    LocalLanguage(Option<String>),
    LocalVoice(Option<String>),
    Permission(crate::app::voice::permissions::Permission),
    RefreshLocal,
    Resources(u64, Option<String>, LocalResources),
    CloudLoaded(u64, Result<VoiceConfig, String>),
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
    RetainRecordings(bool),
    Realtime(bool),
    HotkeysEnabled(bool),
    RecordHotkey(Option<bool>),
    RecordedHotkey(crate::app::voice::hotkeys::Shortcut),
    HotkeyPermission,
    DisableHotkeys,
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
        state.recording_hotkey = None;
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
        Event::Permission(permission) => {
            use crate::app::voice::permissions::{SystemPermissions, VoicePermissions};
            if app.voice.settings.busy || app.voice.settings.edit.is_none() {
                return Task::none();
            }
            match VoicePermissions::new(SystemPermissions).request_from_user_action(permission) {
                Ok(()) => refresh_local(&app.voice.settings),
                Err(_) => app.show_error_toast("无法申请系统权限，请在带语音用途说明的 ShiJu.app 中操作，或在 macOS 隐私与安全性中检查授权后刷新。"),
            }
        }
        Event::RefreshLocal => refresh_local(&app.voice.settings),
        Event::Resources(generation, language, resources) => {
            let state = &mut app.voice.settings;
            if state.generation == generation
                && state.edit.as_ref().is_some_and(|edit| {
                    edit.local.engine == VoiceEngine::Local && edit.local.local_language == language
                })
            {
                state.resources = resources;
            }
            Task::none()
        }
        Event::CloudLoaded(generation, result) => {
            let state = &mut app.voice.settings;
            if state.generation != generation
                || state.edit.as_ref().is_none_or(|edit| edit.local.engine != VoiceEngine::Cloud)
            {
                return Task::none();
            }
            state.busy = false;
            match result {
                Ok(config) => {
                    let local = state.edit.as_ref().unwrap().local.clone();
                    state.edit = Some(Edit::new(config, local));
                    state.cloud_ready = true;
                    state.cloud_error = None;
                    Task::none()
                }
                Err(error) => {
                    state.cloud_error = Some(error.clone());
                    app.show_error_toast(error)
                }
            }
        }
        Event::Engine(engine) => {
            if app.voice.settings.busy || app.voice.settings.edit.is_none() {
                return Task::none();
            }
            if engine == VoiceEngine::Local && !cfg!(target_os = "macos") {
                return app.show_error_toast("本机语音当前仅支持 macOS。");
            }
            super::super::voice::cancel(app);
            let state = &mut app.voice.settings;
            state.edit.as_mut().unwrap().local.engine = engine;
            state.generation = state.generation.wrapping_add(1);
            if engine == VoiceEngine::Local {
                return refresh_local(state);
            }
            if state.cloud_ready {
                return Task::none();
            }
            let Some(endpoint) = state.endpoint.clone() else {
                return Task::none();
            };
            state.busy = true;
            let generation = state.generation;
            Task::perform(io::cloud(endpoint), move |result| {
                message(Event::CloudLoaded(generation, result))
            })
        }
        Event::LocalLanguage(_) | Event::LocalVoice(_) => {
            if app.voice.settings.busy || app.voice.settings.edit.is_none() {
                return Task::none();
            }
            super::super::voice::cancel(app);
            let state = &mut app.voice.settings;
            let edit = state.edit.as_mut().unwrap();
            match event {
                Event::LocalLanguage(language) => {
                    edit.local.local_language = language;
                    state.resources.capability = None;
                    refresh_local(state)
                }
                Event::LocalVoice(voice) => {
                    edit.local.local_voice = voice;
                    Task::none()
                }
                _ => unreachable!(),
            }
        }
        Event::DisableHotkeys => {
            if app.voice.hotkeys.machine.active {
                super::super::voice::cancel(app);
            }
            // Stopping a local capability must work even if the gateway is offline.
            app.voice.hotkeys.stop_session();
            app.voice.settings.recording_hotkey = None;
            if let Some(edit) = &mut app.voice.settings.edit {
                edit.local.hotkeys.enabled = false;
            }
            match Preferences::path().and_then(|path| Preferences::disable_hotkeys_at(&path)) {
                Ok(()) => {
                    app.show_toast(crate::app::state::ToastKind::Success, "全局语音快捷键已停用。")
                }
                Err(error) => {
                    let error = format!("本次运行已停用全局热键，但停用设置保存失败：{error}");
                    app.voice.hotkeys.error = Some(error.clone());
                    app.show_error_toast(error)
                }
            }
        }
        Event::HotkeyPermission => {
            use crate::app::voice::permissions::{Permission, SystemPermissions, VoicePermissions};
            match VoicePermissions::new(SystemPermissions)
                .request_from_user_action(Permission::InputMonitoring)
            {
                Ok(()) => Task::none(),
                Err(error) => app.show_error_toast(error.to_string()),
            }
        }
        Event::Open => {
            if app.voice.settings.busy {
                return Task::none();
            }
            let endpoint = crate::app::gateway_client_endpoint();
            let state = &mut app.voice.settings;
            state.preview.take();
            state.edit = None;
            state.recording_hotkey = None;
            state.busy = true;
            state.generation = state.generation.wrapping_add(1);
            state.endpoint = Some(endpoint.clone());
            let generation = state.generation;
            Task::perform(io::load(endpoint), move |result| {
                message(Event::Loaded(generation, result))
            })
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
                    state.resources = loaded.resources;
                    state.cloud_ready = loaded.cloud_ready;
                    state.cloud_error = loaded.cloud_error;
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
            state.recording_hotkey = None;
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
            if let Err(error) = app.voice.hotkeys.check(&edit.local.hotkeys) {
                return app.show_error_toast(error);
            }
            if edit.local.engine == VoiceEngine::Cloud && !state.cloud_ready {
                return app.show_error_toast("云端设置尚未加载，请重新读取或选择 macOS 本机。");
            }
            if let Err(error) = edit.local.validate() {
                return app.show_error_toast(error);
            }
            let edit = edit.clone();
            let generation = state.generation;
            let cloud_ready = state.cloud_ready;
            state.preview.take();
            state.busy = true;
            Task::perform(
                async move { io::save(edit, endpoint, cloud_ready, &Preferences::path()?).await },
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
                    state.recording_hotkey = None;
                    state.endpoint = None;
                    app.voice.hotkeys.resume_after_save();
                    app.show_toast(
                        crate::app::state::ToastKind::Success,
                        "语音设置已保存，新音色将在下次语音聊天生效。",
                    )
                }
                Err(e) => app.show_error_toast(e),
            }
        }
        Event::Preview => {
            let chat_active = app.voice.chat_active();
            let state = &mut app.voice.settings;
            if state.busy || state.preview.is_some() {
                return Task::none();
            }
            if chat_active
                || app.voice.recordings.replay.is_some()
                || matches!(
                    app.voice.draft.phase,
                    super::super::voice_draft::Phase::Recording
                        | super::super::voice_draft::Phase::Transcribing
                )
            {
                return app.show_error_toast("请先结束当前语音活动再试听。");
            }
            let (Some(edit), Some(endpoint)) = (&state.edit, state.endpoint.clone()) else {
                return Task::none();
            };
            let preview = match edit.local.engine {
                VoiceEngine::Local => Preview::start_local(edit.local.local_voice.clone()),
                VoiceEngine::Cloud if state.cloud_ready => {
                    Preview::start(endpoint, edit.config.voice)
                }
                VoiceEngine::Cloud => Err("云端设置尚未加载，请检查网关连接。".into()),
            };
            match preview {
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
            if !app.voice.settings.busy
                && app.voice.settings.edit.is_some()
                && matches!(event, Event::Voice(_) | Event::Language(_) | Event::Microphone(_))
            {
                super::super::voice::cancel(app);
            }
            let state = &mut app.voice.settings;
            if state.busy {
                return Task::none();
            }
            let Some(edit) = &mut state.edit else {
                return Task::none();
            };
            match event {
                Event::HotkeysEnabled(value) => edit.local.hotkeys.enabled = value,
                Event::RecordHotkey(value) => {
                    state.recording_hotkey = value;
                    return iced::advanced::widget::operate(
                        iced::advanced::widget::operation::focusable::unfocus(),
                    );
                }
                Event::RecordedHotkey(value) => {
                    let Some(chat) = state.recording_hotkey else {
                        return Task::none();
                    };
                    if let Err(error) = value.validate() {
                        return app.show_error_toast(error);
                    }
                    let mut config = edit.local.hotkeys.clone();
                    if chat {
                        config.chat = Some(value);
                    } else {
                        config.dictation = Some(value);
                    }
                    // Both keys may still be unset while the dialog is being filled.
                    let mut validation = config.clone();
                    validation.enabled = false;
                    if let Err(error) = validation.validate() {
                        return app.show_error_toast(error);
                    }
                    edit.local.hotkeys = config;
                    state.recording_hotkey = None;
                }
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
                Event::RetainRecordings(value) => edit.local.retain_recordings = value,
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
fn refresh_local(state: &State) -> Task<Message> {
    let Some(edit) = state.edit.as_ref().filter(|edit| edit.local.engine == VoiceEngine::Local)
    else {
        return Task::none();
    };
    let language = edit.local.local_language.clone();
    let generation = state.generation;
    Task::perform(
        async move {
            let selected = language.clone();
            let resources =
                tokio::task::spawn_blocking(move || local_resources(selected.as_deref()))
                    .await
                    .unwrap_or_default();
            (language, resources)
        },
        move |(language, resources)| message(Event::Resources(generation, language, resources)),
    )
}
#[cfg(test)]
#[path = "voice_tests.rs"]
mod tests;
