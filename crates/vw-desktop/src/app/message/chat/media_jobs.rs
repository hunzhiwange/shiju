//! Session-scoped media composer and gateway job lifecycle.

mod composer;
mod files;

pub(crate) use self::composer::{
    can_send, composer_operation, creating_session, model_options, selected_model,
};
pub(super) use self::composer::{load_models, send};
use self::composer::{prepare_session, session_prepared};
use self::files::{asset_extension, download_asset, job_index, read_job_refs};
use super::{ChatMessage, media_shortcuts};
use crate::app::{App, Message, state::ToastKind};
use desktop_chat_input::input_panel::media_shortcuts::MediaMode;
use gateway_endpoint::GatewayEndpoint;
use iced::Task;
use serde::{Deserialize, Serialize};
use std::{
    collections::{HashMap, HashSet},
    path::Path,
    sync::Arc,
    time::Duration,
};
use vw_gateway_client::{
    GatewayClient,
    media::{
        MediaApiError, MediaAsset, MediaAssetSource, MediaErrorCode, MediaJob, MediaJobId,
        MediaJobState, MediaModel, MediaModels, MediaOperation, MediaResult, MediaSessionQuery,
    },
};

const POLL_INTERVAL: Duration = Duration::from_secs(2);
const MAX_POLL_FAILURES: u8 = 4;

#[derive(Default)]
pub(crate) struct State {
    epoch: u64,
    creating_session: Option<u64>,
    sessions: HashMap<String, Session>,
}

#[derive(Default)]
pub(crate) struct Session {
    pub(crate) catalog: Catalog,
    selected: HashMap<MediaOperation, ModelChoice>,
    jobs: HashMap<MediaJobId, JobEntry>,
    submitting: Option<MediaJobId>,
}

#[derive(Default)]
pub(crate) enum Catalog {
    #[default]
    Idle,
    Loading,
    Ready(Vec<MediaModel>),
    Failed(&'static str),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ModelChoice {
    deployment_id: String,
    model_id: String,
    name: String,
}

impl std::fmt::Display for ModelChoice {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} · {} · {}", self.name, self.deployment_id, self.model_id)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct SessionKey {
    epoch: u64,
    session_id: String,
}

impl SessionKey {
    fn active(app: &App) -> Option<Self> {
        Some(Self {
            epoch: app.media_jobs.epoch,
            session_id: app.session_state.active_session_id.clone()?,
        })
    }

    fn is_active(&self, app: &App) -> bool {
        self.epoch == app.media_jobs.epoch
            && app.session_state.active_session_id.as_ref() == Some(&self.session_id)
    }

    fn query(&self) -> MediaSessionQuery {
        MediaSessionQuery { session_id: self.session_id.clone() }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct JobRef {
    pub(crate) id: MediaJobId,
    pub(crate) operation: MediaOperation,
}

pub(crate) struct JobEntry {
    pub(crate) reference: JobRef,
    pub(crate) snapshot: Option<MediaJob>,
    pub(crate) error: Option<&'static str>,
    pub(crate) submitting: bool,
    pub(crate) canceling: bool,
    pub(crate) refreshing: bool,
    failures: u8,
    pub(crate) previews: HashMap<String, iced::widget::image::Handle>,
    pub(crate) asset_busy: HashSet<(String, AssetAction)>,
    // Keep verified local copies while the player can use them.
    files: HashMap<String, Arc<tempfile::TempPath>>,
}

impl JobEntry {
    fn new(reference: JobRef) -> Self {
        Self {
            reference,
            snapshot: None,
            error: None,
            submitting: false,
            canceling: false,
            refreshing: false,
            failures: 0,
            previews: HashMap::new(),
            asset_busy: HashSet::new(),
            files: HashMap::new(),
        }
    }

    fn accept(&mut self, scope: &MediaSessionQuery, job: MediaJob) -> bool {
        if job.session_id != scope.session_id
            || job.id != self.reference.id
            || job.operation != self.reference.operation
            || self.snapshot.as_ref().is_some_and(|current| {
                current.updated_at_ms > job.updated_at_ms
                    || (terminal(&current.state) && current.state != job.state)
                    || matches!(
                        (&current.state, &job.state),
                        (MediaJobState::Running { .. }, MediaJobState::Submitted)
                    )
            })
        {
            return false;
        }
        self.snapshot = Some(job);
        self.error = None;
        self.failures = 0;
        true
    }

    pub(crate) fn result(&self) -> Option<&MediaResult> {
        match &self.snapshot.as_ref()?.state {
            MediaJobState::Succeeded { result } => Some(result),
            _ => None,
        }
    }

    pub(crate) fn can_cancel(&self) -> bool {
        !self.canceling && self.snapshot.as_ref().is_some_and(|job| !terminal(&job.state))
    }
}

fn terminal(state: &MediaJobState) -> bool {
    matches!(
        state,
        MediaJobState::Succeeded { .. } | MediaJobState::Failed { .. } | MediaJobState::Canceled
    )
}

#[derive(Clone, Debug)]
pub(crate) struct Draft {
    editor: String,
    files: Vec<String>,
    navigation: u64,
}

impl Draft {
    fn matches(&self, editor: &str, files: &[String], navigation: u64) -> bool {
        self.navigation == navigation && self.editor == editor && self.files == files
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) enum AssetAction {
    Preview,
    Play,
    Save,
}

pub(crate) fn asset_actions(asset: &MediaAsset) -> [AssetAction; 2] {
    [
        match &asset.source {
            MediaAssetSource::Image { .. } => AssetAction::Preview,
            MediaAssetSource::Stream => AssetAction::Play,
        },
        AssetAction::Save,
    ]
}

#[derive(Clone, Debug)]
pub(crate) enum Event {
    PrepareSession,
    SessionPrepared {
        navigation: u64,
        epoch: u64,
        project: String,
        result: Result<Box<session_info::Info>, &'static str>,
    },
    RefreshModels,
    ModelsLoaded(SessionKey, Result<MediaModels, MediaApiError>),
    ModelSelected(SessionKey, MediaOperation, ModelChoice),
    HistoryLoaded(SessionKey, Result<Vec<JobRef>, &'static str>),
    Submitted {
        key: SessionKey,
        reference: JobRef,
        mode: MediaMode,
        draft: Draft,
        result: Result<MediaJob, SubmitError>,
    },
    Refresh(SessionKey, MediaJobId),
    Cancel(SessionKey, MediaJobId),
    Updated(SessionKey, MediaJobId, bool, Result<MediaJob, MediaApiError>),
    Asset(SessionKey, MediaJobId, String, AssetAction),
    AssetReady {
        key: SessionKey,
        id: MediaJobId,
        asset: String,
        action: AssetAction,
        result: Result<Option<LocalAsset>, &'static str>,
    },
}

#[derive(Clone, Debug)]
pub(crate) enum LocalAsset {
    Preview(iced::widget::image::Handle),
    Play(Arc<tempfile::TempPath>),
    Saved,
}

#[derive(Clone, Copy, Debug)]
pub(crate) enum SubmitError {
    Local(&'static str),
    Gateway(MediaApiError),
}

pub(crate) fn message(event: Event) -> Message {
    Message::Chat(ChatMessage::Media(event))
}

fn session_mut(app: &mut App, key: &SessionKey) -> Option<&mut Session> {
    if key.epoch != app.media_jobs.epoch {
        return None;
    }
    Some(app.media_jobs.sessions.entry(key.session_id.clone()).or_default())
}

pub(crate) fn active_session(app: &App) -> Option<(SessionKey, &Session)> {
    let key = SessionKey::active(app)?;
    let session = app.media_jobs.sessions.get(&key.session_id)?;
    Some((key, session))
}

pub(crate) fn active_jobs(app: &App) -> Option<(SessionKey, Vec<&JobEntry>)> {
    let (key, session) = active_session(app)?;
    let mut jobs: Vec<_> = session.jobs.values().collect();
    jobs.sort_by_key(|entry| {
        (entry.snapshot.as_ref().map_or(u64::MAX, |job| job.created_at_ms), entry.reference.id)
    });
    (!jobs.is_empty()).then_some((key, jobs))
}

pub(crate) fn gateway_changed(app: &mut App) -> Task<Message> {
    app.media_jobs = State { epoch: app.media_jobs.epoch.wrapping_add(1), ..State::default() };
    restore(app)
}

pub(crate) fn restore(app: &mut App) -> Task<Message> {
    let Some(key) = SessionKey::active(app) else { return Task::none() };
    let endpoint = crate::app::config::gateway_client_endpoint();
    let index = job_index(&endpoint, &key.session_id).filter(|path| path.is_dir());
    let has_active_entries =
        active_session(app).is_some_and(|(_, session)| !session.jobs.is_empty());
    let history = if index.is_some() || has_active_entries {
        Task::perform(
            async move {
                let result = tokio::task::spawn_blocking(move || {
                    index.map_or(Ok(vec![]), |path| {
                        read_job_refs(&path).map_err(|_| "无法读取媒体任务历史")
                    })
                })
                .await
                .unwrap_or(Err("无法读取媒体任务历史"));
                Event::HistoryLoaded(key, result)
            },
            message,
        )
    } else {
        Task::none()
    };
    if media_shortcuts::active_mode(app).is_some() {
        Task::batch([history, load_models(app)])
    } else {
        history
    }
}

fn submitted(
    app: &mut App,
    key: SessionKey,
    reference: JobRef,
    mode: MediaMode,
    draft: Draft,
    result: Result<MediaJob, SubmitError>,
) -> Task<Message> {
    let Some(session) = session_mut(app, &key) else { return Task::none() };
    if session.submitting != Some(reference.id) {
        return Task::none();
    }
    session.submitting = None;
    let Some(entry) = session.jobs.get_mut(&reference.id) else { return Task::none() };
    entry.submitting = false;
    match result {
        Ok(job) => {
            entry.accept(&key.query(), job);
        }
        Err(SubmitError::Gateway(MediaApiError::SubmissionUnknown { job_id }))
            if job_id == reference.id =>
        {
            entry.error = Some("媒体任务提交结果未知，正在查询");
        }
        Err(error) => {
            session.jobs.remove(&reference.id);
            return if key.is_active(app) {
                toast(
                    app,
                    match error {
                        SubmitError::Local(error) => error,
                        SubmitError::Gateway(error) => api_error(error),
                    },
                )
            } else {
                Task::none()
            };
        }
    }
    if key.is_active(app)
        && media_shortcuts::active_mode(app) == Some(mode)
        && app.current_session_runtime_ref().is_some_and(|runtime| {
            draft.matches(
                &runtime.input_editor.text(),
                &app.session_state.files,
                app.session_state.session_view_cache.navigation_generation,
            )
        })
    {
        app.current_session_runtime_mut().input_editor = iced::widget::text_editor::Content::new();
        app.session_state.input_text.clear();
        app.session_state.files.clear();
    }
    continue_poll(app, key, reference.id)
}

fn continue_poll(app: &mut App, key: SessionKey, id: MediaJobId) -> Task<Message> {
    if !key.is_active(app) {
        return Task::none();
    }
    let Some(entry) = session_mut(app, &key).and_then(|session| session.jobs.get(&id)) else {
        return Task::none();
    };
    if entry.canceling
        || entry.failures >= MAX_POLL_FAILURES
        || entry.snapshot.as_ref().is_some_and(|job| {
            terminal(&job.state) || job.state == MediaJobState::SubmissionUnknown
        })
    {
        return Task::none();
    }
    fetch_job(app, key, id, false, POLL_INTERVAL)
}

fn fetch_job(
    app: &mut App,
    key: SessionKey,
    id: MediaJobId,
    cancel: bool,
    delay: Duration,
) -> Task<Message> {
    if !key.is_active(app) {
        return Task::none();
    }
    let Some(entry) = session_mut(app, &key).and_then(|session| session.jobs.get_mut(&id)) else {
        return Task::none();
    };
    if entry.submitting || if cancel { !entry.can_cancel() } else { entry.refreshing } {
        return Task::none();
    }
    if cancel {
        entry.canceling = true;
    } else {
        entry.refreshing = true;
        if delay.is_zero() {
            entry.failures = 0;
        }
    }
    let reference = entry.reference;
    let endpoint = crate::app::config::gateway_client_endpoint();
    Task::perform(
        async move {
            if !delay.is_zero() {
                tokio::time::sleep(delay).await;
            }
            let result = match gateway_client(endpoint) {
                Ok(client) => request_update(&client, &key.query(), reference, cancel).await,
                Err(error) => Err(error),
            };
            Event::Updated(key, id, cancel, result)
        },
        message,
    )
}

async fn request_update(
    client: &GatewayClient,
    scope: &MediaSessionQuery,
    reference: JobRef,
    cancel: bool,
) -> Result<MediaJob, MediaApiError> {
    let job = if cancel {
        client.media_cancel(scope, reference.id).await?
    } else {
        client.media_job(scope, reference.id).await?
    };
    if job.operation != reference.operation {
        return Err(MediaApiError::InvalidResponse);
    }
    Ok(job)
}

fn updated(
    app: &mut App,
    key: SessionKey,
    id: MediaJobId,
    cancel: bool,
    result: Result<MediaJob, MediaApiError>,
) -> Task<Message> {
    let Some(entry) = session_mut(app, &key).and_then(|session| session.jobs.get_mut(&id)) else {
        return Task::none();
    };
    if cancel {
        entry.canceling = false;
    } else {
        entry.refreshing = false;
    }
    match result {
        Ok(job) => {
            entry.accept(&key.query(), job);
        }
        Err(error) if cancel => {
            let notice =
                if key.is_active(app) { toast(app, api_error(error)) } else { Task::none() };
            return Task::batch([notice, continue_poll(app, key, id)]);
        }
        Err(error) => {
            // A previously verified terminal result is replaced by a fresh GET on reopen.
            // Hide it if the gateway no longer authorizes access.
            if matches!(
                error,
                MediaApiError::Service(
                    MediaErrorCode::Unauthorized
                        | MediaErrorCode::Forbidden
                        | MediaErrorCode::NotFound
                )
            ) {
                entry.snapshot = None;
                entry.previews.clear();
                entry.files.clear();
            }
            entry.error = Some(api_error(error));
            entry.failures = if matches!(
                error,
                MediaApiError::Service(MediaErrorCode::Unauthorized | MediaErrorCode::Forbidden)
            ) {
                MAX_POLL_FAILURES
            } else {
                entry.failures.saturating_add(1)
            };
        }
    }
    continue_poll(app, key, id)
}

fn asset_task(
    app: &mut App,
    key: SessionKey,
    id: MediaJobId,
    asset_id: String,
    action: AssetAction,
) -> Task<Message> {
    if !key.is_active(app) {
        return Task::none();
    }
    let Some(entry) = session_mut(app, &key).and_then(|session| session.jobs.get_mut(&id)) else {
        return Task::none();
    };
    let Some(asset) = entry
        .result()
        .and_then(|result| result.assets.iter().find(|asset| asset.id == asset_id))
        .cloned()
    else {
        return Task::none();
    };
    if !asset_actions(&asset).contains(&action)
        || !entry.asset_busy.insert((asset_id.clone(), action))
    {
        return Task::none();
    }
    let endpoint = crate::app::config::gateway_client_endpoint();
    Task::perform(
        async move {
            let result = materialize_asset(endpoint, &key.query(), id, &asset, action).await;
            Event::AssetReady { key, id, asset: asset_id, action, result }
        },
        message,
    )
}

async fn materialize_asset(
    endpoint: GatewayEndpoint,
    scope: &MediaSessionQuery,
    id: MediaJobId,
    asset: &MediaAsset,
    action: AssetAction,
) -> Result<Option<LocalAsset>, &'static str> {
    let destination = if action == AssetAction::Save {
        let name = format!("shiju-{}.{}", asset.id, asset_extension(&asset.mime));
        let Some(file) = rfd::AsyncFileDialog::new().set_file_name(name).save_file().await else {
            return Ok(None);
        };
        Some(file.path().to_path_buf())
    } else {
        None
    };
    let client = gateway_client(endpoint).map_err(api_error)?;
    let file = download_asset(&client, scope, id, asset, destination.as_deref()).await?;
    if let Some(destination) = destination {
        file.persist(destination).map_err(|_| "无法写入媒体文件")?;
        return Ok(Some(LocalAsset::Saved));
    }
    if action == AssetAction::Preview {
        let image = tokio::task::spawn_blocking(move || files::image_preview(file.path()))
            .await
            .map_err(|_| "无法预览媒体图片")??;
        return Ok(Some(LocalAsset::Preview(image)));
    }
    Ok(Some(LocalAsset::Play(Arc::new(file.into_temp_path()))))
}

pub(super) fn update(app: &mut App, event: Event) -> Task<Message> {
    match event {
        Event::PrepareSession => prepare_session(app),
        Event::SessionPrepared { navigation, epoch, project, result } => {
            session_prepared(app, navigation, epoch, project, result)
        }
        Event::RefreshModels => load_models(app),
        Event::ModelsLoaded(key, result) => {
            if let Some(session) = session_mut(app, &key) {
                session.catalog = match result {
                    Ok(models) => Catalog::Ready(models.models),
                    Err(error) => Catalog::Failed(api_error(error)),
                };
            }
            Task::none()
        }
        Event::ModelSelected(key, operation, model) => {
            if key.is_active(app)
                && let Some(session) = session_mut(app, &key)
                && model_options(session, operation).contains(&model)
            {
                session.selected.insert(operation, model);
            }
            Task::none()
        }
        Event::HistoryLoaded(key, result) => {
            if !key.is_active(app) {
                return Task::none();
            }
            let refs = match result {
                Ok(refs) => refs,
                Err(error) => return toast(app, error),
            };
            let Some(session) = session_mut(app, &key) else { return Task::none() };
            for reference in refs {
                session.jobs.entry(reference.id).or_insert_with(|| JobEntry::new(reference));
            }
            let ids: Vec<_> = session.jobs.keys().copied().collect();
            Task::batch(
                ids.into_iter().map(|id| fetch_job(app, key.clone(), id, false, Duration::ZERO)),
            )
        }
        Event::Submitted { key, reference, mode, draft, result } => {
            submitted(app, key, reference, mode, draft, result)
        }
        Event::Refresh(key, id) => fetch_job(app, key, id, false, Duration::ZERO),
        Event::Cancel(key, id) => fetch_job(app, key, id, true, Duration::ZERO),
        Event::Updated(key, id, cancel, result) => updated(app, key, id, cancel, result),
        Event::Asset(key, id, asset, action) => asset_task(app, key, id, asset, action),
        Event::AssetReady { key, id, asset, action, result } => {
            let active = key.is_active(app);
            let Some(entry) = session_mut(app, &key).and_then(|session| session.jobs.get_mut(&id))
            else {
                return Task::none();
            };
            entry.asset_busy.remove(&(asset.clone(), action));
            if entry
                .result()
                .is_none_or(|result| !result.assets.iter().any(|item| item.id == asset))
            {
                return Task::none();
            }
            match result {
                Ok(Some(LocalAsset::Preview(image))) => {
                    entry.previews.insert(asset, image);
                }
                Ok(Some(LocalAsset::Play(file))) if active => {
                    let path: &Path = file.as_ref().as_ref();
                    if open::that_detached(path).is_err() {
                        return toast(app, "无法播放媒体文件");
                    }
                    entry.files.insert(asset, file);
                }
                Err(error) if active => return toast(app, error),
                _ => {}
            }
            Task::none()
        }
    }
}

pub(crate) fn operation_label(operation: MediaOperation) -> &'static str {
    match operation {
        MediaOperation::ImageGenerate => "生图",
        MediaOperation::ImageEdit => "改图",
        MediaOperation::VideoGenerate => "生视频",
        MediaOperation::SpeechSynthesize => "生语音",
        MediaOperation::AudioTranscribe => "识语音",
        MediaOperation::ImageUnderstand => "识图",
        MediaOperation::VideoUnderstand => "理解视频",
    }
}

fn gateway_client(endpoint: GatewayEndpoint) -> Result<GatewayClient, MediaApiError> {
    GatewayClient::new(endpoint).map_err(|_| MediaApiError::Transport)
}

fn toast(app: &mut App, error: &'static str) -> Task<Message> {
    app.show_localized_toast(ToastKind::Error, error, vec![])
}

pub(crate) fn service_error(error: MediaErrorCode) -> &'static str {
    match error {
        MediaErrorCode::Unauthorized | MediaErrorCode::Forbidden => "当前身份未获媒体服务授权",
        MediaErrorCode::NotFound => "找不到媒体任务或文件",
        MediaErrorCode::PayloadTooLarge => "媒体输入或产物超出大小限制",
        MediaErrorCode::UnsupportedMedia => "网关不支持此媒体格式",
        MediaErrorCode::Backpressure => "媒体服务繁忙，请稍后重试",
        MediaErrorCode::InvalidRequest => "网关拒绝了媒体请求",
        MediaErrorCode::Conflict => "媒体任务状态已变化，请刷新",
        MediaErrorCode::UpstreamFailed => "媒体供应商处理失败",
        MediaErrorCode::Unavailable => "媒体服务暂不可用",
    }
}

fn api_error(error: MediaApiError) -> &'static str {
    match error {
        MediaApiError::Service(error) => service_error(error),
        MediaApiError::Transport => "无法连接媒体网关",
        MediaApiError::InvalidResponse => "媒体网关响应无效",
        MediaApiError::SubmissionUnknown { .. } => "媒体任务提交结果未知，正在查询",
    }
}

#[cfg(test)]
#[path = "media_jobs_tests.rs"]
mod media_jobs_tests;
