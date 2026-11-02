use super::Text;
pub(super) fn text(key: Text) -> &'static str {
    match key {
        Text::ConfigureEmbeddingProvidersModelsDimensions => {
            "按場景配置嵌入提供商、模型、維度和 API Key；在“嵌入與檢索”中用 hint:模式名 引用。"
        }
        Text::AddRoute => "添加路由",
        Text::QuickTemplates => "快捷模板",
        Text::InsertACommonConfigurationThen => {
            "一鍵插入常用配置，插入後仍可按需改 provider、模型、維度和 API Key。"
        }
        Text::AlibabaDashscope => "阿里 DashScope",
        Text::DoubaoVision250615 => "豆包 Vision 250615",
        Text::RouteRules => "路由規則",
        Text::EachRouteMapsAScenario => {
            "每條路由定義一個匹配模式，並映射到具體的嵌入 provider/model。"
        }
        Text::SavedSuccessfully => "保存成功",
        Text::NoEmbeddingRoutes => "暫無嵌入路由",
        Text::SelectATemplateOrAdd => "點擊上方模板按鈕或右上角「添加路由」創建第一條規則。",
        Text::Pattern => "匹配模式",
        Text::MatchesABusinessOrTask => "用於匹配業務或任務場景。",
        Text::Provider => "提供商",
        Text::ProviderUsedForEmbeddingRequests => "嵌入請求所使用的 provider。",
        Text::Model => "模型",
        Text::EmbeddingModelToUse => "具體使用的 embedding 模型。",
        Text::Dimensions => "維度",
        Text::LeaveEmptyToUseThe => "留空時按模型默認維度運行。",
        Text::OptionalCredentialOverrideUseA => "可選覆蓋憑據；阿里填 DashScope API Key。",
        Text::PatternProviderModel => "匹配模式 -> 提供商/模型",
        Text::Delete => "刪除",
        Text::Save => "保存",
        Text::Configuration => "配置",
        Text::SkeyManagement => "skey 管理",
        Text::ListenAddress => "監聽地址",
        Text::Port => "端口",
        Text::PublicBinding => "公網綁定",
        Text::ReverseProxy => "反向代理",
        Text::Idempotency => "冪等性",
        Text::Key => "密鑰",
        Text::Expiration => "過期時間",
        Text::Authentication => "鑒權",
        Text::StopService => "停止服務",
        Text::UninstallService => "卸載服務",
        Text::InstallService => "安裝服務",
        Text::StartService => "啓動服務",
        Text::RestartService => "重啓服務",
        Text::QueryStatus => "查詢狀態",
        Text::ServiceCommand => "服務命令",
        Text::NeverExpires => "永不過期",
        Text::Mon => "一",
        Text::Tue => "二",
        Text::Wed => "三",
        Text::Thu => "四",
        Text::Fri => "五",
        Text::Sat => "六",
        Text::Sun => "日",
        Text::Collapse => "收起",
        Text::Calendar => "日曆",
        Text::YyyyMmDdOrType => "YYYY-MM-DD，可手寫",
        Text::PortOnWhichTheServer => "控制服務端 Gateway 的監聽端口。",
        Text::HostAddress => "主機地址",
        Text::AddressOnWhichTheServer => "控制服務端 Gateway 的監聽地址。",
        Text::ForNonLocalAddressesUse => {
            "若 host 非本地地址，建議僅在可信反向代理或隧道環境下使用。"
        }
        Text::EnableSkeyAuthentication => "啓用 skey 鑒權",
        Text::ProtectedRequestsMustSupplyA => {
            "開啓後，受保護請求必須通過 Authorization: Bearer 提供有效 skey。"
        }
        Text::ValidateSkeyThroughAuthorizationBearer => "通過 Authorization: Bearer 校驗 skey",
        Text::AllowPublicBinding => "允許公網綁定",
        Text::AllowBindingToNonLocal => "允許綁定到非本地地址。",
        Text::AllowBindingToNonLocalText => "允許綁定到非本地地址",
        Text::TrustForwardedHeaders => "信任轉發頭",
        Text::EnableOnlyBehindATrusted => "僅在可信反向代理後開啓。",
        Text::TrustXForwardedForX => "信任 X-Forwarded-For / X-Real-IP",
        Text::EnableForwardedHeadersOnlyWhen => "僅在你完全信任前置代理時開啓 forwarded headers。",
        Text::SkeyName => "skey 名稱",
        Text::NameUsedToIdentifyThis => "用於列表展示和識別該 skey。",
        Text::ExampleIphoneCiRunner => "例如 iPhone / CI Runner",
        Text::GenerateSkey => "生成 skey",
        Text::ExpirationDate => "過期日期",
        Text::TypeYyyyMmDdOr => "可手寫 YYYY-MM-DD，也可點日曆選擇；為空表示永不過期。",
        Text::Copy => "複製",
        Text::OriginalSkey => "原始 skey",
        Text::ShownOnceCopyItNow => "僅展示一次，請立即複製。",
        Text::NoSkeysWithSkeyAuthentication => {
            "當前沒有 skey。開啓 skey 鑒權後，缺少有效 skey 的請求會被拒絕。"
        }
        Text::Enabled => "啓用",
        Text::Disabled => "禁用",
        Text::WebhookRateLimitPerMinute => "Webhook速率限制/分鐘",
        Text::MaximumWebhookRequestsAcceptedPer => "限制 /webhook 每分鐘可接受的請求數。",
        Text::RequestsMin => "次/分鐘",
        Text::MaximumRateLimitKeys => "速率限制最大鍵數",
        Text::LimitTheSizeOfThe => "約束網關內部 key map 規模。",
        Text::Keys => "個鍵",
        Text::IdempotencyTtlSeconds => "冪等性TTL(秒)",
        Text::LifetimeOfWebhookIdempotencyKeys => "控制 webhook 冪等 key 的存活時間。",
        Text::Seconds => "秒",
        Text::MaximumIdempotencyKeys => "冪等性最大鍵數",
        Text::MaximumKeysRetainedInThe => "控制冪等緩存允許保留的 key 數量。",
        Text::NodeControlEnabled => "節點控制.已啓用",
        Text::EnableTheExperimentalNodeControl => "啓用實驗性的 node-control API。",
        Text::EnableExperimentalNodeControlApi => "啓用實驗性的 node-control API",
        Text::NodeControlAuthenticationToken => "節點控制.認證令牌",
        Text::AdditionalSharedTokenSentThrough => {
            "額外共享令牌，客戶端需通過 X-Node-Control-Token 傳遞。"
        }
        Text::OptionalSharedToken => "可選共享令牌",
        Text::NodeControlAllowedNodeIds => "節點控制.允許的節點ID",
        Text::LeaveEmptyForNoExplicit => "留空表示不設置顯式 allowlist；可使用 * 允許所有節點。",
        Text::CommaOrNewlineSeparatedSupports => "逗號或換行分隔，支持 *",
        Text::LeaveAllowedNodeIdsEmpty => {
            "allowed_node_ids 留空表示不設置顯式 allowlist；可使用 * 允許所有節點。"
        }
        Text::ServiceManagement => "服務管理",
        Text::ManageOsServicesThroughShiju => {
            "通過本機 gateway 調用 shiju service 管理 OS 服務生命週期。"
        }
        Text::ConfigureTheGatewayHostAnd => {
            "控制網關監聽 host / port。默認僅綁定回環地址，避免意外暴露到公網。"
        }
        Text::SecurityControls => "安全開關",
        Text::ConfigureSkeyAuthenticationPublicBinding => {
            "控制 skey 鑒權、公網綁定與反向代理頭信任策略。"
        }
        Text::RateLimits => "速率限制",
        Text::LimitWebhookRequestsAndThe => "限制 /webhook 請求頻率，並約束網關內部 key map 規模。",
        Text::LimitWebhookIdempotencyKeyLifetime => "控制 webhook 冪等 key 的存活時間與內存上限。",
        Text::ConfigureExperimentalNodeControlAuthentication => {
            "配置實驗性的 node-control 協議，包括額外鑒權和允許的遠端節點 ID。"
        }
        Text::ManageGatewaySkeysOnlyHashes => {
            "維護 gateway.skeys 列表。原始 skey 不保存，僅保存哈希、脫敏 skey、名稱和過期時間。"
        }
        Text::ServerGatewaySettings => "服務端網關配置",
        Text::ConfigureTheGatewayListenAddress => {
            "配置 Gateway 的監聽地址、skey 鑒權、限流和實驗性 node-control 行為。"
        }
        Text::Cancel => "取消",
        Text::GatewaySettingsHelp => "Gateway 配置幫助",
        Text::NoGateways => "暫無網關",
        Text::AddGateway => "新增網關",
        Text::DeleteSelected => "刪除當前",
        Text::RefreshStatus => "刷新狀態",
        Text::GatewayName => "網關名稱",
        Text::NameUsedToDistinguishClient => "用於區分多個客戶端網關。",
        Text::LocalGateway => "本地網關",
        Text::TargetHost => "目標主機",
        Text::GatewayHostUsedByDesktop => "桌面端請求所連接的 Gateway 主機。",
        Text::TargetPort => "目標端口",
        Text::GatewayPortUsedByDesktop => "桌面端請求所連接的 Gateway 端口。",
        Text::PlatformId => "平台 ID",
        Text::PlatformTenantForRequestsTo => "當前網關請求所屬的平台租戶。",
        Text::CompanyId => "公司 ID",
        Text::CompanyTenantForRequestsTo => "當前網關請求所屬的公司租戶。",
        Text::OptionalSkeySentThroughAuthorization => {
            "可選 skey，會作為 Authorization: Bearer 發送。"
        }
        Text::OptionalSkey => "可選 skey",
        Text::ClientGateway => "客戶端網關",
        Text::ConfigureAddressesAndCredentialsFor => {
            "配置 Desktop 作為客戶端訪問一個或多個 Gateway 時使用的地址與認證信息。"
        }
        Text::GatewayServices => "網關服務",
        Text::ManageMultipleClientGatewaysDesktop => {
            "可以維護多個客戶端網關，當前選中的網關會用於 Desktop 的 Gateway 請求。"
        }
        Text::AuthenticationText => "認證",
        Text::OptionalSkeySentAsAuthorization => {
            "可選填寫 skey。請求會通過 Authorization: Bearer <skey> 發送。"
        }
        Text::ConfirmClientGatewayDeletion => "確認刪除客戶端網關",
        Text::ConfirmDeletion => "確認刪除",
        Text::ClientGatewaySettingsHelp => "客戶端網關配置幫助",
        Text::WebSearch => "網頁搜索",
        Text::HttpRequests => "網絡請求",
        Text::Browser => "瀏覽器",
        Text::Proxy => "代理",
        Text::Tunnel => "隧道",
        Text::NetworkSettings => "網絡配置",
        Text::SwitchTabsToConfigureWeb => {
            "通過標籤切換網頁搜索、網絡請求、瀏覽器、代理與隧道配置。"
        }
        Text::EditModel => "編輯模型",
        Text::AddModel => "添加模型",
        Text::ModelId => "模型 ID",
        Text::ActualModelIdSentTo => "用於請求 provider 的真實 model_id。",
        Text::ExampleGpt4oMini => "例如 gpt-4o-mini",
        Text::DisplayName => "顯示名稱",
        Text::DisplayOnlyLeaveEmptyTo => "僅影響 UI 展示，可留空回退為 model_id。",
        Text::Optional => "可選",
        Text::ModelFamily => "模型家族",
        Text::OptionalFamilyMetadataDistinguishingModel => {
            "可選 family 元數據，用於區分同一廠商的模型系列。"
        }
        Text::ExampleQwenGptClaude => "例如 qwen、gpt、claude",
        Text::ContextLimitTokens => "上下文限制（token）",
        Text::TotalContextWindowLeaveEmpty => "總上下文窗口；留空時沿用內置基線或安全默認值。",
        Text::Example200000LeaveEmptyFor => "例如 200000，留空使用默認值",
        Text::MaximumInputTokens => "最大輸入（token）",
        Text::MaximumInputWindowLeaveEmpty => "最大輸入窗口；留空時沿用內置基線或安全默認值。",
        Text::MaximumOutputTokens => "最大輸出（token）",
        Text::MaximumOutputWindowZeroCan => "最大輸出窗口；0 可表示該模型不產生文本輸出。",
        Text::Example4096LeaveEmptyFor => "例如 4096，留空使用默認值",
        Text::Reasoning => "推理",
        Text::ToolCalls => "工具調用",
        Text::Temperature => "溫度",
        Text::Attachments => "附件",
        Text::InterleavedInputOutput => "交錯輸入輸出",
        Text::Capabilities => "能力",
        Text::UnsetValuesUseTheBaseline => "未顯式設置時沿用基線；新模型默認支持文本和工具調用。",
        Text::InputModalities => "輸入模態",
        Text::ChooseProviderInputTypesLeave => {
            "選擇 provider 接受的輸入類型；留空時按文本模型默認值處理。"
        }
        Text::OutputModalities => "輸出模態",
        Text::ChooseProviderOutputTypesLeave => {
            "選擇 provider 返回的輸出類型；留空時按文本模型默認值處理。"
        }
        Text::ModelInformation => "模型信息",
        Text::ModelIdIsSentIn => "模型 ID 會直接用於請求；顯示名稱和家族僅用於元數據。",
        Text::Limits => "限制",
        Text::EmptyValuesUseTheBuilt => "空值明確表示沿用內置基線或安全默認值，數值單位為 token。",
        Text::CapabilitiesAndModalities => "能力與模態",
        Text::SaveOnlyExplicitOverridesPreserving => {
            "只保存用戶顯式設置的覆蓋，保持自動發現模型的其它配置。"
        }
        Text::EnterTheModelIdAnd => "填寫模型 ID 和展示名稱。",
        Text::ModelProviders => "模型提供商",
        Text::ManageConnectedPopularAndCustom => "統一管理已連接、熱門和自定義的模型提供商入口。",
        Text::Syncing => "同步中…",
        Text::SyncFromModelsDev => "從 models.dev 同步",
        Text::Refreshing => "刷新中…",
        Text::Refresh => "刷新",
        Text::ConnectedProviders => "已連接的提供商",
        Text::ViewAvailableProvidersAndManage => {
            "查看當前可用的 provider，並管理密鑰、編輯與斷開連接。"
        }
        Text::NoConnectedProviders => "暫無已連接的提供商。",
        Text::ConfirmDisconnect => "確認斷開",
        Text::AuthorizeAgain => "重新授權",
        Text::ChangeApiKey => "修改密鑰",
        Text::Edit => "編輯",
        Text::Disconnect => "斷開連接",
        Text::PopularProviders => "熱門提供商",
        Text::MatchPopularPatternsAgainstThe => {
            "根據熱門模式匹配目錄中的 provider，快速建立常用連接。"
        }
        Text::OfficialRecommendation => "官方推薦",
        Text::Connected => "已連接",
        Text::Connect => "連接",
        Text::OfficialShijuModelServiceAvailable => "識矩官方模型服務，連接後可自動發現可用模型。",
        Text::NotFound => "未發現",
        Text::Remove => "移除",
        Text::DiscoverMoreProviders => "發現更多提供商",
        Text::BrowseTheModelCatalogAdd => {
            "從模型目錄中瀏覽更多 provider，並快速加入熱門或直接連接。"
        }
        Text::MoreModelProviders => "更多模型提供商",
        Text::CustomProvider => "自定義提供商",
        Text::ConnectAnOpenaiCompatibleProvider => "按 base URL 接入 OpenAI 兼容 provider。",
        Text::OpenaiCompatibleProvider => "OpenAI 兼容 provider",
        Text::AddACustomProviderAnd => "添加一個自定義 provider，並在後續編輯連接參數。",
        Text::AddProvider => "添加提供商",
        Text::EnterAndSaveTheApi => "錄入並保存當前 Provider 的 API 密鑰。",
        Text::ApiKey => "API 密鑰",
        Text::EnterApiKey => "輸入 API 密鑰",
        Text::SaveApiKey => "保存密鑰",
        Text::AlreadyInFavorites => "已在熱門",
        Text::AddToFavorites => "添加到熱門",
        Text::Loading => "加載中…",
        Text::NoModelProvidersAvailable => "暫無可用模型提供商",
        Text::NoMatchingModelProviders => "未找到匹配的模型提供商",
        Text::SearchAndFilter => "搜索與篩選",
        Text::FindCatalogProvidersByName => "按 provider 名稱或 provider ID 快速定位可用目錄。",
        Text::SearchModelProviders => "搜索模型提供商",
        Text::BrowseTheModelCatalogAnd => "瀏覽模型目錄中的 provider，並快速連接或加入熱門。",
        Text::EditProvider => "編輯提供商",
        Text::EditTheProviderConnectionAddress => {
            "調整現有 provider 的連接地址、默認鑒權信息和模型列表。"
        }
        Text::ConnectAnOpenaiCompatibleProviderText => {
            "按 base URL 接入一個與 OpenAI 兼容的 provider；模型可稍後配置。"
        }
        Text::ProviderId => "提供商 ID",
        Text::StableConfigurationIdReadOnly => "作為配置中的穩定標識，編輯模式下保持只讀。",
        Text::StableConfigurationIdUsingLowercase => {
            "作為配置中的穩定標識，只允許小寫字母、數字、連字符或下划線。"
        }
        Text::ProviderNameShownToUsers => "展示給用戶的 provider 名稱。",
        Text::MyAiProvider => "我是 AI 提供商",
        Text::BaseUrl => "基礎 URL",
        Text::OpenaiCompatibleApiRootFor => {
            "OpenAI 兼容 API 根地址，例如 https://api.example.com/v1。"
        }
        Text::AutomaticAccountAuthorization => "賬戶自動授權",
        Text::SignInToYourShiju => "登錄識矩賬戶後自動配置模型，無需填寫模型密鑰。",
        Text::AccountManagedKeyPlaintextIs => "密鑰由賬戶管理，不顯示明文",
        Text::OptionalDefaultKeySentIn => "可選默認密鑰；填寫後會作為 Authorization 頭使用。",
        Text::AddHeader => "添加請求頭",
        Text::HeaderName => "請求頭名稱",
        Text::HeaderValue => "請求頭值",
        Text::Headers => "請求頭",
        Text::AddHeadersForCustomAuthentication => {
            "為特殊鑒權場景補充額外 header，例如代理網關或自定義令牌頭。"
        }
        Text::Discovering => "發現中…",
        Text::DiscoverModels => "自動發現模型",
        Text::All => "全部",
        Text::EnabledText => "已啓用",
        Text::DisabledText => "已禁用",
        Text::LeaveEmptyDiscoverModelsAutomatically => {
            "可留空，或自動發現該 provider 的模型；也可手動添加並啓用/禁用。"
        }
        Text::Filter => "過濾",
        Text::FilterModelsByAvailability => "按啓用狀態篩選模型列表。",
        Text::UnnamedModel => "未命名模型",
        Text::NoModelIdYet => "尚未填寫 model_id",
        Text::NoModels => "暫無模型",
        Text::NoEnabledModels => "當前沒有已啓用的模型",
        Text::NoDisabledModels => "當前沒有已禁用的模型",
        Text::BasicInformation => "基礎信息",
        Text::SetTheProviderIdDisplay => "命名 provider，並指定展示名稱與 OpenAI 兼容接口地址。",
        Text::AuthenticationAndHeaders => "認證與請求頭",
        Text::SetADefaultApiKey => "設置默認 API Key，並為需要額外 header 的網關場景補充請求頭。",
        Text::ManageTheProviderModelsUsed => {
            "維護該 provider 的模型列表，後續聊天與生成界面會復用這裡的配置。"
        }
    }
}
