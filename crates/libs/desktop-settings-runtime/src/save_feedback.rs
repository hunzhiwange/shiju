//! 保存回执在所属领域更新反馈状态；错误文本由配置边界规范化。

pub fn reliability_saved(
    state: &mut crate::state::ReliabilitySettingsState,
    error: Option<String>,
) {
    state.save_error = error;
}

pub fn observability_saved(
    state: &mut crate::state::ObservabilitySettingsState,
    error: Option<String>,
) {
    state.save_error = error;
}

pub fn multimodal_saved(state: &mut crate::state::MultimodalSettingsState, error: Option<String>) {
    state.save_error = error;
}

pub fn hooks_saved(state: &mut crate::state::HooksSettingsState, error: Option<String>) {
    state.save_error = error;
}

pub fn runtime_saved(state: &mut crate::state::RuntimeSettingsState, error: Option<String>) {
    state.save_error = error;
}

pub fn query_classification_saved(
    state: &mut crate::state::QueryClassificationSettingsState,
    error: Option<String>,
) {
    state.save_error = error;
}
