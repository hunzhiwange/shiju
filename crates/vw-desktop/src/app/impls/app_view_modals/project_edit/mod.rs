//! 项目编辑领域视图的宿主消息映射。
use super::{App, Message};
use iced::Element;
use desktop_workspace::project_edit_view as domain;
use crate::app::message::ProjectMessage;
use desktop_workspace::project_message::edit::Event;
fn map(message: domain::Message) -> Message {
    match message {
        domain::Message::Project(event) => Message::Project(match event {
            Event::ProjectToolsMenuToggled(value) => ProjectMessage::ProjectToolsMenuToggled(value),
            Event::CurrentProjectMenuToggled(value) => ProjectMessage::CurrentProjectMenuToggled(value),
            Event::ProjectToolsMenuClosed => ProjectMessage::ProjectToolsMenuClosed,
            Event::ProjectEditOpened(value) => ProjectMessage::ProjectEditOpened(value),
            Event::ProjectEditTabSelected(value) => ProjectMessage::ProjectEditTabSelected(value),
            Event::ProjectEditNameChanged(value) => ProjectMessage::ProjectEditNameChanged(value),
            Event::ProjectEditIconChanged(value) => ProjectMessage::ProjectEditIconChanged(value),
            Event::ProjectEditIconHovered(value) => ProjectMessage::ProjectEditIconHovered(value),
            Event::ProjectEditIconPickFile => ProjectMessage::ProjectEditIconPickFile,
            Event::ProjectEditIconFilePicked(value) => ProjectMessage::ProjectEditIconFilePicked(value),
            Event::ProjectEditIconColorChanged(value) => ProjectMessage::ProjectEditIconColorChanged(value),
            Event::ProjectEditIconColorPresetSelected(value) => ProjectMessage::ProjectEditIconColorPresetSelected(value),
            Event::ProjectEditIconColorPickerToggled => ProjectMessage::ProjectEditIconColorPickerToggled,
            Event::ProjectEditIconColorPickerClosed => ProjectMessage::ProjectEditIconColorPickerClosed,
            Event::ProjectEditIconColorFormatChanged(value) => ProjectMessage::ProjectEditIconColorFormatChanged(value),
            Event::ProjectEditStartScriptChanged(value) => ProjectMessage::ProjectEditStartScriptChanged(value),
            Event::ProjectEditStartScriptEditorAction(value) => ProjectMessage::ProjectEditStartScriptEditorAction(value),
            Event::ProjectEditWorktreeToggled(value) => ProjectMessage::ProjectEditWorktreeToggled(value),
            Event::ProjectEditTaskBoardAutoRefreshToggled(value) => ProjectMessage::ProjectEditTaskBoardAutoRefreshToggled(value),
            Event::ProjectEditSessionAutoRefreshToggled(value) => ProjectMessage::ProjectEditSessionAutoRefreshToggled(value),
            Event::ProjectEditMaxConcurrentInputChanged(value) => ProjectMessage::ProjectEditMaxConcurrentInputChanged(value),
            Event::ProjectEditSessionRefreshIntervalSecondsInputChanged(value) => ProjectMessage::ProjectEditSessionRefreshIntervalSecondsInputChanged(value),
            Event::ProjectEditTaskBoardRefreshIntervalSecondsInputChanged(value) => ProjectMessage::ProjectEditTaskBoardRefreshIntervalSecondsInputChanged(value),
            Event::ProjectEditTaskBoardSchedulerTickIntervalSecondsInputChanged(value) => ProjectMessage::ProjectEditTaskBoardSchedulerTickIntervalSecondsInputChanged(value),
            Event::ProjectEditTaskBoardAutoPromoteTickIntervalSecondsInputChanged(value) => ProjectMessage::ProjectEditTaskBoardAutoPromoteTickIntervalSecondsInputChanged(value),
            Event::ProjectEditFailedRetryMinutesInputChanged(value) => ProjectMessage::ProjectEditFailedRetryMinutesInputChanged(value),
            Event::ProjectEditFailedRetryMaxAttemptsInputChanged(value) => ProjectMessage::ProjectEditFailedRetryMaxAttemptsInputChanged(value),
            Event::ProjectEditRunningTimeoutMinutesInputChanged(value) => ProjectMessage::ProjectEditRunningTimeoutMinutesInputChanged(value),
            Event::ProjectEditPrSubmittedStallTimeoutSecondsInputChanged(value) => ProjectMessage::ProjectEditPrSubmittedStallTimeoutSecondsInputChanged(value),
            Event::ProjectEditMaxConcurrentChanged(value) => ProjectMessage::ProjectEditMaxConcurrentChanged(value),
            Event::ProjectEditSessionRefreshIntervalSecondsChanged(value) => ProjectMessage::ProjectEditSessionRefreshIntervalSecondsChanged(value),
            Event::ProjectEditTaskBoardRefreshIntervalSecondsChanged(value) => ProjectMessage::ProjectEditTaskBoardRefreshIntervalSecondsChanged(value),
            Event::ProjectEditTaskBoardSchedulerTickIntervalSecondsChanged(value) => ProjectMessage::ProjectEditTaskBoardSchedulerTickIntervalSecondsChanged(value),
            Event::ProjectEditTaskBoardAutoPromoteTickIntervalSecondsChanged(value) => ProjectMessage::ProjectEditTaskBoardAutoPromoteTickIntervalSecondsChanged(value),
            Event::ProjectEditFailedRetryMinutesChanged(value) => ProjectMessage::ProjectEditFailedRetryMinutesChanged(value),
            Event::ProjectEditFailedRetryMaxAttemptsChanged(value) => ProjectMessage::ProjectEditFailedRetryMaxAttemptsChanged(value),
            Event::ProjectEditRunningTimeoutMinutesChanged(value) => ProjectMessage::ProjectEditRunningTimeoutMinutesChanged(value),
            Event::ProjectEditPrSubmittedStallTimeoutSecondsChanged(value) => ProjectMessage::ProjectEditPrSubmittedStallTimeoutSecondsChanged(value),
            Event::ProjectEditRecycleWorktreeOnTaskFinishToggled(value) => ProjectMessage::ProjectEditRecycleWorktreeOnTaskFinishToggled(value),
            Event::ProjectEditWorkflowIdChanged(value) => ProjectMessage::ProjectEditWorkflowIdChanged(value),
            Event::ProjectEditSaved => ProjectMessage::ProjectEditSaved,
            Event::ProjectEditRuntimeSaved(value) => ProjectMessage::ProjectEditRuntimeSaved(value),
            Event::ProjectEditCanceled => ProjectMessage::ProjectEditCanceled,
            Event::RecentRevealPressed(value) => ProjectMessage::RecentRevealPressed(value),
            Event::RecentRemovePressed(value) => ProjectMessage::RecentRemovePressed(value),
            Event::RecentRemoveCanceled => ProjectMessage::RecentRemoveCanceled,
            Event::RecentRemoveConfirmed(value) => ProjectMessage::RecentRemoveConfirmed(value),
        }),
        domain::Message::OpenWorkflowManager => Message::View(crate::app::message::view::ViewMessage::OpenWorkflowManager),
        domain::Message::Copy(text) => Message::CopyCode(text),
        domain::Message::Append(text) => crate::app::components::text_editor_context_menu::text_context_message(desktop_ui_controls::text_editor_context_menu::TextContextEffect::Append(text)),
        domain::Message::OpenUrl(url) => crate::app::components::text_editor_context_menu::text_context_message(desktop_ui_controls::text_editor_context_menu::TextContextEffect::OpenUrl(url)),
        domain::Message::None => Message::None,
    }
}
// The selectable help widget keeps the host clipboard effects explicit.
#[derive(Clone)]
struct HelpMessage(domain::Message);
impl desktop_chat_view::message_view::markdown_selection::SelectionMessage for HelpMessage {
    fn none() -> Self { Self(domain::Message::None) }
    fn copy(text: String) -> Self { Self(domain::Message::Copy(text)) }
    fn context(effect: desktop_ui_controls::text_editor_context_menu::TextContextEffect) -> Self { Self(match effect { desktop_ui_controls::text_editor_context_menu::TextContextEffect::Copy(text) => domain::Message::Copy(text), desktop_ui_controls::text_editor_context_menu::TextContextEffect::Append(text) => domain::Message::Append(text), desktop_ui_controls::text_editor_context_menu::TextContextEffect::OpenUrl(url) => domain::Message::OpenUrl(url) }) }
    fn selection(_: String, _: usize, _: usize, _: Option<String>) -> Self { Self::none() }
}
pub(crate) fn with_project_edit<'a>(app: &'a App, root: Element<'a, Message>) -> Element<'a, Message> {
    // The domain owns rendering; only the borrowed input projection lives in the shell.
    domain::with_project_edit(domain_view(app), root, map)
}
fn domain_view(app: &App) -> domain::View<'_> {
    domain::View {
        project_editor: &app.project_editor,
        desktop_locale: app.desktop_locale,
        app_theme: &app.app_theme,
        workflow_manager: domain::LifecycleStatus {
            loading: app.workflow_manager.lifecycles_loading(),
            loaded: app.workflow_manager.lifecycles_loaded(),
            error: app.workflow_manager.lifecycles_error(),
        },
        help: |title, body, color, i18n| desktop_chat_view::message_view::markdown_selection::selectable_titled_text::<HelpMessage>(title, body, color, i18n).map(|event| event.0),
    }
}
#[cfg(test)]
mod general_tab;
