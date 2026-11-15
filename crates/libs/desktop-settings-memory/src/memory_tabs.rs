use crate::state::MemorySettingsTab;
use desktop_i18n::settings_runtime::text as runtime_text;
#[derive(Debug, Clone, Copy)]
pub struct SettingsTabDescriptor<T> {
    pub tab: T,
    pub label: &'static str,
    pub keywords: &'static [&'static str],
}
impl<T> SettingsTabDescriptor<T> {
    const fn new(tab: T, label: &'static str, keywords: &'static [&'static str]) -> Self { Self { tab, label, keywords } }
}
pub const MEMORY_TABS: &[SettingsTabDescriptor<MemorySettingsTab>] = &[
    SettingsTabDescriptor::new(
        MemorySettingsTab::General,
        "基础行为",
        &["backend", "auto save", "自动保存", "清理"],
    ),
    SettingsTabDescriptor::new(
        MemorySettingsTab::RetentionAndCache,
        "保留与缓存",
        &["保留", "缓存", "归档", "快照", "自动恢复", "SQLite 打开超时"],
    ),
    SettingsTabDescriptor::new(
        MemorySettingsTab::Embedding,
        "嵌入与检索",
        &["embedding", "嵌入模型", "维度", "混合检索", "权重"],
    ),
    SettingsTabDescriptor::new(
        MemorySettingsTab::EmbeddingRoutes,
        "嵌入路由",
        &["embedding routes", "路由规则", "匹配模式", "提供商", "API Key"],
    ),
    SettingsTabDescriptor::new(
        MemorySettingsTab::Storage,
        "记忆存储",
        &["storage", "database", "存储后端", "连接串", "Schema", "表名", "TLS", "持久化"],
    ),
];
pub const QDRANT_TAB: &[SettingsTabDescriptor<MemorySettingsTab>] =
    &[SettingsTabDescriptor::new(
        MemorySettingsTab::Qdrant,
        "Qdrant 连接",
        &["qdrant", "向量数据库", "collection", "API Key"],
    )];

pub fn memory_tab_label(
    locale: config_ui::DesktopLocale,
    tab: MemorySettingsTab,
) -> &'static str {
    runtime_text(
        locale,
        match tab {
            MemorySettingsTab::General => "基础行为",
            MemorySettingsTab::RetentionAndCache => "保留与缓存",
            MemorySettingsTab::Embedding => "嵌入与检索",
            MemorySettingsTab::EmbeddingRoutes => "嵌入路由",
            MemorySettingsTab::Storage => "记忆存储",
            MemorySettingsTab::Qdrant => "Qdrant 连接",
        },
    )
}
