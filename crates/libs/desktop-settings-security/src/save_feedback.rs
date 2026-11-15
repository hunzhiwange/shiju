//! 保存回执在所属领域更新反馈状态；错误文本由配置边界规范化。

pub fn security_saved(
    state: &mut crate::security_state::SecuritySettingsState,
    error: Option<String>,
) {
    state.save_error = error;
}

pub fn autonomy_saved(
    state: &mut crate::app_init::state::AutonomySettingsState,
    error: Option<String>,
) {
    state.save_error = error;
}
