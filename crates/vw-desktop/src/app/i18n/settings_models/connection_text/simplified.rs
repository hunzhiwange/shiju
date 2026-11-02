use super::Text;
pub(super) fn text(key: Text) -> &'static str {
    match key {
        Text::ConfigureEmbeddingProvidersModelsDimensions => {
            "按场景配置嵌入提供商、模型、维度和 API Key；在“嵌入与检索”中用 hint:模式名 引用。"
        }
        Text::AddRoute => "添加路由",
        Text::QuickTemplates => "快捷模板",
        Text::InsertACommonConfigurationThen => {
            "一键插入常用配置，插入后仍可按需改 provider、模型、维度和 API Key。"
        }
        Text::AlibabaDashscope => "阿里 DashScope",
        Text::DoubaoVision250615 => "豆包 Vision 250615",
        Text::RouteRules => "路由规则",
        Text::EachRouteMapsAScenario => {
            "每条路由定义一个匹配模式，并映射到具体的嵌入 provider/model。"
        }
        Text::SavedSuccessfully => "保存成功",
        Text::NoEmbeddingRoutes => "暂无嵌入路由",
        Text::SelectATemplateOrAdd => "点击上方模板按钮或右上角「添加路由」创建第一条规则。",
        Text::Pattern => "匹配模式",
        Text::MatchesABusinessOrTask => "用于匹配业务或任务场景。",
        Text::Provider => "提供商",
        Text::ProviderUsedForEmbeddingRequests => "嵌入请求所使用的 provider。",
        Text::Model => "模型",
        Text::EmbeddingModelToUse => "具体使用的 embedding 模型。",
        Text::Dimensions => "维度",
        Text::LeaveEmptyToUseThe => "留空时按模型默认维度运行。",
        Text::OptionalCredentialOverrideUseA => "可选覆盖凭据；阿里填 DashScope API Key。",
        Text::PatternProviderModel => "匹配模式 -> 提供商/模型",
        Text::Delete => "删除",
        Text::Save => "保存",
        Text::Configuration => "配置",
        Text::SkeyManagement => "skey 管理",
        Text::ListenAddress => "监听地址",
        Text::Port => "端口",
        Text::PublicBinding => "公网绑定",
        Text::ReverseProxy => "反向代理",
        Text::Idempotency => "幂等性",
        Text::Key => "密钥",
        Text::Expiration => "过期时间",
        Text::Authentication => "鉴权",
        Text::StopService => "停止服务",
        Text::UninstallService => "卸载服务",
        Text::InstallService => "安装服务",
        Text::StartService => "启动服务",
        Text::RestartService => "重启服务",
        Text::QueryStatus => "查询状态",
        Text::ServiceCommand => "服务命令",
        Text::NeverExpires => "永不过期",
        Text::Mon => "一",
        Text::Tue => "二",
        Text::Wed => "三",
        Text::Thu => "四",
        Text::Fri => "五",
        Text::Sat => "六",
        Text::Sun => "日",
        Text::Collapse => "收起",
        Text::Calendar => "日历",
        Text::YyyyMmDdOrType => "YYYY-MM-DD，可手写",
        Text::PortOnWhichTheServer => "控制服务端 Gateway 的监听端口。",
        Text::HostAddress => "主机地址",
        Text::AddressOnWhichTheServer => "控制服务端 Gateway 的监听地址。",
        Text::ForNonLocalAddressesUse => {
            "若 host 非本地地址，建议仅在可信反向代理或隧道环境下使用。"
        }
        Text::EnableSkeyAuthentication => "启用 skey 鉴权",
        Text::ProtectedRequestsMustSupplyA => {
            "开启后，受保护请求必须通过 Authorization: Bearer 提供有效 skey。"
        }
        Text::ValidateSkeyThroughAuthorizationBearer => "通过 Authorization: Bearer 校验 skey",
        Text::AllowPublicBinding => "允许公网绑定",
        Text::AllowBindingToNonLocal => "允许绑定到非本地地址。",
        Text::AllowBindingToNonLocalText => "允许绑定到非本地地址",
        Text::TrustForwardedHeaders => "信任转发头",
        Text::EnableOnlyBehindATrusted => "仅在可信反向代理后开启。",
        Text::TrustXForwardedForX => "信任 X-Forwarded-For / X-Real-IP",
        Text::EnableForwardedHeadersOnlyWhen => "仅在你完全信任前置代理时开启 forwarded headers。",
        Text::SkeyName => "skey 名称",
        Text::NameUsedToIdentifyThis => "用于列表展示和识别该 skey。",
        Text::ExampleIphoneCiRunner => "例如 iPhone / CI Runner",
        Text::GenerateSkey => "生成 skey",
        Text::ExpirationDate => "过期日期",
        Text::TypeYyyyMmDdOr => "可手写 YYYY-MM-DD，也可点日历选择；为空表示永不过期。",
        Text::Copy => "复制",
        Text::OriginalSkey => "原始 skey",
        Text::ShownOnceCopyItNow => "仅展示一次，请立即复制。",
        Text::NoSkeysWithSkeyAuthentication => {
            "当前没有 skey。开启 skey 鉴权后，缺少有效 skey 的请求会被拒绝。"
        }
        Text::Enabled => "启用",
        Text::Disabled => "禁用",
        Text::WebhookRateLimitPerMinute => "Webhook速率限制/分钟",
        Text::MaximumWebhookRequestsAcceptedPer => "限制 /webhook 每分钟可接受的请求数。",
        Text::RequestsMin => "次/分钟",
        Text::MaximumRateLimitKeys => "速率限制最大键数",
        Text::LimitTheSizeOfThe => "约束网关内部 key map 规模。",
        Text::Keys => "个键",
        Text::IdempotencyTtlSeconds => "幂等性TTL(秒)",
        Text::LifetimeOfWebhookIdempotencyKeys => "控制 webhook 幂等 key 的存活时间。",
        Text::Seconds => "秒",
        Text::MaximumIdempotencyKeys => "幂等性最大键数",
        Text::MaximumKeysRetainedInThe => "控制幂等缓存允许保留的 key 数量。",
        Text::NodeControlEnabled => "节点控制.已启用",
        Text::EnableTheExperimentalNodeControl => "启用实验性的 node-control API。",
        Text::EnableExperimentalNodeControlApi => "启用实验性的 node-control API",
        Text::NodeControlAuthenticationToken => "节点控制.认证令牌",
        Text::AdditionalSharedTokenSentThrough => {
            "额外共享令牌，客户端需通过 X-Node-Control-Token 传递。"
        }
        Text::OptionalSharedToken => "可选共享令牌",
        Text::NodeControlAllowedNodeIds => "节点控制.允许的节点ID",
        Text::LeaveEmptyForNoExplicit => "留空表示不设置显式 allowlist；可使用 * 允许所有节点。",
        Text::CommaOrNewlineSeparatedSupports => "逗号或换行分隔，支持 *",
        Text::LeaveAllowedNodeIdsEmpty => {
            "allowed_node_ids 留空表示不设置显式 allowlist；可使用 * 允许所有节点。"
        }
        Text::ServiceManagement => "服务管理",
        Text::ManageOsServicesThroughShiju => {
            "通过本机 gateway 调用 shiju service 管理 OS 服务生命周期。"
        }
        Text::ConfigureTheGatewayHostAnd => {
            "控制网关监听 host / port。默认仅绑定回环地址，避免意外暴露到公网。"
        }
        Text::SecurityControls => "安全开关",
        Text::ConfigureSkeyAuthenticationPublicBinding => {
            "控制 skey 鉴权、公网绑定与反向代理头信任策略。"
        }
        Text::RateLimits => "速率限制",
        Text::LimitWebhookRequestsAndThe => "限制 /webhook 请求频率，并约束网关内部 key map 规模。",
        Text::LimitWebhookIdempotencyKeyLifetime => "控制 webhook 幂等 key 的存活时间与内存上限。",
        Text::ConfigureExperimentalNodeControlAuthentication => {
            "配置实验性的 node-control 协议，包括额外鉴权和允许的远端节点 ID。"
        }
        Text::ManageGatewaySkeysOnlyHashes => {
            "维护 gateway.skeys 列表。原始 skey 不保存，仅保存哈希、脱敏 skey、名称和过期时间。"
        }
        Text::ServerGatewaySettings => "服务端网关配置",
        Text::ConfigureTheGatewayListenAddress => {
            "配置 Gateway 的监听地址、skey 鉴权、限流和实验性 node-control 行为。"
        }
        Text::Cancel => "取消",
        Text::GatewaySettingsHelp => "Gateway 配置帮助",
        Text::NoGateways => "暂无网关",
        Text::AddGateway => "新增网关",
        Text::DeleteSelected => "删除当前",
        Text::RefreshStatus => "刷新状态",
        Text::GatewayName => "网关名称",
        Text::NameUsedToDistinguishClient => "用于区分多个客户端网关。",
        Text::LocalGateway => "本地网关",
        Text::TargetHost => "目标主机",
        Text::GatewayHostUsedByDesktop => "桌面端请求所连接的 Gateway 主机。",
        Text::TargetPort => "目标端口",
        Text::GatewayPortUsedByDesktop => "桌面端请求所连接的 Gateway 端口。",
        Text::PlatformId => "平台 ID",
        Text::PlatformTenantForRequestsTo => "当前网关请求所属的平台租户。",
        Text::CompanyId => "公司 ID",
        Text::CompanyTenantForRequestsTo => "当前网关请求所属的公司租户。",
        Text::OptionalSkeySentThroughAuthorization => {
            "可选 skey，会作为 Authorization: Bearer 发送。"
        }
        Text::OptionalSkey => "可选 skey",
        Text::ClientGateway => "客户端网关",
        Text::ConfigureAddressesAndCredentialsFor => {
            "配置 Desktop 作为客户端访问一个或多个 Gateway 时使用的地址与认证信息。"
        }
        Text::GatewayServices => "网关服务",
        Text::ManageMultipleClientGatewaysDesktop => {
            "可以维护多个客户端网关，当前选中的网关会用于 Desktop 的 Gateway 请求。"
        }
        Text::AuthenticationText => "认证",
        Text::OptionalSkeySentAsAuthorization => {
            "可选填写 skey。请求会通过 Authorization: Bearer <skey> 发送。"
        }
        Text::ConfirmClientGatewayDeletion => "确认删除客户端网关",
        Text::ConfirmDeletion => "确认删除",
        Text::ClientGatewaySettingsHelp => "客户端网关配置帮助",
        Text::WebSearch => "网页搜索",
        Text::HttpRequests => "网络请求",
        Text::Browser => "浏览器",
        Text::Proxy => "代理",
        Text::Tunnel => "隧道",
        Text::NetworkSettings => "网络配置",
        Text::SwitchTabsToConfigureWeb => {
            "通过标签切换网页搜索、网络请求、浏览器、代理与隧道配置。"
        }
        Text::EditModel => "编辑模型",
        Text::AddModel => "添加模型",
        Text::ModelId => "模型 ID",
        Text::ActualModelIdSentTo => "用于请求 provider 的真实 model_id。",
        Text::ExampleGpt4oMini => "例如 gpt-4o-mini",
        Text::DisplayName => "显示名称",
        Text::DisplayOnlyLeaveEmptyTo => "仅影响 UI 展示，可留空回退为 model_id。",
        Text::Optional => "可选",
        Text::ModelFamily => "模型家族",
        Text::OptionalFamilyMetadataDistinguishingModel => {
            "可选 family 元数据，用于区分同一厂商的模型系列。"
        }
        Text::ExampleQwenGptClaude => "例如 qwen、gpt、claude",
        Text::ContextLimitTokens => "上下文限制（token）",
        Text::TotalContextWindowLeaveEmpty => "总上下文窗口；留空时沿用内置基线或安全默认值。",
        Text::Example200000LeaveEmptyFor => "例如 200000，留空使用默认值",
        Text::MaximumInputTokens => "最大输入（token）",
        Text::MaximumInputWindowLeaveEmpty => "最大输入窗口；留空时沿用内置基线或安全默认值。",
        Text::MaximumOutputTokens => "最大输出（token）",
        Text::MaximumOutputWindowZeroCan => "最大输出窗口；0 可表示该模型不产生文本输出。",
        Text::Example4096LeaveEmptyFor => "例如 4096，留空使用默认值",
        Text::Reasoning => "推理",
        Text::ToolCalls => "工具调用",
        Text::Temperature => "温度",
        Text::Attachments => "附件",
        Text::InterleavedInputOutput => "交错输入输出",
        Text::Capabilities => "能力",
        Text::UnsetValuesUseTheBaseline => "未显式设置时沿用基线；新模型默认支持文本和工具调用。",
        Text::InputModalities => "输入模态",
        Text::ChooseProviderInputTypesLeave => {
            "选择 provider 接受的输入类型；留空时按文本模型默认值处理。"
        }
        Text::OutputModalities => "输出模态",
        Text::ChooseProviderOutputTypesLeave => {
            "选择 provider 返回的输出类型；留空时按文本模型默认值处理。"
        }
        Text::ModelInformation => "模型信息",
        Text::ModelIdIsSentIn => "模型 ID 会直接用于请求；显示名称和家族仅用于元数据。",
        Text::Limits => "限制",
        Text::EmptyValuesUseTheBuilt => "空值明确表示沿用内置基线或安全默认值，数值单位为 token。",
        Text::CapabilitiesAndModalities => "能力与模态",
        Text::SaveOnlyExplicitOverridesPreserving => {
            "只保存用户显式设置的覆盖，保持自动发现模型的其它配置。"
        }
        Text::EnterTheModelIdAnd => "填写模型 ID 和展示名称。",
        Text::ModelProviders => "模型提供商",
        Text::ManageConnectedPopularAndCustom => "统一管理已连接、热门和自定义的模型提供商入口。",
        Text::Syncing => "同步中…",
        Text::SyncFromModelsDev => "从 models.dev 同步",
        Text::Refreshing => "刷新中…",
        Text::Refresh => "刷新",
        Text::ConnectedProviders => "已连接的提供商",
        Text::ViewAvailableProvidersAndManage => {
            "查看当前可用的 provider，并管理密钥、编辑与断开连接。"
        }
        Text::NoConnectedProviders => "暂无已连接的提供商。",
        Text::ConfirmDisconnect => "确认断开",
        Text::AuthorizeAgain => "重新授权",
        Text::ChangeApiKey => "修改密钥",
        Text::Edit => "编辑",
        Text::Disconnect => "断开连接",
        Text::PopularProviders => "热门提供商",
        Text::MatchPopularPatternsAgainstThe => {
            "根据热门模式匹配目录中的 provider，快速建立常用连接。"
        }
        Text::OfficialRecommendation => "官方推荐",
        Text::Connected => "已连接",
        Text::Connect => "连接",
        Text::OfficialShijuModelServiceAvailable => "识矩官方模型服务，连接后可自动发现可用模型。",
        Text::NotFound => "未发现",
        Text::Remove => "移除",
        Text::DiscoverMoreProviders => "发现更多提供商",
        Text::BrowseTheModelCatalogAdd => {
            "从模型目录中浏览更多 provider，并快速加入热门或直接连接。"
        }
        Text::MoreModelProviders => "更多模型提供商",
        Text::CustomProvider => "自定义提供商",
        Text::ConnectAnOpenaiCompatibleProvider => "按 base URL 接入 OpenAI 兼容 provider。",
        Text::OpenaiCompatibleProvider => "OpenAI 兼容 provider",
        Text::AddACustomProviderAnd => "添加一个自定义 provider，并在后续编辑连接参数。",
        Text::AddProvider => "添加提供商",
        Text::EnterAndSaveTheApi => "录入并保存当前 Provider 的 API 密钥。",
        Text::ApiKey => "API 密钥",
        Text::EnterApiKey => "输入 API 密钥",
        Text::SaveApiKey => "保存密钥",
        Text::AlreadyInFavorites => "已在热门",
        Text::AddToFavorites => "添加到热门",
        Text::Loading => "加载中…",
        Text::NoModelProvidersAvailable => "暂无可用模型提供商",
        Text::NoMatchingModelProviders => "未找到匹配的模型提供商",
        Text::SearchAndFilter => "搜索与筛选",
        Text::FindCatalogProvidersByName => "按 provider 名称或 provider ID 快速定位可用目录。",
        Text::SearchModelProviders => "搜索模型提供商",
        Text::BrowseTheModelCatalogAnd => "浏览模型目录中的 provider，并快速连接或加入热门。",
        Text::EditProvider => "编辑提供商",
        Text::EditTheProviderConnectionAddress => {
            "调整现有 provider 的连接地址、默认鉴权信息和模型列表。"
        }
        Text::ConnectAnOpenaiCompatibleProviderText => {
            "按 base URL 接入一个与 OpenAI 兼容的 provider；模型可稍后配置。"
        }
        Text::ProviderId => "提供商 ID",
        Text::StableConfigurationIdReadOnly => "作为配置中的稳定标识，编辑模式下保持只读。",
        Text::StableConfigurationIdUsingLowercase => {
            "作为配置中的稳定标识，只允许小写字母、数字、连字符或下划线。"
        }
        Text::ProviderNameShownToUsers => "展示给用户的 provider 名称。",
        Text::MyAiProvider => "我是 AI 提供商",
        Text::BaseUrl => "基础 URL",
        Text::OpenaiCompatibleApiRootFor => {
            "OpenAI 兼容 API 根地址，例如 https://api.example.com/v1。"
        }
        Text::AutomaticAccountAuthorization => "账户自动授权",
        Text::SignInToYourShiju => "登录识矩账户后自动配置模型，无需填写模型密钥。",
        Text::AccountManagedKeyPlaintextIs => "密钥由账户管理，不显示明文",
        Text::OptionalDefaultKeySentIn => "可选默认密钥；填写后会作为 Authorization 头使用。",
        Text::AddHeader => "添加请求头",
        Text::HeaderName => "请求头名称",
        Text::HeaderValue => "请求头值",
        Text::Headers => "请求头",
        Text::AddHeadersForCustomAuthentication => {
            "为特殊鉴权场景补充额外 header，例如代理网关或自定义令牌头。"
        }
        Text::Discovering => "发现中…",
        Text::DiscoverModels => "自动发现模型",
        Text::All => "全部",
        Text::EnabledText => "已启用",
        Text::DisabledText => "已禁用",
        Text::LeaveEmptyDiscoverModelsAutomatically => {
            "可留空，或自动发现该 provider 的模型；也可手动添加并启用/禁用。"
        }
        Text::Filter => "过滤",
        Text::FilterModelsByAvailability => "按启用状态筛选模型列表。",
        Text::UnnamedModel => "未命名模型",
        Text::NoModelIdYet => "尚未填写 model_id",
        Text::NoModels => "暂无模型",
        Text::NoEnabledModels => "当前没有已启用的模型",
        Text::NoDisabledModels => "当前没有已禁用的模型",
        Text::BasicInformation => "基础信息",
        Text::SetTheProviderIdDisplay => "命名 provider，并指定展示名称与 OpenAI 兼容接口地址。",
        Text::AuthenticationAndHeaders => "认证与请求头",
        Text::SetADefaultApiKey => "设置默认 API Key，并为需要额外 header 的网关场景补充请求头。",
        Text::ManageTheProviderModelsUsed => {
            "维护该 provider 的模型列表，后续聊天与生成界面会复用这里的配置。"
        }
    }
}
