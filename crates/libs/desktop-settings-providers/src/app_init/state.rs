#[derive(Debug, Clone, Default)]
pub struct EmbeddingRouteDraft {
    /// 路由模式（映射到配置中的 hint）
    pub pattern: String,
    /// 提供商名称
    pub provider: String,
    /// 模型名称
    pub model: String,
    /// 维度输入文本
    pub dimensions: String,
    /// 可选 API Key 覆盖，优先于全局 api_key。
    pub api_key_input: String,
}

/// 嵌入路由设置面板状态
///
/// 管理嵌入模型路由列表的新增、编辑和删除草稿状态。
#[derive(Debug, Clone, Default)]
pub struct EmbeddingRoutesSettingsState {
    pub routes: Vec<EmbeddingRouteDraft>,
    pub save_error: Option<desktop_i18n::settings_models::messages::Feedback>,
    pub save_success: bool,
}
