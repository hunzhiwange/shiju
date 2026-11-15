//! 保存回执在所属领域更新反馈状态；错误文本由配置边界规范化。

pub fn memory_saved(state: &mut crate::state::MemorySettingsState, error: Option<String>) {
    state.save_error = error;
}

pub fn storage_saved(state: &mut crate::state::StorageSettingsState, error: Option<String>) {
    state.save_error = error;
}
