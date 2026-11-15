//! 桌面设置记录和保存结果提示适配。
use crate::app::{App, Message};
use iced::Task;
pub(crate) use desktop_editor::persistence::{ConfigPersistence, EditorSettingsPatch};

pub(super) fn enqueue(app: &mut App, patch: EditorSettingsPatch) -> Task<Message> {
    #[cfg(not(target_arch = "wasm32"))]
    for field in patch.edited_fields() {
        app.note_startup_setting_edit(field);
    }
    app.editor_config_persistence
        .enqueue(crate::app::config::gateway_client_endpoint(), patch)
        .map(Message::Editor)
}

pub(super) fn save_complete(app: &mut App, result: Result<(), String>) -> Task<Message> {
    let next = app.editor_config_persistence.save_complete().map(Message::Editor);
    if result.is_err() {
        tracing::warn!(target: "vw_desktop", "failed to save editor settings");
        return Task::batch([next, app.show_error_toast("编辑器配置保存失败，请重试")]);
    }
    next
}
