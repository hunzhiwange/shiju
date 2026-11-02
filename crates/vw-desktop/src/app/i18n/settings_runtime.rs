use vw_config_types::ui::DesktopLocale;
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Text {
    Retention,
    Embedding,
    EmbeddingRoutes,
    MemoryStorage,
    QdrantConnection,
    StorageBackend,
    MemoryBackendHelp,
    MemoryBasicHelp,
    AutoSave,
    AutoSaveHelp,
    AutoSaveLabel,
    Hygiene,
    HygieneHelp,
    EnableHygiene,
    ResponseCache,
    ResponseCacheHelp,
    EnableResponseCache,
    Snapshot,
    SnapshotHelp,
    EnableSnapshot,
    HygieneSnapshot,
    HygieneSnapshotHelp,
    HygieneSnapshotLabel,
    AutoRecover,
    AutoRecoverHelp,
    AutoRecoverLabel,
    RetentionHelp,
    ArchiveDays,
    ArchiveSuffix,
    PurgeDays,
    PurgeSuffix,
    ConversationDays,
    Days,
    EmbeddingCache,
    Entries,
    ChunkTokens,
    CacheTtl,
    Minutes,
    CacheEntries,
    SqliteTimeout,
    SecondsUnlimited,
    EmbeddingHelp,
    EmbeddingProvider,
    AlibabaEmbeddingHelp,
    EmbeddingModel,
    ModelNamePrefix,
    RouteHint,
    RouteReference,
    Dimensions,
    DimensionsSuffix,
    VectorWeight,
    KeywordWeight,
    MinRelevance,
    QdrantHelp,
    QdrantUrl,
    QdrantUrlHelp,
    QdrantCollection,
    QdrantCollectionHelp,
    QdrantKey,
    QdrantKeyHelp,
    Optional,
    MemoryConfig,
    MemoryIntro,

    StorageKind,
    StorageKindHelp,
    DatabaseUrl,
    DatabaseHint,
    Schema,
    SchemaHelp,
    Table,
    TableHelp,
    Timeout,
    TimeoutPlaceholder,
    TimeoutHint,
    Tls,
    TlsHelp,
    EnableTls,
    ConnectionParams,
    ConnectionIntro,
    NamingTimeout,
    NamingHelp,
    Security,
    StorageSecurityHelp,
    Backend,
    BackendHelp,
    None,
    Log,
    OtelEndpoint,
    OtelEndpointHelp,
    OtelService,
    OtelServiceHelp,
    Trace,
    TraceHelp,
    Rolling,
    Full,
    TraceFile,
    TraceFileHelp,
    RollingEntries,
    RollingHelp,
    ObservabilityConfig,
    ObservabilityIntro,
    ObservabilityBackend,
    ObservabilityBackendHelp,
    TraceStorageHelp,
    ObservabilityHelpTitle,

    RuntimeKind,
    EnvironmentHelp,
    Auto,
    Enabled,
    Disabled,
    EnableReasoning,
    ReasoningHelp,
    Default,
    Minimal,
    Low,
    Medium,
    High,
    ExtraHigh,
    ReasoningLevel,
    LevelHelp,
    DockerConfig,
    DockerHelp,
    Image,
    ImageHelp,
    Network,
    NetworkHelp,
    MemoryLimit,
    UnlimitedHelp,
    Unlimited,
    CpuLimit,
    CpuExampleHelp,
    CpuExample,
    ReadOnly,
    ReadOnlyHelp,
    Mount,
    MountHelp,
    AllowedRoots,
    RootsHelp,
    PathsHint,
    RootsHint,
    RuntimeConfig,
    RuntimeIntro,
    Basic,
    KindHelp,
    Overrides,
    OverridesHelp,
}
pub fn text(locale: DesktopLocale, key: Text) -> &'static str {
    let values = match key {
        Text::Retention => ("保留与缓存", "保留與快取", "Retention and cache"),
        Text::Embedding => ("嵌入与检索", "嵌入與檢索", "Embeddings and retrieval"),
        Text::EmbeddingRoutes => ("嵌入路由", "嵌入路由", "Embedding routes"),
        Text::MemoryStorage => ("记忆存储", "記憶儲存", "Memory storage"),
        Text::QdrantConnection => ("Qdrant 连接", "Qdrant 連線", "Qdrant connection"),
        Text::StorageBackend => ("存储后端", "儲存後端", "Storage backend"),
        Text::MemoryBackendHelp => (
            "选择记忆系统使用的持久化后端。",
            "選擇記憶系統使用的持久化後端。",
            "Choose the persistence backend for memory.",
        ),
        Text::MemoryBasicHelp => (
            "控制持久化、清理、快照和自动恢复等核心记忆行为。",
            "控制持久化、清理、快照與自動還原等核心記憶行為。",
            "Control persistence, cleanup, snapshots and automatic recovery.",
        ),
        Text::AutoSave => ("自动保存", "自動儲存", "Auto-save"),
        Text::AutoSaveHelp => (
            "自动保存用户输入到记忆。",
            "自動將使用者輸入儲存至記憶。",
            "Automatically save user input to memory.",
        ),
        Text::AutoSaveLabel => (
            "自动保存用户输入到记忆",
            "自動將使用者輸入儲存至記憶",
            "Automatically save user input to memory",
        ),
        Text::Hygiene => ("记忆卫生清理", "記憶清理", "Memory hygiene"),
        Text::HygieneHelp => (
            "定期清理低价值或陈旧记忆。",
            "定期清理低價值或過時的記憶。",
            "Periodically clean up low-value or stale memories.",
        ),
        Text::EnableHygiene => ("启用记忆卫生清理", "啟用記憶清理", "Enable memory hygiene"),
        Text::ResponseCache => ("响应缓存", "回應快取", "Response cache"),
        Text::ResponseCacheHelp => (
            "缓存部分响应结果以减少重复开销。",
            "快取部分回應以減少重複開銷。",
            "Cache some responses to reduce repeated work.",
        ),
        Text::EnableResponseCache => ("启用响应缓存", "啟用回應快取", "Enable response cache"),
        Text::Snapshot => ("记忆快照", "記憶快照", "Memory snapshots"),
        Text::SnapshotHelp => {
            ("启用记忆快照导出。", "啟用記憶快照匯出。", "Enable memory snapshot export.")
        }
        Text::EnableSnapshot => {
            ("启用记忆快照导出", "啟用記憶快照匯出", "Enable memory snapshot export")
        }
        Text::HygieneSnapshot => ("清理时快照", "清理時建立快照", "Snapshot during cleanup"),
        Text::HygieneSnapshotHelp => (
            "执行卫生清理时同步生成快照。",
            "執行清理時同步產生快照。",
            "Create a snapshot during hygiene cleanup.",
        ),
        Text::HygieneSnapshotLabel => {
            ("清理时同步生成快照", "清理時同步產生快照", "Create a snapshot during cleanup")
        }
        Text::AutoRecover => ("自动恢复", "自動還原", "Automatic recovery"),
        Text::AutoRecoverHelp => (
            "数据库缺失时自动从快照恢复。",
            "資料庫遺失時自動從快照還原。",
            "Automatically recover from a snapshot if the database is missing.",
        ),
        Text::AutoRecoverLabel => (
            "数据库缺失时自动从快照恢复",
            "資料庫遺失時自動從快照還原",
            "Recover from a snapshot if the database is missing",
        ),
        Text::RetentionHelp => (
            "配置归档、清理、SQLite 打开超时与缓存容量。",
            "設定封存、清理、SQLite 開啟逾時與快取容量。",
            "Configure archival, cleanup, SQLite open timeout and cache capacity.",
        ),
        Text::ArchiveDays => ("归档天数", "封存天數", "Archive after days"),
        Text::ArchiveSuffix => ("天后归档", "天後封存", "days before archival"),
        Text::PurgeDays => ("清除天数", "清除天數", "Purge after days"),
        Text::PurgeSuffix => ("天后清除", "天後清除", "days before purge"),
        Text::ConversationDays => ("对话保留天数", "對話保留天數", "Conversation retention days"),
        Text::Days => ("天", "天", "days"),
        Text::EmbeddingCache => ("嵌入缓存大小", "嵌入快取大小", "Embedding cache size"),
        Text::Entries => ("条", "筆", "entries"),
        Text::ChunkTokens => ("分块最大 Token", "分塊最大 Token", "Maximum tokens per chunk"),
        Text::CacheTtl => ("缓存有效期", "快取有效期", "Cache lifetime"),
        Text::Minutes => ("分钟", "分鐘", "minutes"),
        Text::CacheEntries => ("缓存最大条目", "快取最大項目數", "Maximum cache entries"),
        Text::SqliteTimeout => ("SQLite 超时", "SQLite 逾時", "SQLite timeout"),
        Text::SecondsUnlimited => {
            ("秒（0 表示无限等待）", "秒（0 表示無限等待）", "seconds (0 waits indefinitely)")
        }
        Text::EmbeddingHelp => (
            "配置 embedding provider/model、维度和混合检索权重。",
            "設定 embedding provider/model、維度與混合檢索權重。",
            "Configure the embedding provider/model, dimensions and hybrid retrieval weights.",
        ),
        Text::EmbeddingProvider => ("嵌入提供者", "嵌入提供者", "Embedding provider"),
        Text::AlibabaEmbeddingHelp => (
            "阿里建议在嵌入路由里配置 alibaba-cn、text-embedding-v4 和 DashScope API Key。",
            "阿里建議在嵌入路由中設定 alibaba-cn、text-embedding-v4 與 DashScope API Key。",
            "For Alibaba, configure alibaba-cn, text-embedding-v4 and a DashScope API Key in embedding routes.",
        ),
        Text::EmbeddingModel => ("嵌入模型", "嵌入模型", "Embedding model"),
        Text::ModelNamePrefix => ("模型名称，可用", "模型名稱，可用", "Model name; use"),
        Text::RouteHint => ("hint:模式名", "hint:模式名稱", "hint:pattern"),
        Text::RouteReference => ("引用路由。", "引用路由。", "to reference a route."),
        Text::Dimensions => ("嵌入维度", "嵌入維度", "Embedding dimensions"),
        Text::DimensionsSuffix => ("维", "維", "dimensions"),
        Text::VectorWeight => ("向量权重", "向量權重", "Vector weight"),
        Text::KeywordWeight => ("关键词权重", "關鍵字權重", "Keyword weight"),
        Text::MinRelevance => ("最小相关度", "最低相關度", "Minimum relevance"),
        Text::QdrantHelp => (
            "仅在 backend=qdrant 时显示，用于配置向量数据库地址、集合和 API Key。",
            "僅於 backend=qdrant 時顯示，設定向量資料庫位址、集合與 API Key。",
            "Shown only for backend=qdrant; configure the vector database URL, collection and API Key.",
        ),
        Text::QdrantUrl => ("Qdrant 地址", "Qdrant 位址", "Qdrant URL"),
        Text::QdrantUrlHelp => (
            "Qdrant 服务的 HTTP 地址。",
            "Qdrant 服務的 HTTP 位址。",
            "HTTP URL of the Qdrant service.",
        ),
        Text::QdrantCollection => ("Qdrant 集合", "Qdrant 集合", "Qdrant collection"),
        Text::QdrantCollectionHelp => {
            ("记忆向量集合名称。", "記憶向量集合名稱。", "Name of the memory vector collection.")
        }
        Text::QdrantKey => ("Qdrant API 密钥", "Qdrant API 金鑰", "Qdrant API key"),
        Text::QdrantKeyHelp => {
            ("可选的鉴权密钥。", "選填的驗證金鑰。", "Optional authentication key.")
        }
        Text::Optional => ("可选", "選填", "Optional"),
        Text::MemoryConfig => ("记忆系统配置", "記憶系統設定", "Memory system configuration"),
        Text::MemoryIntro => (
            "统一配置后端、保留策略、缓存和向量检索参数。",
            "統一設定後端、保留策略、快取與向量檢索參數。",
            "Configure the backend, retention, caching and vector retrieval parameters.",
        ),

        Text::StorageKind => ("存储类型", "儲存類型", "Storage type"),
        Text::StorageKindHelp => {
            ("选择后端数据库类型。", "選擇後端資料庫類型。", "Choose the backend database type.")
        }
        Text::DatabaseUrl => ("数据库地址", "資料庫位址", "Database URL"),
        Text::DatabaseHint => (
            "远程 SQL 存储使用连接串；留空时不写入数据库地址。",
            "遠端 SQL 儲存使用連線字串；留空時不寫入資料庫位址。",
            "Use a connection string for remote SQL storage; leave blank to omit the database URL.",
        ),
        Text::Schema => ("Schema 名称", "Schema 名稱", "Schema name"),
        Text::SchemaHelp => {
            ("数据库 schema 名称。", "資料庫 schema 名稱。", "Database schema name.")
        }
        Text::Table => ("数据表", "資料表", "Table"),
        Text::TableHelp => (
            "保存记录所使用的数据表名。",
            "儲存記錄使用的資料表名稱。",
            "Table name used to store records.",
        ),
        Text::Timeout => ("连接超时", "連線逾時", "Connection timeout"),
        Text::TimeoutPlaceholder => {
            ("秒；留空表示使用默认", "秒；留空表示使用預設值", "Seconds; leave blank for default")
        }
        Text::TimeoutHint => (
            "仅接受整数秒；留空会写入 None。",
            "僅接受整數秒；留空會寫入 None。",
            "Whole seconds only; blank saves None.",
        ),
        Text::Tls => ("TLS 加密", "TLS 加密", "TLS encryption"),
        Text::TlsHelp => (
            "控制远程存储连接是否启用 TLS。",
            "控制遠端儲存連線是否啟用 TLS。",
            "Choose whether remote storage connections use TLS.",
        ),
        Text::EnableTls => {
            ("启用存储连接 TLS", "啟用儲存連線 TLS", "Enable TLS for storage connections")
        }
        Text::ConnectionParams => ("连接参数", "連線參數", "Connection parameters"),
        Text::ConnectionIntro => (
            "选择数据库类型并配置基础连接信息。",
            "選擇資料庫類型並設定基本連線資訊。",
            "Choose the database type and configure connection details.",
        ),
        Text::NamingTimeout => ("命名与超时", "命名與逾時", "Names and timeout"),
        Text::NamingHelp => (
            "配置 schema、表名和连接超时。",
            "設定 schema、資料表名稱與連線逾時。",
            "Configure the schema, table name and connection timeout.",
        ),
        Text::Security => ("安全", "安全", "Security"),
        Text::StorageSecurityHelp => (
            "配置与远程存储的加密连接策略。",
            "設定遠端儲存的加密連線策略。",
            "Configure encrypted connections to remote storage.",
        ),
        Text::Backend => ("后端", "後端", "Backend"),
        Text::BackendHelp => (
            "选择运行时观测后端。",
            "選擇執行環境的觀測後端。",
            "Choose the runtime observability backend.",
        ),
        Text::None => ("无", "無", "None"),
        Text::Log => ("日志", "日誌", "Log"),
        Text::OtelEndpoint => ("OTEL 端点", "OTEL 端點", "OTEL endpoint"),
        Text::OtelEndpointHelp => (
            "仅 backend=otel 时生效。",
            "僅於 backend=otel 時生效。",
            "Applies only when backend=otel.",
        ),
        Text::OtelService => ("OTEL 服务名", "OTEL 服務名稱", "OTEL service name"),
        Text::OtelServiceHelp => (
            "OTel 上报时使用的 service.name。",
            "OTel 回報使用的 service.name。",
            "The service.name used for OTel reporting.",
        ),
        Text::Trace => ("运行时追踪", "執行環境追蹤", "Runtime tracing"),
        Text::TraceHelp => (
            "选择 none、rolling 或 full 模式。",
            "選擇 none、rolling 或 full 模式。",
            "Choose none, rolling or full mode.",
        ),
        Text::Rolling => ("滚动", "滾動", "Rolling"),
        Text::Full => ("完整", "完整", "Full"),
        Text::TraceFile => ("追踪文件", "追蹤檔案", "Trace file"),
        Text::TraceFileHelp => {
            ("运行时追踪文件路径。", "執行環境追蹤檔案路徑。", "Path to the runtime trace file.")
        }
        Text::RollingEntries => ("rolling 条数", "rolling 筆數", "Rolling entries"),
        Text::RollingHelp => (
            "rolling 模式下保留的最大条目数。",
            "rolling 模式下保留的最大項目數。",
            "Maximum entries retained in rolling mode.",
        ),
        Text::ObservabilityConfig => {
            ("可观测性配置", "可觀測性設定", "Observability configuration")
        }
        Text::ObservabilityIntro => (
            "配置运行时观测后端、OTEL 上报和追踪存储策略。",
            "設定執行環境觀測後端、OTEL 回報與追蹤儲存策略。",
            "Configure runtime observability, OTEL reporting and trace storage.",
        ),
        Text::ObservabilityBackend => ("观测后端", "觀測後端", "Observability backend"),
        Text::ObservabilityBackendHelp => (
            "选择观测后端并配置 OTel 端点。",
            "選擇觀測後端並設定 OTel 端點。",
            "Choose an observability backend and configure its OTel endpoint.",
        ),
        Text::TraceStorageHelp => (
            "控制 trace 模式、文件路径和 rolling 保留量。",
            "控制 trace 模式、檔案路徑與 rolling 保留量。",
            "Control the trace mode, file path and rolling retention.",
        ),
        Text::ObservabilityHelpTitle => {
            ("可观测性配置帮助", "可觀測性設定說明", "Observability configuration help")
        }

        Text::RuntimeKind => ("运行时类型", "執行環境類型", "Runtime type"),
        Text::EnvironmentHelp => (
            "选择 native 或 docker 执行环境。",
            "選擇 native 或 docker 執行環境。",
            "Choose the native or docker execution environment.",
        ),
        Text::Auto => ("自动", "自動", "Automatic"),
        Text::Enabled => ("启用", "啟用", "Enabled"),
        Text::Disabled => ("禁用", "停用", "Disabled"),
        Text::EnableReasoning => ("启用推理", "啟用推理", "Enable reasoning"),
        Text::ReasoningHelp => (
            "覆盖 runtime.reasoning_enabled 的兼容配置。",
            "覆寫 runtime.reasoning_enabled 的相容設定。",
            "Override the runtime.reasoning_enabled compatibility setting.",
        ),
        Text::Default => ("默认", "預設", "Default"),
        Text::Minimal => ("最小", "最小", "Minimal"),
        Text::Low => ("低", "低", "Low"),
        Text::Medium => ("中", "中", "Medium"),
        Text::High => ("高", "高", "High"),
        Text::ExtraHigh => ("超高", "超高", "Extra high"),
        Text::ReasoningLevel => ("推理级别", "推理等級", "Reasoning level"),
        Text::LevelHelp => (
            "兼容别名 runtime.reasoning_level。",
            "相容別名 runtime.reasoning_level。",
            "Compatibility alias runtime.reasoning_level.",
        ),
        Text::DockerConfig => ("Docker 配置", "Docker 設定", "Docker configuration"),
        Text::DockerHelp => (
            "当 kind=docker 时生效，用于控制镜像、资源限制和工作区挂载。",
            "於 kind=docker 時生效，控制映像、資源限制與工作區掛載。",
            "Applies when kind=docker; controls the image, resource limits and workspace mounts.",
        ),
        Text::Image => ("镜像 (image)", "映像 (image)", "Image (image)"),
        Text::ImageHelp => ("默认执行镜像。", "預設執行映像。", "Default execution image."),
        Text::Network => ("网络模式 (network)", "網路模式 (network)", "Network mode (network)"),
        Text::NetworkHelp => {
            ("控制容器的网络模式。", "控制容器的網路模式。", "Control the container network mode.")
        }
        Text::MemoryLimit => (
            "内存限制 MB (memory_limit_mb)",
            "記憶體限制 MB (memory_limit_mb)",
            "Memory limit MB (memory_limit_mb)",
        ),
        Text::UnlimitedHelp => {
            ("留空表示不限制。", "留空表示不限制。", "Leave blank for no limit.")
        }
        Text::Unlimited => ("留空表示不限制", "留空表示不限制", "Leave blank for no limit"),
        Text::CpuLimit => ("CPU 限制 (cpu_limit)", "CPU 限制 (cpu_limit)", "CPU limit (cpu_limit)"),
        Text::CpuExampleHelp => {
            ("例如 1 / 1.5 / 2。", "例如 1 / 1.5 / 2。", "For example 1 / 1.5 / 2.")
        }
        Text::CpuExample => ("例如 1 / 1.5 / 2", "例如 1 / 1.5 / 2", "For example 1 / 1.5 / 2"),
        Text::ReadOnly => (
            "只读根文件系统 (read_only_rootfs)",
            "唯讀根檔案系統 (read_only_rootfs)",
            "Read-only root filesystem (read_only_rootfs)",
        ),
        Text::ReadOnlyHelp => (
            "限制容器根文件系统为只读。",
            "將容器根檔案系統限制為唯讀。",
            "Make the container root filesystem read-only.",
        ),
        Text::Mount => (
            "挂载工作区 (mount_workspace)",
            "掛載工作區 (mount_workspace)",
            "Mount workspace (mount_workspace)",
        ),
        Text::MountHelp => (
            "决定是否把工作区挂载进容器。",
            "決定是否將工作區掛載至容器。",
            "Choose whether to mount the workspace in the container.",
        ),
        Text::AllowedRoots => (
            "允许的工作区根目录 (allowed_workspace_roots)",
            "允許的工作區根目錄 (allowed_workspace_roots)",
            "Allowed workspace roots (allowed_workspace_roots)",
        ),
        Text::RootsHelp => (
            "仅放行明确需要挂载的根目录。",
            "僅允許明確需要掛載的根目錄。",
            "Allow only the root directories that need mounting.",
        ),
        Text::PathsHint => {
            ("逗号或换行分隔路径", "以逗號或換行分隔路徑", "Paths separated by commas or newlines")
        }
        Text::RootsHint => (
            "建议仅放行明确需要挂载的根目录。",
            "建議僅允許明確需要掛載的根目錄。",
            "Allow only the root directories that need mounting.",
        ),
        Text::RuntimeConfig => ("运行时配置", "執行環境設定", "Runtime configuration"),
        Text::RuntimeIntro => (
            "配置 native、docker 执行环境及推理覆盖项。",
            "設定 native、docker 執行環境及推理覆寫項目。",
            "Configure native and docker execution environments and reasoning overrides.",
        ),
        Text::Basic => ("基础行为", "基本行為", "Basic behavior"),
        Text::KindHelp => ("运行时类型。", "執行環境類型。", "Runtime type."),
        Text::Overrides => ("推理覆盖", "推理覆寫", "Reasoning overrides"),
        Text::OverridesHelp => (
            "该区域用于配置 runtime.reasoning_enabled 与兼容别名 runtime.reasoning_level。",
            "設定 runtime.reasoning_enabled 與相容別名 runtime.reasoning_level。",
            "Configure runtime.reasoning_enabled and the compatibility alias runtime.reasoning_level.",
        ),
    };
    match locale {
        DesktopLocale::SimplifiedChinese => values.0,
        DesktopLocale::TraditionalChinese => values.1,
        DesktopLocale::English => values.2,
    }
}

/// Help content is selected at render time; configuration keys retain their spelling.
pub fn observability_help(locale: DesktopLocale) -> &'static str {
    match locale {
        DesktopLocale::SimplifiedChinese => {
            "可观测性配置说明\n\n一、作用\n- observability 用于控制运行时观测后端与追踪存储行为。\n\n二、字段含义\n1) backend\n- 类型: \"none\" | \"log\" | \"prometheus\" | \"otel\"\n\n2) otelendpoint\n- OTLP 采集端点，仅 backend = \"otel\" 时生效。\n\n3) otel_service_name\n- 上报到 OTel 的服务名，仅 backend = \"otel\" 时生效。\n\n4) runtime_trace_mode\n- 类型: \"none\" | \"rolling\" | \"full\"\n\n5) runtime_trace_path\n- 追踪文件路径；相对路径相对 workspace_dir 解析。\n\n6) runtime_trace_max_entries\n- rolling 模式下保留的最大条目数。\n\n三、示例\n{\n  \"observability\": {\n    \"backend\": \"none\",\n    \"otel_endpoint\": null,\n    \"otel_service_name\": null,\n    \"runtime_trace_mode\": \"none\",\n    \"runtime_trace_path\": \"state/runtime-trace.jsonl\",\n    \"runtime_trace_max_entries\": 200\n  }\n}\n"
        }
        DesktopLocale::TraditionalChinese => {
            "可觀測性設定說明\n\nobservability 控制觀測後端與追蹤儲存。\nbackend: none / log / prometheus / otel。\notel_endpoint: OTLP 收集端點，僅於 backend=otel 時生效。\notel_service_name: OTel 服務名稱，僅於 backend=otel 時生效。\nruntime_trace_mode: none / rolling / full。\nruntime_trace_path: 追蹤檔案路徑；相對路徑以 workspace_dir 為基準。\nruntime_trace_max_entries: rolling 模式保留的最大項目數。"
        }
        DesktopLocale::English => {
            "Observability configuration help\n\nobservability controls the observability backend and trace storage.\nbackend: none / log / prometheus / otel.\notel_endpoint: OTLP collector endpoint; applies only when backend=otel.\notel_service_name: OTel service name; applies only when backend=otel.\nruntime_trace_mode: none / rolling / full.\nruntime_trace_path: trace file path; relative paths resolve against workspace_dir.\nruntime_trace_max_entries: maximum entries retained in rolling mode."
        }
    }
}
