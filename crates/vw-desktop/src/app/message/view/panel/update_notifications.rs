//! 静默检测桌面更新，以及持久化用户跳过的发布版本。
use super::ViewMessage;
use crate::app::desktop_update::SkippedRelease;
use crate::app::{App, Message};
use iced::Task;

pub(super) fn update(app: &mut App, message: ViewMessage) -> Task<Message> {
    match message {
        ViewMessage::AutoCheckAppUpdate => {
            let state = &mut app.desktop_update;
            if state.busy
                || state.checking
                || state.saving
                || state.background_request.is_some()
                || app.show_cli_install_modal
            {
                return Task::none();
            }
            #[cfg(not(target_arch = "wasm32"))]
            if state.ready.is_some() {
                return Task::none();
            }
            let Some(source) = state.source else { return Task::none() };
            state.request += 1;
            let request = state.request;
            state.background_request = Some(request);
            Task::perform(super::settings::fetch_latest_app_release(source), move |result| {
                Message::View(ViewMessage::AutoCheckAppUpdateFinished(request, source, result))
            })
        }
        ViewMessage::AutoCheckAppUpdateFinished(request, source, result) => {
            let state = &mut app.desktop_update;
            if state.background_request != Some(request) {
                return Task::none();
            }
            state.background_request = None;
            if state.request != request || state.source != Some(source) || state.busy {
                return Task::none();
            }
            // 网络失败保留上次成功检测结果，不弹窗、不打断用户。
            if let Ok(release) = result
                && release.source == source
                && release.is_newer_than(env!("CARGO_PKG_VERSION")).is_ok()
            {
                state.release = Some(release);
            }
            Task::none()
        }
        ViewMessage::SkipAppUpdate => {
            let state = &mut app.desktop_update;
            if state.busy || state.checking || state.saving_skip {
                return Task::none();
            }
            let Some(release) = state.notification_release() else { return Task::none() };
            let skipped =
                SkippedRelease { source: release.source, version: release.version.clone() };
            let patch = serde_json::json!({"app_update_skipped_release": skipped});
            state.saving_skip = true;
            Task::perform(crate::app::config::save_app_config_async(patch), move |result| {
                Message::View(ViewMessage::SkipAppUpdateSaved(skipped.clone(), result))
            })
        }
        ViewMessage::SkipAppUpdateSaved(skipped, result) => {
            app.desktop_update.saving_skip = false;
            match result {
                Ok(()) => {
                    app.desktop_update.skipped = Some(skipped);
                    Task::none()
                }
                Err(error) => app.show_error_toast(format!("跳过版本未保存：{error}")),
            }
        }
        _ => Task::none(),
    }
}

#[cfg(test)]
#[path = "update_notifications_tests.rs"]
mod tests;
