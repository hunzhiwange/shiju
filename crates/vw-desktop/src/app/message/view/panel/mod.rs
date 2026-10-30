//! 处理视图面板的外部打开、弹出层与使用量展示交互。
//!
//! 注释说明当前文件的职责边界，帮助调用方理解数据流与错误传播，
//! 不改变任何运行时行为。

mod apps;
mod external;
mod popovers;
mod settings;
mod tabs;
mod update_notifications;
mod usage;
mod web;
pub(crate) mod work_browser;
mod work_sidebar;
mod work_workspace;
mod workspace_mode;

use super::ViewMessage;
use crate::app::{App, Message};
use iced::Task;

/// 执行 update 对应的领域操作。
///
/// 参数由调用方提供，函数在当前模块的状态边界内完成处理。
/// 返回值表达处理结果；失败时保留错误信息给上层界面或调度逻辑。
pub fn update(app: &mut App, message: ViewMessage) -> Task<Message> {
    match message {
        ViewMessage::AutoCheckAppUpdate
        | ViewMessage::AutoCheckAppUpdateFinished(..)
        | ViewMessage::SkipAppUpdate
        | ViewMessage::SkipAppUpdateSaved(..) => update_notifications::update(app, message),
        #[cfg(not(target_arch = "wasm32"))]
        msg @ ViewMessage::AppUpdatePrepared(_) => settings::update(app, msg),
        ViewMessage::ToggleWorkspaceModePopover
        | ViewMessage::CloseWorkspaceModePopover
        | ViewMessage::WorkspaceModeSelected(_) => workspace_mode::update(app, message),

        ViewMessage::WorkResourceModeSelected(_)
        | ViewMessage::WorkResourcesToggled
        | ViewMessage::WorkSplitRatioChanged(_)
        | ViewMessage::WorkResourceFullscreenToggled
        | ViewMessage::WorkResourceHalfFullscreenToggled
        | ViewMessage::WorkSpaceToggled(_) => work_workspace::update(app, message),

        ViewMessage::WorkSidebarToggled
        | ViewMessage::WorkSidebarFilterToggled
        | ViewMessage::WorkSidebarFilterClosed
        | ViewMessage::WorkSidebarMoreToggled
        | ViewMessage::WorkSidebarMoreOpened
        | ViewMessage::WorkSidebarMoreClosed
        | ViewMessage::WorkSidebarProjectNavHovered(_)
        | ViewMessage::WorkSidebarFilterScopeSelected(_)
        | ViewMessage::WorkSidebarFilterStatusSelected(_)
        | ViewMessage::WorkSidebarFilterTimeSelected(_)
        | ViewMessage::WorkSidebarFilterReset
        | ViewMessage::WorkSidebarTabSelected(_)
        | ViewMessage::WorkSidebarLoadMorePressed
        | ViewMessage::WorkSidebarSpaceTasksLoadMorePressed(_)
        | ViewMessage::WorkSpacePickerToggled
        | ViewMessage::WorkSpacePickerClosed => work_sidebar::update(app, message),

        ViewMessage::WorkBrowserAddressChanged(_)
        | ViewMessage::WorkBrowserNavigate
        | ViewMessage::WorkBrowserBack
        | ViewMessage::WorkBrowserForward
        | ViewMessage::WorkBrowserReload
        | ViewMessage::WorkBrowserClose
        | ViewMessage::WorkBrowserRuntimeTick => work_browser::update(app, message),

        ViewMessage::ToggleSettingsPanel
        | ViewMessage::ProjectFileNewSession
        | ViewMessage::ProjectFileNewProject
        | ViewMessage::ProjectFileShowSessions
        | ViewMessage::ProjectFileShowProjects
        | ViewMessage::ProjectFileSaveAll
        | ViewMessage::ToggleSystemSettings
        | ViewMessage::OpenSystemSettingsTab(_)
        | ViewMessage::OpenSystemSettingsModelDetail(_, _)
        | ViewMessage::ToggleAboutModal
        | ViewMessage::RestartApp
        | ViewMessage::RestartAppFinished(_)
        | ViewMessage::InstallCliTool
        | ViewMessage::RunInstallCliTool
        | ViewMessage::CheckCliToolUpdate
        | ViewMessage::CliCommandsTabSelected(_)
        | ViewMessage::CheckCliToolUpdateFinished(..)
        | ViewMessage::OpenAppUpdateModal
        | ViewMessage::CheckAppUpdate
        | ViewMessage::CheckAppUpdateFinished(..)
        | ViewMessage::AppUpdateSourceSelected(_)
        | ViewMessage::AppUpdateSourceSaved(_)
        | ViewMessage::OpenAppInstaller
        | ViewMessage::RunAppUpdate
        | ViewMessage::AppUpdateProgressTick
        | ViewMessage::AppUpdateFinished(_)
        | ViewMessage::InstallCliToolFinished(_)
        | ViewMessage::CloseInstallCliModal
        | ViewMessage::AutoMaxToggled(_)
        | ViewMessage::ToggleDiffPanel
        | ViewMessage::ToggleGitDiffSummary
        | ViewMessage::ToggleTerminalPanel
        | ViewMessage::FileManagerPanelVisible(_)
        | ViewMessage::OpenTerminalPressed
        | ViewMessage::ToggleMenu(_)
        | ViewMessage::MenuAction(_)
        | ViewMessage::MenuHovered(_)
        | ViewMessage::GatewayServicesTabSelected(_) => settings::update(app, message),

        ViewMessage::ToggleModelPopover
        | ViewMessage::ToggleModePopover
        | ViewMessage::ToggleSendModePopover
        | ViewMessage::ToggleFilePopover
        | ViewMessage::ToggleAcpPopover
        | ViewMessage::ToggleReasoningEffortPopover
        | ViewMessage::ToggleUsagePopover
        | ViewMessage::ToggleSessionToolSelectorPopover
        | ViewMessage::ToggleSessionActionsPopover
        | ViewMessage::ToggleExecutorPopover
        | ViewMessage::ClosePopovers
        | ViewMessage::CloseModelPopover
        | ViewMessage::CloseModePopover
        | ViewMessage::CloseSendModePopover
        | ViewMessage::CloseFilePopover
        | ViewMessage::CloseAcpPopover
        | ViewMessage::CloseReasoningEffortPopover
        | ViewMessage::CloseUsagePopover
        | ViewMessage::CloseExecutorPopover
        | ViewMessage::ModelPopoverHoverChanged(_)
        | ViewMessage::SelectChatSendBehavior(_) => popovers::update(app, message),

        ViewMessage::GoHome
        | ViewMessage::TabSelected(_)
        | ViewMessage::TabClosed(_)
        | ViewMessage::TabHovered(_)
        | ViewMessage::HomeAppsBarScrollChanged(_)
        | ViewMessage::HomeAppsBarPrev
        | ViewMessage::HomeAppsBarNext => tabs::update(app, message),

        ViewMessage::OpenApps
        | ViewMessage::OpenUsage
        | ViewMessage::OpenKnowledge
        | ViewMessage::OpenWorkflowTool
        | ViewMessage::OpenWorkflowManager
        | ViewMessage::AppsOpenMostRecent
        | ViewMessage::AppsSearchChanged(_) => apps::update(app, message),

        ViewMessage::UsageModelInfoLoaded(_)
        | ViewMessage::UsageStepToggled(_)
        | ViewMessage::UsageTurnStepToggled(_, _)
        | ViewMessage::UsageStepRawLoaded(_, _, _)
        | ViewMessage::UsageTurnStepRawLoaded(_, _, _, _)
        | ViewMessage::UsageStepRawOpen(_)
        | ViewMessage::UsageTurnStepRawOpen(_, _)
        | ViewMessage::UsageStepRawClose => usage::update(app, message),

        ViewMessage::OpenWebUrl(_)
        | ViewMessage::OpenWebUrlWithTitle(_, _)
        | ViewMessage::OpenWebUrlWithTitleAndSize(_, _, _, _)
        | ViewMessage::OpenUrlExternal(_)
        | ViewMessage::ToggleWebLinksMenu
        | ViewMessage::WebBookmarkTitleChanged(_)
        | ViewMessage::WebBookmarkUrlChanged(_)
        | ViewMessage::WebBookmarkWidthChanged(_)
        | ViewMessage::WebBookmarkHeightChanged(_)
        | ViewMessage::WebBookmarkAddSave
        | ViewMessage::WebBookmarkAddCancel
        | ViewMessage::WebBookmarkEditStart(_)
        | ViewMessage::WebBookmarkEditTitleChanged(_)
        | ViewMessage::WebBookmarkEditUrlChanged(_)
        | ViewMessage::WebBookmarkEditWidthChanged(_)
        | ViewMessage::WebBookmarkEditHeightChanged(_)
        | ViewMessage::WebBookmarkEditCookieConfigsChanged(_)
        | ViewMessage::WebBookmarkEditCookieConfigsInsertExample
        | ViewMessage::WebBookmarkEditSave
        | ViewMessage::WebBookmarkEditCancel
        | ViewMessage::WebBookmarkRemove(_) => web::update(app, message),

        ViewMessage::OpenPathInFinder(_)
        | ViewMessage::OpenProjectInExternalPreferred
        | ViewMessage::OpenProjectInExternalWith(_)
        | ViewMessage::CopyProjectPath => external::update(app, message),

        ViewMessage::ReplaySessionFromSnapshot(_) => Task::none(),

        ViewMessage::CloseRequested(window_id) => settings::close_requested(app, window_id),
        ViewMessage::ShutdownFinished => {
            #[cfg(not(target_arch = "wasm32"))]
            std::process::exit(0);

            #[cfg(target_arch = "wasm32")]
            iced::exit()
        }
        _ => Task::none(),
    }
}

#[cfg(test)]
mod tests;

pub(crate) fn sync_work_browser(app: &mut App) -> Task<Message> {
    work_browser::sync_lifecycle(app)
}
