//! Local media ownership; the host coordinates ordinary chat requests.
use super::voice_dictation::{self as dictation, Device};
use desktop_voice::{local::Cancellation, preferences::Preferences};
use gateway_endpoint::GatewayEndpoint;
pub use crate::components::voice_chat::Stage;
pub struct State {
    pub stage: Stage,
    pub show: bool,
    pub muted: bool,
    pub captions: Vec<(bool, String)>,
    pub notice: Option<String>,
    pub call: u128,
    pub round: u64,
    pub session: String,
    pub preferences: Preferences,
    pub native: Cancellation,
    pub cancel: Option<tokio::sync::watch::Sender<bool>>,
    pub device: Option<Device>,
    pub route: Option<dictation::Route>,
    pub request: Option<u64>,
    pub watermark: usize,
}
impl State {
    pub fn cancel_media(&mut self) {
        self.native.cancel();
        if let Some(tx) = self.cancel.take() {
            let _ = tx.send(true);
        }
        if let Some(device) = self.device.take()
            && let Ok(mut d) = device.lock()
        {
            drop(d.take());
        }
        self.route = None;
        self.round = self.round.wrapping_add(1);
        self.stage = Stage::Ready;
    }
    pub fn caption(&mut self, assistant: bool, text: String) {
        self.captions.push((assistant, text));
        while self.captions.len() > 40
            || self.captions.iter().map(|(_, s)| s.len()).sum::<usize>() > 65_536
        {
            self.captions.remove(0);
        }
    }
}
impl Drop for State {
    fn drop(&mut self) {
        self.cancel_media();
    }
}
#[derive(Clone)]
pub enum Event {
    Begin,
    Finish,
    ViewChat,
    Prepared(u128, u64, Result<dictation::Prepared, String>),
    Recognized(u128, u64, Result<String, String>),
    Spoken(u128, u64, Result<(), String>),
    Stopped(u128, Result<(), String>),
}
pub struct Stop {
    pub call: u128,
    pub endpoint: GatewayEndpoint,
    pub request: String,
    pub sent: bool,
}

/// Synchronous effects let the host preserve ordinary chat dispatch ordering.
pub enum Effect { Send(String), Error(String) }
fn failed(state: &mut State, error: String) -> (iced::Task<Event>, Option<Effect>) {
    state.cancel_media();
    state.notice = Some(error.clone());
    (iced::Task::none(), Some(Effect::Error(error)))
}
pub fn update_media(state: &mut State, endpoint: GatewayEndpoint, event: Event) -> (iced::Task<Event>, Option<Effect>) {
    use iced::Task;
    let task = match event {
        Event::ViewChat => { state.show = false; Task::none() }
        Event::Begin => {
            if state.stage != Stage::Ready || state.muted { return (Task::none(), None); }
            state.round = state.round.wrapping_add(1);
            state.notice = None;
            state.stage = Stage::Preparing;
            state.native = Cancellation::default();
            let (tx, rx) = tokio::sync::watch::channel(false);
            state.cancel = Some(tx);
            let (call, round) = (state.call, state.round);
            let future = dictation::prepare(state.preferences.clone(), endpoint, Some(state.session.clone()), round, rx, state.native.clone());
            Task::perform(async move {
                tokio::time::timeout(std::time::Duration::from_secs(20), future).await
                    .unwrap_or_else(|_| Err("准备本机录音超时，请重试。".into()))
            }, move |result| Event::Prepared(call, round, result))
        }
        Event::Prepared(call, round, result) => {
            if state.call != call || state.round != round || state.stage != Stage::Preparing { return (Task::none(), None); }
            match result {
                Ok(prepared) => { state.device = Some(prepared.device); state.route = Some(prepared.route); state.stage = Stage::Recording; Task::none() }
                Err(error) => return failed(state, error),
            }
        }
        Event::Finish => {
            if state.stage != Stage::Recording { return (Task::none(), None); }
            let (Some(device), Some(route)) = (state.device.take(), state.route.take()) else { return failed(state, "录音已失效，请重新开始。".into()); };
            if !matches!(route, dictation::Route::Local { .. }) { return failed(state, "本机语音路由不匹配，录音不会上传。".into()); }
            state.stage = Stage::Recognizing;
            let (call, round, cancel) = (state.call, state.round, state.native.clone());
            Task::perform(async move { dictation::transcribe(device, route, cancel).await.map(|(text, _)| text) }, move |result| Event::Recognized(call, round, result))
        }
        Event::Recognized(call, round, result) => {
            if state.call != call || state.round != round || state.stage != Stage::Recognizing { return (Task::none(), None); }
            let input = match result {
                Ok(text) if !text.trim().is_empty() && !text.trim_start().starts_with('/') => text.trim().to_owned(),
                Ok(_) => return failed(state, "未识别到普通聊天文字；斜杠命令请在聊天输入框中发送。".into()),
                Err(error) => return failed(state, error),
            };
            return (Task::none(), Some(Effect::Send(input)));
        }
        Event::Spoken(call, round, result) => {
            if state.call != call || state.round != round || state.stage != Stage::Speaking { return (Task::none(), None); }
            state.cancel_media();
            if let Err(error) = result { return failed(state, error); }
            Task::none()
        }
        Event::Stopped(..) => Task::none(),
    };
    (task, None)
}

pub fn model_completed<'a>(chat: &mut State, id: u64, session: &str, messages: impl Iterator<Item = (bool, &'a str)>) -> Result<iced::Task<Event>, String> {
    if chat.request != Some(id) || chat.session != session || chat.stage != Stage::WaitingModel { return Ok(iced::Task::none()); }
    chat.request = None;
    let prose = messages.skip(chat.watermark).filter(|(assistant, _)| *assistant).last()
        .map(|(_, content)| if content.len() > 65_536 { String::new() } else { desktop_chat_view::message_view::parse::assistant_speech_text(content) }).unwrap_or_default();
    if prose.is_empty() { return Err("回复没有可朗读的正文，请查看聊天。".into()); }
    let spoken = desktop_voice::local_conversation::spoken_prefix(&prose);
    chat.notice = (spoken != prose).then(|| "长回复仅朗读开头，完整内容请查看聊天。".into());
    chat.caption(true, prose); chat.stage = Stage::Speaking; chat.native = Cancellation::default();
    let (call, round) = (chat.call, chat.round);
    Ok(iced::Task::perform(desktop_voice::local_conversation::speak(spoken, chat.preferences.local_voice.clone(), chat.native.clone()), move |result| Event::Spoken(call, round, result)))
}
#[path = "voice_local_screen.rs"]
mod screen;
pub use screen::{screen_call, screen_input};

/// The host projects only the chat readiness conditions used by local voice.
pub struct Readiness {
    pub has_runtime: bool,
    pub requesting: bool,
    pub has_draft: bool,
    pub special_mode: bool,
    pub permissions_loading: bool,
    pub approval_pending: bool,
}
pub fn ready(state: Readiness) -> Result<(), &'static str> {
    if !state.has_runtime { return Err("请先选择聊天会话。"); }
    if state.requesting { return Err("请先完成或停止当前聊天请求及排队消息。"); }
    if state.has_draft { return Err("请先发送或清空聊天输入框中的草稿。"); }
    if state.special_mode { return Err("本机逐轮语音请使用普通聊天模型，并关闭任务与工作流模式。"); }
    if state.permissions_loading { return Err("会话正在加载或保存权限，请处理聊天提示后重试。"); }
    if state.approval_pending { return Err("请先处理当前聊天的审批或问题。"); }
    Ok(())
}

pub fn new(session: String, preferences: Preferences) -> State {
State {
        stage: Stage::Ready,
        show: true,
        muted: false,
        captions: Vec::new(),
        notice: None,
        call: rand::random(),
        round: 0,
        session,
        preferences,
        native: Cancellation::default(),
        cancel: None,
        device: None,
        route: None,
        request: None,
        watermark: 0,
    }
}

pub fn observe(chat: &mut State, request_active: bool) {
    let failed_request = chat.stage == Stage::WaitingModel
        && chat.request.is_some() && !request_active;
    let failed_capture = chat.stage == Stage::Recording
        && chat.device.as_ref().is_some_and(|device| {
            device.lock().ok().and_then(|mut d| d.as_mut()?.try_result()).is_some()
        });
    if failed_request || failed_capture {
        chat.cancel_media();
        chat.request = None;
        chat.notice = Some(
            if failed_capture {
                "录音已停止，请检查麦克风并在60秒内完成录音。"
            } else {
                "聊天请求已结束或失败，请查看聊天后重试。"
            }
            .into(),
        );
    }
}

pub fn mute(chat: &mut State) {
        chat.muted = !chat.muted;
        if chat.muted
            && matches!(chat.stage, Stage::Preparing | Stage::Recording | Stage::Recognizing)
        {
            chat.cancel_media();
        }
}
