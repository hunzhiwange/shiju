//! 保存回执在所属领域更新反馈状态；错误文本由配置边界规范化。

pub fn skills_saved(state: &mut crate::update::state::State, error: Option<String>) {
    state.save_error = error;
}
