//! Host actions for the media composer shortcuts.

use super::ChatMessage;
use crate::app::{App, Message};
use desktop_chat_input::input_panel::media_shortcuts::MediaMode;
#[cfg(not(target_arch = "wasm32"))]
use desktop_chat_input::input_panel::media_shortcuts::{
    self, AttachmentRule, MediaModeInput, Shortcut,
};
use iced::Task;

pub(crate) fn active_mode(app: &App) -> Option<MediaMode> {
    app.current_session_runtime_ref().and_then(|runtime| runtime.media_mode)
}

#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn view(app: &App) -> iced::Element<'static, Message> {
    media_shortcuts::view(
        crate::app::i18n::I18n::new(app.general_settings.desktop_locale),
        active_mode(app),
        |shortcut| {
            Message::Chat(match shortcut {
                Shortcut::AppStudio => ChatMessage::OpenMediaAppStudio,
                Shortcut::VideoEditor => ChatMessage::OpenMediaVideoEditor,
                Shortcut::Media(mode) => ChatMessage::MediaModeSelected(Some(mode)),
            })
        },
        Message::Chat(ChatMessage::MediaModeSelected(None)),
    )
}

pub(super) fn select(app: &mut App, mode: Option<MediaMode>) -> Task<Message> {
    let fallback_editor =
        app.current_session_runtime_ref().is_none().then(|| app.session_state.input_editor.clone());
    let runtime = app.current_session_runtime_mut();
    if let Some(editor) = fallback_editor {
        runtime.input_editor = editor;
    }
    runtime.media_mode = mode;
    app.input_suggestions.clear_files();
    app.input_suggestions.clear_slash();
    app.focus_area = crate::app::FocusArea::None;
    iced::widget::operation::focus(app.session_state.input_editor_id.clone())
}

#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn pick_attachments(app: &mut App, mode: MediaMode) -> Task<Message> {
    if mode.attachment_rule() == AttachmentRule::None {
        return app.show_localized_toast(
            crate::app::state::ToastKind::Info,
            "当前媒体模式仅接受文本",
            vec![],
        );
    }
    let label =
        crate::app::i18n::I18n::new(app.general_settings.desktop_locale).__("媒体文件").to_owned();
    let session_id = app.session_state.active_session_id.clone();
    let project_path = app.navigation_tabs.project_path.clone();
    iced::Task::perform(
        desktop_workspace::basic::pick_media_attachments(
            label,
            mode.attachment_extensions().collect(),
        ),
        move |paths| match paths {
            Some(paths) => Message::Chat(ChatMessage::MediaAttachmentsPicked {
                session_id: session_id.clone(),
                project_path: project_path.clone(),
                mode,
                paths,
            }),
            None => Message::None,
        },
    )
}

pub(super) fn attachments_picked(
    app: &mut App,
    session_id: Option<String>,
    project_path: Option<String>,
    mode: MediaMode,
    paths: Vec<String>,
) -> Task<Message> {
    if app.session_state.active_session_id == session_id
        && app.navigation_tabs.project_path == project_path
        && active_mode(app) == Some(mode)
    {
        crate::app::message::project::helpers::append_local_attachments(app, paths);
    }
    Task::none()
}

pub(super) fn open_app_studio(app: &mut App) -> Task<Message> {
    #[cfg(not(target_arch = "wasm32"))]
    return crate::app::message::shiju_apps::update(
        app,
        crate::app::message::ShijuAppsMessage::OpenStudio,
    );
    #[cfg(target_arch = "wasm32")]
    app.show_localized_toast(
        crate::app::state::ToastKind::Warning,
        "当前平台不支持识矩应用 WebView",
        vec![],
    )
}

pub(super) fn open_video_editor(app: &mut App) -> Task<Message> {
    use crate::apps::workflow::{self, WorkflowMessage};

    let open = workflow::ensure_initialized(app);
    let template = workflow::update(app, WorkflowMessage::OpenVideoTemplate);
    Task::batch([open, template])
}

#[cfg(test)]
#[path = "media_shortcuts_tests.rs"]
mod media_shortcuts_tests;
