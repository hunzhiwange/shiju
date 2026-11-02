//! Semantic local feedback. Parameters and remote errors are kept verbatim.
use vw_config_types::ui::DesktopLocale;
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Message {
    GatewayConfigUnavailable,
    DiscoveryTimeout,
    DiscoveryRequest,
    DiscoveryHttpStatus,
    DiscoveryRedirect,
    DiscoveryTooLarge,
    DiscoveryInvalidResponse,
    DiscoveryClient,
    DiscoveryUnknown,

    InvalidContextLimit,
    InvalidInputLimit,
    InvalidOutputLimit,
    InvalidPositiveOutputLimit,

    ConfigureKey,
    PatternValue,
    HeaderCount,
    ConfirmAction,
    DangerService,
    DeleteGateway,
    ProviderMissing,
    ModelMissing,
    ApiKeyRequired,
    ModelIdRequired,
    ModelIdExists,
    InvalidProviderId,
    BaseUrlRequired,
    RoutePatternRequired,
    RouteProviderRequired,
    RouteModelRequired,
    RouteDimensionsInvalid,
    RouteMissingPattern,
    RouteMissingProvider,
    RouteMissingModel,
    RoutePriorityInvalid,
    ClassificationWithoutRoutes,
    TenantInvalid,
    SkeyNameRequired,
    SkeyNameTooLong,
    ExpirationInvalid,
    ServiceComplete,
    ServiceFailed,
    GatewayRecovered,
    GatewayRecoveryFailed,
    GatewayRecoveryUnsupported,
    SignInFirst,
    InvalidAuthorization,
    CredentialSaveFailed,
    CredentialRemoveFailed,
    InvalidLogin,
    ModelsConfigured,
    RealKeyRequired,
    NoDiscoveredModels,
    DiscoveryFailed,
    DiscoveryUnsupported,
    ProviderNotConfigured,
    InvalidProviderConfig,
    DuplicateModelId,
    OutputExceedsContext,
    InvalidModelLimit,
    Supported,
    Unsupported,
    None,
    SyncFetch,
    SyncCache,
    SyncRefresh,
    SourceApi,
    SourceEnv,
    SourceConfig,
    SourceBuiltin,
    DetailName,
    DetailId,
    DetailProvider,
    DetailStatus,
    DetailRelease,
    DetailApiUrl,
    DetailAdapter,
    DetailContext,
    DetailInput,
    DetailOutput,
    DetailTools,
    DetailAttachment,
    DetailReasoning,
    DetailTemperature,
    DetailInputModality,
    DetailOutputModality,
    DetailInterleaved,
    DetailInputCost,
    DetailOutputCost,
    DetailCacheRead,
    DetailCacheWrite,
    DetailHeaders,
    DetailOptions,
    DetailVariants,
    Context,
    Input,
    Output,
    PositiveInteger,
    NonNegativeInteger,
    CurrentModel,
}
pub fn render(locale: DesktopLocale, key: Message, args: &[String]) -> String {
    let texts = match key {
        Message::GatewayConfigUnavailable => (
            "服务端配置不可达，请检查 Gateway 连接状态。{}",
            "服務端設定無法連線，請檢查 Gateway 連線狀態。{}",
            "Server configuration is unavailable. Check the Gateway connection.{}",
        ),
        Message::DiscoveryTimeout => (
            "自动发现模型失败：请求超时",
            "自動探索模型失敗：請求超時",
            "Model discovery failed: Request timed out",
        ),
        Message::DiscoveryRequest => (
            "自动发现模型失败：网络请求失败",
            "自動探索模型失敗：網絡請求失敗",
            "Model discovery failed: Network request failed",
        ),
        Message::DiscoveryHttpStatus => (
            "自动发现模型失败：服务返回错误状态",
            "自動探索模型失敗：服務返回錯誤狀態",
            "Model discovery failed: Service returned an error status",
        ),
        Message::DiscoveryRedirect => (
            "自动发现模型失败：服务返回重定向",
            "自動探索模型失敗：服務返回重定向",
            "Model discovery failed: Service returned a redirect",
        ),
        Message::DiscoveryTooLarge => (
            "自动发现模型失败：响应过大",
            "自動探索模型失敗：響應過大",
            "Model discovery failed: Response too large",
        ),
        Message::DiscoveryInvalidResponse => (
            "自动发现模型失败：返回格式无效",
            "自動探索模型失敗：返回格式無效",
            "Model discovery failed: Invalid response format",
        ),
        Message::DiscoveryClient => (
            "自动发现模型失败：请求初始化失败",
            "自動探索模型失敗：請求初始化失敗",
            "Model discovery failed: Failed to initialize request",
        ),
        Message::DiscoveryUnknown => (
            "自动发现模型失败：未知错误",
            "自動探索模型失敗：未知錯誤",
            "Model discovery failed: Unknown error",
        ),

        Message::InvalidContextLimit => (
            "模型 {} 的上下文限制必须是正整数",
            "模型 {} 的上下文限制必須是正整數",
            "Model {} context limit must be a positive integer",
        ),
        Message::InvalidInputLimit => (
            "模型 {} 的输入限制必须是正整数",
            "模型 {} 的輸入限制必須是正整數",
            "Model {} input limit must be a positive integer",
        ),
        Message::InvalidOutputLimit => (
            "模型 {} 的输出限制必须是非负整数",
            "模型 {} 的輸出限制必須是非負整數",
            "Model {} output limit must be a non-negative integer",
        ),
        Message::InvalidPositiveOutputLimit => (
            "模型 {} 的输出限制必须是正整数",
            "模型 {} 的輸出限制必須是正整數",
            "Model {} output limit must be a positive integer",
        ),

        Message::ConfigureKey => {
            ("配置 {} 的密钥", "配置 {} 的密鑰", "Configure credentials for {}")
        }
        Message::PatternValue => ("匹配模式：{}", "匹配模式：{}", "Pattern: {}"),
        Message::HeaderCount => ("已配置 {} 项", "已配置 {} 項", "{} configured"),
        Message::ConfirmAction => ("确认{}", "確認{}", "Confirm {}"),
        Message::DangerService => (
            "{} 是危险操作，可能会中断当前本机 gateway 连接、停止后台任务，卸载服务还会移除开机自启动配置。\n\n将执行命令：{}\n\n确认继续？",
            "{} 是危險操作，可能會中斷當前本機 gateway 連接、停止後台任務，卸載服務還會移除開機自啓動配置。\n\n將執行命令：{}\n\n確認繼續？",
            "{} may disconnect the local Gateway and stop background tasks. Uninstalling also removes automatic startup.\n\nCommand: {}\n\nContinue?",
        ),
        Message::DeleteGateway => (
            "将删除客户端网关「{}」。此操作不可撤销。\n地址: {}:{}\nID: {}",
            "將刪除客戶端網關「{}」。此操作不可撤銷。\n地址: {}:{}\nID: {}",
            "Delete client Gateway \"{}\"? This cannot be undone.\nAddress: {}:{}\nID: {}",
        ),
        Message::ProviderMissing => ("未找到提供商", "未找到提供商", "Provider not found"),
        Message::ModelMissing => ("未找到模型", "未找到模型", "Model not found"),
        Message::ApiKeyRequired => ("请输入 API 密钥", "請輸入 API 密鑰", "Enter an API key"),
        Message::ModelIdRequired => {
            ("模型 ID 不能为空", "模型 ID 不能為空", "Model ID must not be empty")
        }
        Message::ModelIdExists => {
            ("模型 ID 已存在：{}", "模型 ID 已存在：{}", "Model ID already exists: {}")
        }
        Message::InvalidProviderId => (
            "提供商 ID 仅支持小写字母、数字、连字符或下划线",
            "提供商 ID 僅支持小寫字母、數字、連字符或下划線",
            "Provider ID supports only lowercase letters, digits, hyphens and underscores",
        ),
        Message::BaseUrlRequired => {
            ("基础 URL 不能为空", "基礎 URL 不能為空", "Base URL must not be empty")
        }
        Message::RoutePatternRequired => (
            "第 {} 条路由的 pattern 不能为空",
            "第 {} 條路由的 pattern 不能為空",
            "Route {} pattern must not be empty",
        ),
        Message::RouteProviderRequired => (
            "第 {} 条路由的 provider 不能为空",
            "第 {} 條路由的 provider 不能為空",
            "Route {} provider must not be empty",
        ),
        Message::RouteModelRequired => (
            "第 {} 条路由的 model 不能为空",
            "第 {} 條路由的 model 不能為空",
            "Route {} model must not be empty",
        ),
        Message::RouteDimensionsInvalid => (
            "第 {} 条路由的 dimensions 必须是大于 0 的整数",
            "第 {} 條路由的 dimensions 必須是大於 0 的整數",
            "Route {} dimensions must be a positive integer",
        ),
        Message::RouteMissingPattern => {
            ("第 {} 条路由缺少 pattern", "第 {} 條路由缺少 pattern", "Route {} is missing pattern")
        }
        Message::RouteMissingProvider => (
            "第 {} 条路由缺少 provider",
            "第 {} 條路由缺少 provider",
            "Route {} is missing provider",
        ),
        Message::RouteMissingModel => {
            ("第 {} 条路由缺少 model", "第 {} 條路由缺少 model", "Route {} is missing model")
        }
        Message::RoutePriorityInvalid => (
            "第 {} 条路由的 priority 不是有效数字",
            "第 {} 條路由的 priority 不是有效數字",
            "Route {} priority is not a valid number",
        ),
        Message::ClassificationWithoutRoutes => (
            "已检测到 query_classification 规则，但桌面模型路由列表为空",
            "已檢測到 query_classification 規則，但桌面模型路由列表為空",
            "query_classification rules were found, but the Desktop model route list is empty",
        ),
        Message::TenantInvalid => {
            ("租户配置无效：{}", "租戶配置無效：{}", "Invalid tenant configuration: {}")
        }
        Message::SkeyNameRequired => {
            ("skey 名称不能为空", "skey 名稱不能為空", "skey name must not be empty")
        }
        Message::SkeyNameTooLong => (
            "skey 名称不能超过 80 个字符",
            "skey 名稱不能超過 80 個字符",
            "skey name must be 80 characters or fewer",
        ),
        Message::ExpirationInvalid => (
            "过期日期必须从日历选择，或使用 2026-12-31 格式",
            "過期日期必須從日曆選擇，或使用 2026-12-31 格式",
            "Select an expiration date or use the format 2026-12-31",
        ),
        Message::ServiceComplete => {
            ("service {} 执行完成", "service {} 執行完成", "service {} completed")
        }
        Message::ServiceFailed => {
            ("service {} 失败: {}", "service {} 失敗: {}", "service {} failed: {}")
        }
        Message::GatewayRecovered => (
            "本机 gateway 已恢复并刷新连接状态。",
            "本機 gateway 已恢復並刷新連接狀態。",
            "The local Gateway recovered and its connection status was refreshed.",
        ),
        Message::GatewayRecoveryFailed => (
            "service {} 已执行，但本机 gateway 自动恢复失败: {}",
            "service {} 已執行，但本機 gateway 自動恢復失敗: {}",
            "service {} finished, but local Gateway recovery failed: {}",
        ),
        Message::GatewayRecoveryUnsupported => (
            "wasm 环境不支持本机 gateway 自动恢复",
            "wasm 環境不支持本機 gateway 自動恢復",
            "Local Gateway recovery is unavailable in wasm",
        ),
        Message::SignInFirst => (
            "请先登录识矩账户，模型将自动配置",
            "請先登錄識矩賬戶，模型將自動配置",
            "Sign in to ShiJu to configure models automatically",
        ),
        Message::InvalidAuthorization => (
            "识矩模型授权响应无效",
            "識矩模型授權響應無效",
            "Invalid ShiJu model authorization response",
        ),
        Message::CredentialSaveFailed => (
            "识矩模型凭据保存失败，请检查认证文件权限",
            "識矩模型憑據保存失敗，請檢查認證文件權限",
            "Failed to save ShiJu model credentials. Check authentication file permissions",
        ),
        Message::CredentialRemoveFailed => (
            "无法清理识矩模型凭据，请检查认证文件权限",
            "無法清理識矩模型憑據，請檢查認證文件權限",
            "Failed to remove ShiJu model credentials. Check authentication file permissions",
        ),
        Message::InvalidLogin => (
            "识矩登录凭据无效，请重新登录",
            "識矩登錄憑據無效，請重新登錄",
            "Invalid ShiJu login credentials. Sign in again",
        ),
        Message::ModelsConfigured => {
            ("识矩模型已自动配置", "識矩模型已自動配置", "ShiJu models configured automatically")
        }
        Message::RealKeyRequired => (
            "请先填写或保存真实 API Key",
            "請先填寫或保存真實 API Key",
            "Enter or save an actual API key first",
        ),
        Message::NoDiscoveredModels => {
            ("未发现可用模型", "未發現可用模型", "No available models discovered")
        }
        Message::DiscoveryFailed => {
            ("自动发现模型失败：{}", "自動發現模型失敗：{}", "Model discovery failed: {}")
        }
        Message::DiscoveryUnsupported => (
            "当前平台暂不支持自动发现模型",
            "當前平台暫不支持自動發現模型",
            "Model discovery is unavailable on this platform",
        ),
        Message::ProviderNotConfigured => (
            "提供商 {} 不在系统配置的 providers 中",
            "提供商 {} 不在系統配置的 providers 中",
            "Provider {} is missing from the system providers configuration",
        ),
        Message::InvalidProviderConfig => (
            "提供商 {} 的配置格式无效",
            "提供商 {} 的配置格式無效",
            "Provider {} has an invalid configuration format",
        ),
        Message::DuplicateModelId => {
            ("模型 ID 重复：{}", "模型 ID 重復：{}", "Duplicate model ID: {}")
        }
        Message::OutputExceedsContext => (
            "模型 {} 的输出限制不能超过上下文限制",
            "模型 {} 的輸出限制不能超過上下文限制",
            "Model {} output limit must not exceed its context limit",
        ),
        Message::InvalidModelLimit => {
            ("模型 {} 的{}限制必须是{}", "模型 {} 的{}限制必須是{}", "Model {} {} limit must be {}")
        }
        Message::Supported => ("支持", "支持", "Supported"),
        Message::Unsupported => ("不支持", "不支持", "Unsupported"),
        Message::None => ("无", "無", "None"),
        Message::SyncFetch => (
            "正在从 models.dev 拉取模型目录…",
            "正在從 models.dev 拉取模型目錄…",
            "Fetching the model catalog from models.dev…",
        ),
        Message::SyncCache => {
            ("正在写入本地缓存…", "正在寫入本地緩存…", "Writing the local cache…")
        }
        Message::SyncRefresh => {
            ("正在刷新提供商视图…", "正在刷新提供商視圖…", "Refreshing providers…")
        }
        Message::SourceApi => ("API 密钥", "API 密鑰", "API key"),
        Message::SourceEnv => ("环境变量", "環境變量", "Environment variable"),
        Message::SourceConfig => ("配置", "配置", "Configuration"),
        Message::SourceBuiltin => ("内置", "內置", "Built-in"),
        Message::DetailName => ("模型名称 (name)", "模型名稱 (name)", "Model name (name)"),
        Message::DetailId => ("模型 ID (id)", "模型 ID (id)", "Model ID (id)"),
        Message::DetailProvider => {
            ("提供商 (providerID)", "提供商 (providerID)", "Provider (providerID)")
        }
        Message::DetailStatus => ("状态 (status)", "狀態 (status)", "Status (status)"),
        Message::DetailRelease => {
            ("发布日期 (release_date)", "發佈日期 (release_date)", "Release date (release_date)")
        }
        Message::DetailApiUrl => {
            ("API Base URL (api.url)", "API Base URL (api.url)", "API Base URL (api.url)")
        }
        Message::DetailAdapter => {
            ("适配器 (api.adapter)", "適配器 (api.adapter)", "Adapter (api.adapter)")
        }
        Message::DetailContext => (
            "上下文限制 (limit.context)",
            "上下文限制 (limit.context)",
            "Context limit (limit.context)",
        ),
        Message::DetailInput => {
            ("输入限制 (limit.input)", "輸入限制 (limit.input)", "Input limit (limit.input)")
        }
        Message::DetailOutput => {
            ("输出限制 (limit.output)", "輸出限制 (limit.output)", "Output limit (limit.output)")
        }
        Message::DetailTools => (
            "支持工具调用 (capabilities.toolcall)",
            "支持工具調用 (capabilities.toolcall)",
            "Tool calling (capabilities.toolcall)",
        ),
        Message::DetailAttachment => (
            "支持附件 (capabilities.attachment)",
            "支持附件 (capabilities.attachment)",
            "Attachments (capabilities.attachment)",
        ),
        Message::DetailReasoning => (
            "支持推理 (capabilities.reasoning)",
            "支持推理 (capabilities.reasoning)",
            "Reasoning (capabilities.reasoning)",
        ),
        Message::DetailTemperature => (
            "支持温度 (capabilities.temperature)",
            "支持溫度 (capabilities.temperature)",
            "Temperature (capabilities.temperature)",
        ),
        Message::DetailInputModality => (
            "输入模态 (capabilities.input.*)",
            "輸入模態 (capabilities.input.*)",
            "Input modalities (capabilities.input.*)",
        ),
        Message::DetailOutputModality => (
            "输出模态 (capabilities.output.*)",
            "輸出模態 (capabilities.output.*)",
            "Output modalities (capabilities.output.*)",
        ),
        Message::DetailInterleaved => (
            "混合输入 (capabilities.interleaved)",
            "混合輸入 (capabilities.interleaved)",
            "Interleaved input (capabilities.interleaved)",
        ),
        Message::DetailInputCost => {
            ("输入价格 (cost.input)", "輸入價格 (cost.input)", "Input price (cost.input)")
        }
        Message::DetailOutputCost => {
            ("输出价格 (cost.output)", "輸出價格 (cost.output)", "Output price (cost.output)")
        }
        Message::DetailCacheRead => {
            ("缓存读 (cost.cache.read)", "緩存讀 (cost.cache.read)", "Cache read (cost.cache.read)")
        }
        Message::DetailCacheWrite => (
            "缓存写 (cost.cache.write)",
            "緩存寫 (cost.cache.write)",
            "Cache write (cost.cache.write)",
        ),
        Message::DetailHeaders => {
            ("请求头数量 (headers)", "請求頭數量 (headers)", "Header count (headers)")
        }
        Message::DetailOptions => (
            "自定义选项数量 (options)",
            "自定義選項數量 (options)",
            "Custom option count (options)",
        ),
        Message::DetailVariants => {
            ("变体数量 (variants)", "變體數量 (variants)", "Variant count (variants)")
        }
        Message::Context => ("上下文", "上下文", "context"),
        Message::Input => ("输入", "輸入", "input"),
        Message::Output => ("输出", "輸出", "output"),
        Message::PositiveInteger => ("正整数", "正整數", "a positive integer"),
        Message::NonNegativeInteger => ("非负整数", "非負整數", "a non-negative integer"),
        Message::CurrentModel => ("当前模型", "當前模型", "current model"),
    };
    let template = match locale {
        DesktopLocale::SimplifiedChinese => texts.0,
        DesktopLocale::TraditionalChinese => texts.1,
        DesktopLocale::English => texts.2,
    };
    // Substitute only template slots. Never process inserted user/service content.
    let mut parts = template.split("{}");
    let mut output = parts.next().unwrap_or_default().to_string();
    for (index, part) in parts.enumerate() {
        output.push_str(args.get(index).map(String::as_str).unwrap_or_default());
        output.push_str(part);
    }
    output
}
