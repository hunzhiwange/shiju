use super::files::{attachment_mime, forget_job, job_index, read_inputs, remember_job};
use super::{
    Catalog, Draft, Event, JobEntry, JobRef, ModelChoice, Session, SessionKey, State, SubmitError,
    active_session, api_error, gateway_client, media_shortcuts, message, service_error,
    session_mut, toast,
};
use crate::app::{App, Message};
use desktop_chat_input::input_panel::media_shortcuts::{MediaMode, MediaModeInput};
use iced::Task;
use std::path::{Path, PathBuf};
use vw_gateway_client::{
    GatewayClient, GatewaySessionCreateBody,
    media::{MediaApiError, MediaJob, MediaJobId, MediaOperation, MediaSessionQuery, MediaSubmit},
};

pub(crate) fn creating_session(app: &App) -> bool {
    app.media_jobs.creating_session
        == Some(app.session_state.session_view_cache.navigation_generation)
}

pub(super) fn prepare_session(app: &mut App) -> Task<Message> {
    if app.session_state.active_session_id.is_some()
        || creating_session(app)
        || media_shortcuts::active_mode(app).is_none()
    {
        return Task::none();
    }
    let Some(project) = app.navigation_tabs.project_path.clone() else {
        return toast(app, "未选择项目，无法创建会话");
    };
    let directory = app
        .current_session_runtime_ref()
        .and_then(|runtime| runtime.draft.as_ref())
        .filter(|draft| draft.project_path == project)
        .map_or_else(|| project.clone(), |draft| draft.directory.clone());
    let epoch = app.media_jobs.epoch;
    let navigation = app.session_state.session_view_cache.navigation_generation;
    app.media_jobs.creating_session = Some(navigation);
    let endpoint = crate::app::config::gateway_client_endpoint();
    let title =
        crate::app::i18n::I18n::new(app.general_settings.desktop_locale).__("媒体会话").to_owned();
    Task::perform(
        async move {
            let client = gateway_client(endpoint).map_err(api_error)?;
            let body = Some(GatewaySessionCreateBody { title: Some(title), ..Default::default() });
            client
                .session_create(&directory, &body)
                .await
                .map(Box::new)
                .map_err(|_| "无法创建媒体会话")
        },
        move |result| message(Event::SessionPrepared { navigation, epoch, project, result }),
    )
}

pub(super) fn session_prepared(
    app: &mut App,
    navigation: u64,
    epoch: u64,
    project: String,
    result: Result<Box<session_info::Info>, &'static str>,
) -> Task<Message> {
    if epoch != app.media_jobs.epoch {
        return Task::none();
    }
    if app.media_jobs.creating_session == Some(navigation) {
        app.media_jobs.creating_session = None;
    }
    let still_here = app.session_state.session_view_cache.navigation_generation == navigation
        && app.session_state.active_session_id.is_none()
        && app.navigation_tabs.project_path.as_ref() == Some(&project)
        && media_shortcuts::active_mode(app).is_some();
    let info = match result {
        Ok(info) => *info,
        Err(error) if still_here => return toast(app, error),
        Err(_) => return Task::none(),
    };
    let id = info.id.clone();
    if (MediaSessionQuery { session_id: id.clone() }).validate().is_err() {
        return if still_here { toast(app, "媒体网关响应无效") } else { Task::none() };
    }
    desktop_workspace::project_message::session::lifecycle::cache_project_session(
        &mut app.session_state.sessions,
        &mut app.workspace_state.project_sessions,
        project,
        info,
    );
    if !still_here {
        return Task::none();
    }
    // Keep the active editor and attachments, including changes made during creation.
    let runtime = app.current_session_runtime();
    app.session_state.session_runtime_states.insert(id.clone(), runtime);
    let blank = app.get_session_runtime_mut(crate::app::session::EMPTY_SESSION_ID);
    blank.input_editor = iced::widget::text_editor::Content::new();
    blank.media_mode = None;
    app.session_state.active_session_id = Some(id.clone());
    app.mark_active_session_viewed();
    app.sync_active_session_preferences();
    Task::batch([load_models(app), app.load_session_full_access(&id)])
}

pub(in crate::app::message::chat) fn load_models(app: &mut App) -> Task<Message> {
    let Some(key) = SessionKey::active(app) else { return Task::none() };
    let Some(session) = session_mut(app, &key) else { return Task::none() };
    if matches!(session.catalog, Catalog::Loading) {
        return Task::none();
    }
    session.catalog = Catalog::Loading;
    let endpoint = crate::app::config::gateway_client_endpoint();
    Task::perform(
        async move {
            let result = match gateway_client(endpoint) {
                Ok(client) => client.media_models(&key.query()).await,
                Err(error) => Err(error),
            };
            Event::ModelsLoaded(key, result)
        },
        message,
    )
}

pub(in crate::app::message::chat) fn send(app: &mut App, mode: MediaMode) -> Task<Message> {
    let Some(key) = SessionKey::active(app) else {
        return toast(app, "请先创建会话并加载媒体模型");
    };
    let Some(runtime) = app.current_session_runtime_ref() else { return Task::none() };
    if runtime.is_employee_chat() {
        return toast(app, "数字员工会话支持纯文本聊天");
    }
    if app.chat_view.active_session_view_state.ui_preparing
        && !app.chat_view.active_session_view_state.base_ready
    {
        return toast(app, "会话历史正在加载，请稍后发送");
    }
    let editor = runtime.input_editor.text();
    let prompt = editor.trim().to_owned();
    if let Err(error) = validate_prompt(mode, &prompt) {
        return toast(app, error);
    }
    let files = match attachments(app, mode) {
        Ok(files) => files,
        Err(error) => return toast(app, error),
    };
    let operation = match operation(mode, &files.iter().map(|(_, mime)| *mime).collect::<Vec<_>>())
    {
        Ok(operation) => operation,
        Err(error) => return toast(app, error),
    };
    let draft = Draft {
        editor,
        files: app.session_state.files.clone(),
        navigation: app.session_state.session_view_cache.navigation_generation,
    };
    let Some(session) = session_mut(app, &key) else { return Task::none() };
    if session.submitting.is_some() {
        return Task::none();
    }
    let Some(model) = selected_model(session, operation) else {
        return toast(app, "请选择可用的媒体模型");
    };
    let id = MediaJobId::new_v4();
    let reference = JobRef { id, operation };
    let mut entry = JobEntry::new(reference);
    entry.submitting = true;
    session.jobs.insert(id, entry);
    session.submitting = Some(id);
    let endpoint = crate::app::config::gateway_client_endpoint();
    let index = job_index(&endpoint, &key.session_id);
    let request = MediaSubmit {
        request_id: id,
        session_id: key.session_id.clone(),
        operation,
        deployment_id: model.deployment_id,
        model_id: model.model_id,
        text: (!prompt.is_empty()).then_some(prompt),
        inputs: vec![],
    };
    Task::perform(
        async move {
            let result = match gateway_client(endpoint) {
                Ok(client) => submit(&client, index, request, reference, files).await,
                Err(error) => Err(SubmitError::Gateway(error)),
            };
            Event::Submitted { key, reference, mode, draft, result }
        },
        message,
    )
}

pub(super) async fn submit(
    client: &GatewayClient,
    index: Option<PathBuf>,
    mut request: MediaSubmit,
    reference: JobRef,
    files: Vec<(PathBuf, &'static str)>,
) -> Result<MediaJob, SubmitError> {
    let index = index.ok_or(SubmitError::Local("无法保存媒体任务索引"))?;
    let saved_index = index.clone();
    request = tokio::task::spawn_blocking(move || {
        request.inputs = read_inputs(files).map_err(SubmitError::Local)?;
        request.validate().map_err(|error| SubmitError::Local(service_error(error)))?;
        remember_job(&saved_index, reference)
            .map_err(|_| SubmitError::Local("无法保存媒体任务索引"))?;
        Ok::<_, SubmitError>(request)
    })
    .await
    .map_err(|_| SubmitError::Local("无法准备媒体任务"))??;
    let result = client.media_submit(&request).await;
    if matches!(&result, Err(error) if !matches!(error, MediaApiError::SubmissionUnknown { .. })) {
        let _ = tokio::task::spawn_blocking(move || forget_job(&index, reference.id)).await;
    }
    result.map_err(SubmitError::Gateway)
}

pub(super) fn validate_prompt(mode: MediaMode, text: &str) -> Result<(), &'static str> {
    if mode != MediaMode::Transcription && text.trim().is_empty() {
        return Err("请输入媒体任务提示词");
    }
    Ok(())
}

fn attachments(app: &App, mode: MediaMode) -> Result<Vec<(PathBuf, &'static str)>, &'static str> {
    app.session_state
        .files
        .iter()
        .map(|name| {
            let path = Path::new(name);
            if !mode.accepts(path) {
                return Err("当前媒体模式不支持这些附件");
            }
            let mime = attachment_mime(path).ok_or("当前媒体模式不支持这些附件")?;
            Ok((path.to_path_buf(), mime))
        })
        .collect()
}

pub(super) fn operation(mode: MediaMode, mimes: &[&str]) -> Result<MediaOperation, &'static str> {
    use MediaOperation as Op;
    let images = mimes.iter().all(|mime| mime.starts_with("image/"));
    match (mode, mimes) {
        (MediaMode::Image, []) => Ok(Op::ImageGenerate),
        (MediaMode::Image, _) if images => Ok(Op::ImageEdit),
        (MediaMode::Video, [] | [_]) if images => Ok(Op::VideoGenerate),
        (MediaMode::Video, _) if images => Err("视频生成只接受一张参考图"),
        (MediaMode::Speech, []) => Ok(Op::SpeechSynthesize),
        (MediaMode::Vision, []) | (MediaMode::Transcription, []) => {
            Err("请添加当前媒体模式需要的附件")
        }
        (MediaMode::Vision, _) if images => Ok(Op::ImageUnderstand),
        (MediaMode::Vision, [mime]) if mime.starts_with("video/") => Ok(Op::VideoUnderstand),
        (MediaMode::Vision, _) => Err("请添加图片或单个视频"),
        (MediaMode::Transcription, [mime])
            if mime.starts_with("audio/") || mime.starts_with("video/") =>
        {
            Ok(Op::AudioTranscribe)
        }
        (MediaMode::Transcription, _) => Err("转写只接受单个音频或视频"),
        _ => Err("当前媒体模式不支持这些附件"),
    }
}

pub(crate) fn composer_operation(
    app: &App,
    mode: MediaMode,
) -> Result<MediaOperation, &'static str> {
    let files = attachments(app, mode)?;
    operation(mode, &files.iter().map(|(_, mime)| *mime).collect::<Vec<_>>())
}

pub(crate) fn model_options(session: &Session, operation: MediaOperation) -> Vec<ModelChoice> {
    let Catalog::Ready(models) = &session.catalog else { return vec![] };
    let mut choices = Vec::new();
    for model in models {
        if model.operations.contains(&operation)
            && !choices.iter().any(|choice: &ModelChoice| {
                choice.deployment_id == model.deployment_id && choice.model_id == model.model_id
            })
        {
            choices.push(ModelChoice {
                deployment_id: model.deployment_id.clone(),
                model_id: model.model_id.clone(),
                name: model.display_name.clone(),
            });
        }
    }
    choices
}

pub(crate) fn selected_model(session: &Session, operation: MediaOperation) -> Option<ModelChoice> {
    let options = model_options(session, operation);
    let selected = session.selected.get(&operation);
    options
        .iter()
        .find(|model| {
            selected.is_some_and(|selected| {
                selected.deployment_id == model.deployment_id && selected.model_id == model.model_id
            })
        })
        .cloned()
        .or_else(|| (options.len() == 1).then(|| options[0].clone()))
}

pub(crate) fn can_send(app: &App, mode: MediaMode, text: &str) -> bool {
    validate_prompt(mode, text).is_ok()
        && composer_operation(app, mode).is_ok_and(|operation| {
            active_session(app).is_some_and(|(_, session)| {
                session.submitting.is_none() && selected_model(session, operation).is_some()
            })
        })
}
