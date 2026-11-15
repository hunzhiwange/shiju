//! 保存回执在所属领域更新反馈状态；错误文本由配置边界规范化。

pub fn channels_config_saved(
    state: &mut crate::state::ChannelsSettingsState,
    error: Option<String>,
) {
    state.save_error = error;
}
