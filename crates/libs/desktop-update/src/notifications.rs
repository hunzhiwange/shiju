//! Silent background update checks and persisted release dismissal.
use iced::Task;
use crate::app::desktop_update::{DesktopUpdate, SkippedRelease};
use desktop_release::{DesktopRelease, UpdateSource};
#[derive(Debug, Clone)]
pub enum Message {
    AutoCheckAppUpdate,
    AutoCheckAppUpdateFinished(u64, UpdateSource, Result<DesktopRelease, String>),
    SkipAppUpdate,
    SkipAppUpdateSaved(SkippedRelease, Result<(), String>),
}
pub enum Effect {
    SaveSkipped(SkippedRelease, serde_json::Value),
    SaveFailed(String),
}
pub fn update(state: &mut DesktopUpdate, show_cli_install_modal: bool, version: &'static str, message: Message) -> (Task<Message>, Option<Effect>) {
    match message {
        Message::AutoCheckAppUpdate => {
            if state.busy
                || state.checking
                || state.saving
                || state.background_request.is_some()
                || show_cli_install_modal
            {
                return (Task::none(), None);
            }
            #[cfg(not(target_arch = "wasm32"))]
            if state.ready.is_some() {
                return (Task::none(), None);
            }
            let Some(source) = state.source else { return (Task::none(), None) };
            state.request += 1;
            let request = state.request;
            state.background_request = Some(request);
            (Task::perform(crate::panel::fetch_latest_app_release(source), move |result| {
                Message::AutoCheckAppUpdateFinished(request, source, result)
            }), None)
        }
        Message::AutoCheckAppUpdateFinished(request, source, result) => {
            if state.background_request != Some(request) {
                return (Task::none(), None);
            }
            state.background_request = None;
            if state.request != request || state.source != Some(source) || state.busy {
                return (Task::none(), None);
            }
            // 网络失败保留上次成功检测结果，不弹窗、不打断用户。
            if let Ok(release) = result
                && release.source == source
                && release.is_newer_than(version).is_ok()
            {
                state.release = Some(release);
            }
            (Task::none(), None)
        }
        Message::SkipAppUpdate => {
            if state.busy || state.checking || state.saving_skip {
                return (Task::none(), None);
            }
            let Some(release) = state.notification_release() else { return (Task::none(), None) };
            let skipped =
                SkippedRelease { source: release.source, version: release.version.clone() };
            let patch = serde_json::json!({"app_update_skipped_release": skipped});
            state.saving_skip = true;
            (Task::none(), Some(Effect::SaveSkipped(skipped, patch)))
        }
        Message::SkipAppUpdateSaved(skipped, result) => {
            state.saving_skip = false;
            match result {
                Ok(()) => {
                    state.skipped = Some(skipped);
                    (Task::none(), None)
                }
                Err(error) => (Task::none(), Some(Effect::SaveFailed(error))),
            }
        }

    }
}


#[cfg(test)]
#[path = "notifications_tests.rs"]
mod tests;
