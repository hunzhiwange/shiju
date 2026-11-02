//! Display-only model settings catalog. Provider IDs and model content stay verbatim.
use vw_config_types::ui::DesktopLocale;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Text {
    ModelClassification,
    Multimodal,
    Models,
    SearchModels,
    ModelDetails,
    Provider,
    ModelRoutes,
    Pattern,
    Priority,
    EnableAll,
    DisableAll,
    ResetDefaults,
    Collapse,
    Expand,
    More,
    RefreshingCatalog,
    NoModels,
    DiscoveryPending,
    ConnectFirst,
    Refreshing,
    Saving,
    Refresh,
    ModelSettings,
    ModelIntro,
    SearchFilter,
    SearchHelp,
    ConnectedModels,
    SavingModels,
    GroupedModels,
    FieldList,
    RawJson,
    RouteIntro,
    AddRoute,
    RouteRules,
    RoutePriorityHelp,
    NoRoutes,
    DeleteRoute,
    PatternHelp,
    PatternExample,
    ProviderHelp,
    ProviderExample,
    ModelHelp,
    ModelExample,
    PriorityHelp,
}

pub fn text(locale: DesktopLocale, key: Text) -> &'static str {
    let translations = match key {
        Text::ModelClassification => ("模型分类", "模型分類", "Model classification"),
        Text::Multimodal => ("多模态", "多模態", "Multimodal"),
        Text::Models => ("模型", "模型", "Models"),
        Text::SearchModels => ("搜索模型", "搜尋模型", "Search models"),
        Text::ModelDetails => ("模型详情", "模型詳情", "Model details"),
        Text::Provider => ("提供商", "提供商", "Provider"),
        Text::ModelRoutes => ("模型路由", "模型路由", "Model routes"),
        Text::Pattern => ("匹配模式", "比對模式", "Pattern"),
        Text::Priority => ("优先级", "優先順序", "Priority"),
        Text::EnableAll => ("全部启用", "全部啟用", "Enable all"),
        Text::DisableAll => ("全部禁用", "全部停用", "Disable all"),
        Text::ResetDefaults => ("恢复默认", "恢復預設", "Restore defaults"),
        Text::Collapse => ("收起", "收合", "Collapse"),
        Text::Expand => ("展开", "展開", "Expand"),
        Text::More => ("更多", "更多", "More"),
        Text::RefreshingCatalog => {
            ("正在刷新模型目录…", "正在重新整理模型目錄…", "Refreshing model catalog…")
        }
        Text::NoModels => (
            "暂无已连接的提供商或模型",
            "暫無已連接的提供商或模型",
            "No connected providers or models",
        ),
        Text::DiscoveryPending => (
            "稍后会展示当前已发现的 provider 与模型。",
            "稍後會顯示目前已探索到的 provider 與模型。",
            "Discovered providers and models will appear shortly.",
        ),
        Text::ConnectFirst => (
            "请先完成 provider 连接，或调整搜索关键字后重试。",
            "請先連接 provider，或調整搜尋關鍵字後重試。",
            "Connect a provider or adjust your search and try again.",
        ),
        Text::Refreshing => ("刷新中…", "重新整理中…", "Refreshing…"),
        Text::Saving => ("保存中…", "儲存中…", "Saving…"),
        Text::Refresh => ("刷新", "重新整理", "Refresh"),
        Text::ModelSettings => ("模型配置", "模型設定", "Model settings"),
        Text::ModelIntro => (
            "浏览已连接 provider 的模型目录，搜索、筛选并切换模型可用状态。",
            "瀏覽已連接 provider 的模型目錄，搜尋、篩選並切換模型可用狀態。",
            "Browse connected providers, search models and change availability.",
        ),
        Text::SearchFilter => ("搜索与过滤", "搜尋與篩選", "Search and filter"),
        Text::SearchHelp => (
            "按 provider 名称、provider ID、模型名称或模型 ID 过滤结果。",
            "依 provider 名稱、provider ID、模型名稱或模型 ID 篩選結果。",
            "Filter by provider name, provider ID, model name or model ID.",
        ),
        Text::ConnectedModels => ("已连接模型", "已連接模型", "Connected models"),
        Text::SavingModels => (
            "正在保存模型状态，完成后即可继续操作。",
            "正在儲存模型狀態，完成後即可繼續操作。",
            "Saving model availability. You can continue when it finishes.",
        ),
        Text::GroupedModels => (
            "按 provider 分组展示可用模型，可单独或一次性切换启用状态。",
            "依 provider 分組顯示可用模型，可個別或一次切換啟用狀態。",
            "Models are grouped by provider. Enable them individually or together.",
        ),
        Text::FieldList => ("字段列表", "欄位清單", "Fields"),
        Text::RawJson => ("原始 JSON", "原始 JSON", "Raw JSON"),
        Text::RouteIntro => (
            "将匹配模式映射到指定 provider/model，并与查询分类优先级保持一致。",
            "將比對模式對應至指定 provider/model，並與查詢分類優先順序保持一致。",
            "Map patterns to a provider/model with the same priority as query classification.",
        ),
        Text::AddRoute => ("新增路由", "新增路由", "Add route"),
        Text::RouteRules => ("路由规则", "路由規則", "Route rules"),
        Text::RoutePriorityHelp => (
            "优先级越高的规则越先命中，并会同步到 query classification。",
            "優先順序越高的規則越先命中，並會同步至 query classification。",
            "Higher priority rules match first and sync to query classification.",
        ),
        Text::NoRoutes => (
            "暂无模型路由，点击右上角“新增路由”开始配置",
            "暫無模型路由，點選右上角「新增路由」開始設定",
            "No model routes. Select Add route to get started.",
        ),
        Text::DeleteRoute => ("删除路由", "刪除路由", "Delete route"),
        Text::PatternHelp => (
            "定义命中的任务场景，例如 reasoning / fast / code。",
            "定義比對的任務情境，例如 reasoning / fast / code。",
            "Define a task pattern, such as reasoning / fast / code.",
        ),
        Text::PatternExample => (
            "如：reasoning / fast / code",
            "例如：reasoning / fast / code",
            "Example: reasoning / fast / code",
        ),
        Text::ProviderHelp => (
            "命中后要切换到的 provider。",
            "命中後要切換至的 provider。",
            "Provider to use when the pattern matches.",
        ),
        Text::ProviderExample => ("如：openai", "例如：openai", "Example: openai"),
        Text::ModelHelp => (
            "命中后优先使用的模型。",
            "命中後優先使用的模型。",
            "Preferred model when the pattern matches.",
        ),
        Text::ModelExample => ("如：gpt-5", "例如：gpt-5", "Example: gpt-5"),
        Text::PriorityHelp => (
            "越大优先级越高；保存时同步生成 query_classification 规则。",
            "數值越大優先順序越高；儲存時同步產生 query_classification 規則。",
            "Larger values take priority. Saving also generates query_classification rules.",
        ),
    };
    match locale {
        DesktopLocale::SimplifiedChinese => translations.0,
        DesktopLocale::TraditionalChinese => translations.1,
        DesktopLocale::English => translations.2,
    }
}

pub fn enabled_count(locale: DesktopLocale, enabled: usize, total: usize) -> String {
    match locale {
        DesktopLocale::SimplifiedChinese => format!("{enabled} / {total} 已启用"),
        DesktopLocale::TraditionalChinese => format!("{enabled} / {total} 已啟用"),
        DesktopLocale::English => format!("{enabled} / {total} enabled"),
    }
}
pub fn model_count(locale: DesktopLocale, count: usize) -> String {
    match locale {
        DesktopLocale::SimplifiedChinese => format!("{count} 个模型"),
        DesktopLocale::TraditionalChinese => format!("{count} 個模型"),
        DesktopLocale::English => {
            format!("{count} {}", if count == 1 { "model" } else { "models" })
        }
    }
}
pub fn provider_count(locale: DesktopLocale, count: usize) -> String {
    match locale {
        DesktopLocale::SimplifiedChinese => format!("{count} 个提供商"),
        DesktopLocale::TraditionalChinese => format!("{count} 個提供商"),
        DesktopLocale::English => {
            format!("{count} {}", if count == 1 { "provider" } else { "providers" })
        }
    }
}
pub fn route_title(locale: DesktopLocale, index: usize) -> String {
    match locale {
        DesktopLocale::SimplifiedChinese | DesktopLocale::TraditionalChinese => {
            format!("路由 {index}")
        }
        DesktopLocale::English => format!("Route {index}"),
    }
}
pub fn model_metadata(locale: DesktopLocale, tools: bool, images: bool, context: &str) -> String {
    let supported = |value| match (locale, value) {
        (DesktopLocale::SimplifiedChinese, true) => "支持",
        (DesktopLocale::SimplifiedChinese, false) => "不支持",
        (DesktopLocale::TraditionalChinese, true) => "支援",
        (DesktopLocale::TraditionalChinese, false) => "不支援",
        (DesktopLocale::English, true) => "Supported",
        (DesktopLocale::English, false) => "Unsupported",
    };
    let tools = supported(tools);
    let images = supported(images);
    match locale {
        DesktopLocale::SimplifiedChinese => {
            format!("工具: {tools} · 图片输入: {images} · 上下文限制: {context}")
        }
        DesktopLocale::TraditionalChinese => {
            format!("工具: {tools} · 圖片輸入: {images} · 上下文限制: {context}")
        }
        DesktopLocale::English => {
            format!("Tools: {tools} · Image input: {images} · Context limit: {context}")
        }
    }
}

pub mod connection_text;

pub mod messages;

pub mod help;

/// Map known modality identifiers in the display layer; protocol values are untouched.
pub fn modality_label(locale: DesktopLocale, modality: &str) -> &str {
    match (locale, modality) {
        (DesktopLocale::SimplifiedChinese, "text") => "文本",
        (DesktopLocale::TraditionalChinese, "text") => "文字",
        (DesktopLocale::SimplifiedChinese, "image") => "图片",
        (DesktopLocale::TraditionalChinese, "image") => "圖片",
        (DesktopLocale::SimplifiedChinese, "audio") => "音频",
        (DesktopLocale::TraditionalChinese, "audio") => "音訊",
        (DesktopLocale::SimplifiedChinese, "video") => "视频",
        (DesktopLocale::TraditionalChinese, "video") => "影片",
        _ => modality,
    }
}
