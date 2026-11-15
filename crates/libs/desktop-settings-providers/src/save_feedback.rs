//! 保存回执在所属领域更新反馈状态；错误文本由配置边界规范化。

pub fn model_routes_saved(
    error: &mut Option<desktop_i18n::settings_models::messages::Feedback>,
    raw_error: Option<String>,
) {
    *error = raw_error.map(desktop_i18n::settings_models::messages::Feedback::gateway);
}

pub fn embedding_routes_saved(
    state: &mut crate::app_init::state::EmbeddingRoutesSettingsState,
    error: Option<String>,
) {
    state.save_success = error.is_none();
    state.save_error = error.map(desktop_i18n::settings_models::messages::Feedback::gateway);
}
