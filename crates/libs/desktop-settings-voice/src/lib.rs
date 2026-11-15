#![cfg(not(target_arch = "wasm32"))]
//! Transactional voice form. Every asynchronous result is scoped to one edit generation.
use desktop_voice::{

    preferences::{Edit, Preferences, VoiceEngine},
    preview::Preview,
};
use audio_capture::{InputDevice, input_devices};
type Message = Event;
use iced::Task;
use std::time::Duration;
use config_voice::{RecognitionLanguage, VoiceConfig, VoiceName};
use {gateway_endpoint::{GatewayEndpoint}, gateway_voice::{GatewayVoiceClient}};

#[path = "voice_io.rs"]
mod io;
pub use io::{LocalResources, resources as local_resources};

#[derive(Default)]
pub struct State {
    pub edit: Option<Edit>,
    pub devices: Vec<InputDevice>,
    pub dictionary: String,
    pub busy: bool,
    pub recording_hotkey: Option<bool>,
    pub preview: Option<Preview>,
    pub resources: LocalResources,
    pub cloud_ready: bool,
    pub cloud_error: Option<String>,
    permission_cancel: desktop_voice::local::Cancellation,
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
    Onboarding,
    Noop,
    Feedback(bool, String),
    Engine(VoiceEngine),
    LocalLanguage(Option<String>),
    LocalVoice(Option<String>),
    Permission(desktop_voice::permissions::Permission),
    RefreshLocal,
    PermissionFinished(u64, Result<desktop_voice::permissions::PermissionStatus, String>),
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
    RecordedHotkey(desktop_voice::hotkeys::Shortcut),
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
    event
}
fn same_endpoint(a: &GatewayEndpoint, b: &GatewayEndpoint) -> bool {
    a.base_url() == b.base_url()
    && a.tenant_scope() == b.tenant_scope()
    && a.auth.as_ref().and_then(|a| a.skey.as_ref())
    == b.auth.as_ref().and_then(|a| a.skey.as_ref())
}
pub fn invalidate(state: &mut State, current: &GatewayEndpoint, visible: bool) {
    if state.endpoint.as_ref().is_some_and(|old| !same_endpoint(old, current))
    || (!visible && state.endpoint.is_some())
    {
        state.preview.take();
        state.edit = None;
        state.recording_hotkey = None;
        state.endpoint = None;
        state.busy = false;
        state.permission_cancel.cancel();
        state.generation = state.generation.wrapping_add(1);
    }
}
fn tick(generation: u64) -> Task<Message> {
    Task::perform(async { tokio::time::sleep(Duration::from_millis(100)).await }, move |_| Event::Tick(generation))
}
pub fn update(state: &mut State, hotkeys: &mut desktop_voice::hotkeys::Runtime, preview_blocked: bool, endpoint: GatewayEndpoint, preference_path: Result<std::path::PathBuf, String>, event: Event) -> Task<Message> {
    match event {
        Event::Feedback(_, _) | Event::Onboarding | Event::Noop => Task::none(),
        Event::Permission(permission) => {
            use desktop_voice::permissions::{SystemPermissions, VoicePermissions};
            if state.busy || state.edit.is_none() {
                return Task::none();
            }
            state.permission_cancel.cancel();
            state.permission_cancel = Default::default();
            state.generation = state.generation.wrapping_add(1);
            let cancel = state.permission_cancel.clone();
            let generation = state.generation;
            Task::perform(
                async move {
                    tokio::task::spawn_blocking(move || {
                            VoicePermissions::new(SystemPermissions)
                            .wait_from_user_action(permission, Duration::from_secs(60), || {
                                    cancel.is_cancelled()
                            })
                            .map_err(|error| error.to_string())
                    })
                    .await
                    .unwrap_or_else(|_| Err("系统授权任务异常结束。".into()))
                },
                move |result| message(Event::PermissionFinished(generation, result)),
            )
        }
        Event::PermissionFinished(generation, result) => {
            if state.generation != generation || state.edit.is_none() {
                return Task::none();
            }
            let refresh = refresh_local(state);
            let feedback = match result {
                Ok(desktop_voice::permissions::PermissionStatus::Granted) => Task::none(),
                Ok(status) => {
                    use desktop_voice::permissions::PermissionStatus;
                    feedback_error(match status {
                            PermissionStatus::Restricted => {
                                "系统策略限制了此权限，请检查设备管理或屏幕使用时间。"
                            }
                            PermissionStatus::Unsupported => "当前平台不支持此系统权限。",
                            _ => "系统权限尚未允许，请在隐私与安全性设置中授权后返回。",
                    })
                }
                Err(error) => feedback_error(format!("申请系统权限失败：{error}")),
            };
            Task::batch([refresh, feedback])
        }
        Event::RefreshLocal => refresh_local(&state),
        Event::Resources(generation, language, resources) => {
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
                    feedback_error(error)
                }
            }
        }
        Event::Engine(engine) => {
            if state.busy || state.edit.is_none() {
                return Task::none();
            }
            if engine == VoiceEngine::Local && !cfg!(target_os = "macos") {
                return feedback_error("本机语音当前仅支持 macOS。");
            }
            state.edit.as_mut().unwrap().local.engine = engine;
            state.permission_cancel.cancel();
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
            if state.busy || state.edit.is_none() {
                return Task::none();
            }
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
            // Stopping a local capability must work even if the gateway is offline.
            hotkeys.stop_session();
            state.recording_hotkey = None;
            if let Some(edit) = &mut state.edit {
                edit.local.hotkeys.enabled = false;
            }
            match preference_path.and_then(|path| Preferences::disable_hotkeys_at(&path)) {
                Ok(()) => {
                    success("全局语音快捷键已停用。")
                }
                Err(error) => {
                    let error = format!("本次运行已停用全局热键，但停用设置保存失败：{error}");
                    hotkeys.error = Some(error.clone());
                    feedback_error(error)
                }
            }
        }
        Event::HotkeyPermission => {
            use desktop_voice::permissions::{Permission, SystemPermissions, VoicePermissions};
            match VoicePermissions::new(SystemPermissions)
            .request_from_user_action(Permission::InputMonitoring)
            {
                Ok(()) => Task::none(),
                Err(error) => feedback_error(error.to_string()),
            }
        }
        Event::Open => {
            if state.busy {
                return Task::none();
            }
            state.preview.take();
            state.edit = None;
            state.recording_hotkey = None;
            state.busy = true;
            state.permission_cancel.cancel();
            state.generation = state.generation.wrapping_add(1);
            state.endpoint = Some(endpoint.clone());
            let generation = state.generation;
            Task::perform(io::load(endpoint, preference_path), move |result| {
                    message(Event::Loaded(generation, result))
            })
        }
        Event::Loaded(generation, result) => {
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
                        feedback_error(warning)
                    } else {
                        Task::none()
                    }
                }
                Err(error) => feedback_error(error),
            }
        }
        Event::Cancel => {
            if state.busy {
                return Task::none();
            }
            state.preview.take();
            state.recording_hotkey = None;
            if let Some(edit) = state.edit.take() {
                let _ = edit.cancel();
            }
            state.endpoint = None;
            state.permission_cancel.cancel();
            state.generation = state.generation.wrapping_add(1);
            Task::none()
        }
        Event::Save => {
            if state.busy {
                return Task::none();
            }
            let (Some(edit), Some(endpoint)) = (&state.edit, state.endpoint.clone()) else {
                return Task::none();
            };
            if let Err(error) = hotkeys.check(&edit.local.hotkeys) {
                return feedback_error(error);
            }
            if edit.local.engine == VoiceEngine::Cloud && !state.cloud_ready {
                return feedback_error("云端设置尚未加载，请重新读取或选择 macOS 本机。");
            }
            if let Err(error) = edit.local.validate() {
                return feedback_error(error);
            }
            let edit = edit.clone();
            let generation = state.generation;
            let cloud_ready = state.cloud_ready;
            state.preview.take();
            state.busy = true;
            Task::perform(
                async move { io::save(edit, endpoint, cloud_ready, &preference_path?).await },
                move |result| message(Event::Saved(generation, result)),
            )
        }
        Event::Saved(generation, result) => {
            if generation != state.generation || state.endpoint.is_none() {
                return Task::none();
            }
            state.busy = false;
            match result {
                Ok(()) => {
                    state.edit = None;
                    state.recording_hotkey = None;
                    state.endpoint = None;
                    hotkeys.resume_after_save();
                    success(
                        "语音设置已保存，新音色将在下次语音聊天生效。",
                    )
                }
                Err(e) => feedback_error(e),
            }
        }
        Event::Preview => {
            if state.busy || state.preview.is_some() { return Task::none(); }
            if preview_blocked { return feedback_error("请先结束当前语音活动再试听。"); }
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
                Err(e) => feedback_error(e),
            }
        }
        Event::StopPreview => {
            state.preview.take();
            Task::none()
        }
        Event::Tick(generation) => {
            if generation != state.generation {
                return Task::none();
            }
            if let Some(result) = state.preview.as_ref().and_then(Preview::result) {
                state.preview.take();
                return match result {
                    Ok(()) => Task::none(),
                    Err(e) => feedback_error(e),
                };
            }
            if state.preview.is_some() { tick(generation) } else { Task::none() }
        }
        event => {
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
                        return feedback_error(error);
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
                        return feedback_error(error);
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
                        return feedback_error(e);
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
#[path = "tests.rs"]
mod tests;

fn feedback_error(value: impl Into<String>) -> Task<Event> { Task::done(Event::Feedback(false, value.into())) }
fn success(value: impl Into<String>) -> Task<Event> { Task::done(Event::Feedback(true, value.into())) }
/// Shell cancels cross-domain activity before mutating the selected route.
pub fn cancels_activity(state: &State, hotkeys_active: bool, event: &Event) -> bool {
    match event {
        Event::DisableHotkeys => hotkeys_active,
        Event::Engine(engine) => !state.busy && state.edit.is_some() && (*engine != VoiceEngine::Local || cfg!(target_os = "macos")),
        Event::LocalLanguage(_) | Event::LocalVoice(_) | Event::Voice(_) | Event::Language(_) | Event::Microphone(_) => !state.busy && state.edit.is_some(),
        _ => false,
    }
}

pub mod view;
