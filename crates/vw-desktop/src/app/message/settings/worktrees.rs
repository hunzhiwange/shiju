//! Worktree policy editor events and gateway effects.
use crate::app::state::{WorktreeSettingsState, WorktreesTab};

pub(crate) mod environment;
mod managed;
pub(crate) mod recovery;
#[cfg(test)]
pub(crate) use managed::load_groups;

pub use desktop_settings_workspace::messages::{WorktreeSettingsMessage, ManagedMessage};
#[cfg(test)]
pub(crate) use desktop_settings_workspace::worktrees::apply;

use super::SettingsMessage;
use crate::app::{App, Message};
use iced::Task;

pub(crate) fn update(app: &mut App, event: WorktreeSettingsMessage) -> Task<Message> {
    if let WorktreeSettingsMessage::Recovery(event) = event {
        return recovery::update(app, event);
    }
    if let WorktreeSettingsMessage::Environment(event) = event {
        return environment::update(app, event);
    }
    if let WorktreeSettingsMessage::Managed(event) = event {
        return managed::update(app, event);
    }
    if matches!(
        event,
        WorktreeSettingsMessage::ContextChanged | WorktreeSettingsMessage::TabSelected(_)
    ) {
        recovery::invalidate(app);
    }
    let current = crate::app::state::worktree_identity(app);
    use desktop_settings_workspace::worktrees::{prepare, UpdatePlan};
    let visible = app.navigation_panels.system_settings_tab == crate::app::components::system_settings::SystemTab::Worktrees;
    let (policy, refresh, failed, saved) = match prepare(&mut app.worktree_settings, event, current.clone(), visible) {
        UpdatePlan::Ignore => return Task::none(),
        UpdatePlan::Refresh => return update(app, WorktreeSettingsMessage::Refresh),
        UpdatePlan::Environment => return environment::update(app, environment::EnvironmentMessage::Refresh),
        UpdatePlan::Apply { policy, refresh, failed, saved } => (policy, refresh, failed, saved),
    };
    let generation = app.worktree_settings.generation;
    let client = crate::app::config::gateway_client();
    if let Some(policy) = policy {
        return Task::perform(
            async move {
                desktop_settings_workspace::worktrees::save_policy(&client?, policy).await
            },
            move |result| {
                Message::Settings(SettingsMessage::Worktrees(WorktreeSettingsMessage::Saved {
                    identity: current.clone(),
                    generation,
                    result,
                }))
            },
        );
    }
    if refresh {
        let managed_refresh = managed::update(app, ManagedMessage::Refresh);
        let recovery_refresh = recovery::update(app, recovery::RecoveryMessage::Refresh);
        let environment_refresh = if app.worktree_settings.active_tab == WorktreesTab::Environment {
            environment::update(app, environment::EnvironmentMessage::Refresh)
        } else {
            Task::none()
        };
        return Task::batch([
            managed_refresh,
            recovery_refresh,
            environment_refresh,
            Task::perform(
                async move {
                    desktop_settings_workspace::worktrees::load_policy(&client?).await
                },
                move |result| {
                    Message::Settings(SettingsMessage::Worktrees(WorktreeSettingsMessage::Loaded {
                        identity: current.clone(),
                        generation,
                        result,
                    }))
                },
            ),
        ]);
    }
    if failed {
        return app.show_localized_toast(
            crate::app::state::ToastKind::Error,
            "工作树设置请求失败，未提交的编辑已保留。",
            vec![],
        );
    }
    if saved {
        return app.show_localized_toast(
            crate::app::state::ToastKind::Success,
            "工作树设置已保存。",
            vec![],
        );
    }
    Task::none()
}

pub(crate) fn gateway_changed(app: &mut App) -> Task<Message> {
    recovery::invalidate(app);
    let generation = app.worktree_settings.generation + 1;
    app.worktree_settings = WorktreeSettingsState {
        generation,
        recovery: crate::app::state::WorktreeRecoveryState {
            generation: app.worktree_settings.recovery.generation + 1,
            ..Default::default()
        },
        environment: crate::app::state::WorktreeEnvironmentState {
            generation: app.worktree_settings.environment.generation + 1,
            list_generation: app.worktree_settings.environment.list_generation + 1,
            ..Default::default()
        },
        managed: crate::app::state::ManagedWorktreesState {
            generation: app.worktree_settings.managed.generation + 1,
            ..Default::default()
        },
        ..Default::default()
    };
    if app.navigation_panels.system_settings_tab == crate::app::components::system_settings::SystemTab::Worktrees {
        update(app, WorktreeSettingsMessage::Refresh)
    } else {
        Task::none()
    }
}
