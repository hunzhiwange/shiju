use super::Text;
pub(super) fn text(key: Text) -> &'static str {
    match key {
        Text::ConfigureEmbeddingProvidersModelsDimensions => {
            "Configure embedding providers, models, dimensions and API keys by scenario; reference them with hint:pattern in Embedding and retrieval."
        }
        Text::AddRoute => "Add route",
        Text::QuickTemplates => "Quick templates",
        Text::InsertACommonConfigurationThen => {
            "Insert a common configuration, then adjust its provider, model, dimensions and API key."
        }
        Text::AlibabaDashscope => "Alibaba DashScope",
        Text::DoubaoVision250615 => "Doubao Vision 250615",
        Text::RouteRules => "Route rules",
        Text::EachRouteMapsAScenario => {
            "Each route maps a scenario pattern to an embedding provider/model."
        }
        Text::SavedSuccessfully => "Saved successfully",
        Text::NoEmbeddingRoutes => "No embedding routes",
        Text::SelectATemplateOrAdd => "Select a template or Add route to create your first rule.",
        Text::Pattern => "Pattern",
        Text::MatchesABusinessOrTask => "Matches a business or task scenario.",
        Text::Provider => "Provider",
        Text::ProviderUsedForEmbeddingRequests => "Provider used for embedding requests.",
        Text::Model => "Model",
        Text::EmbeddingModelToUse => "Embedding model to use.",
        Text::Dimensions => "Dimensions",
        Text::LeaveEmptyToUseThe => "Leave empty to use the model default dimensions.",
        Text::OptionalCredentialOverrideUseA => {
            "Optional credential override; use a DashScope API key for Alibaba."
        }
        Text::PatternProviderModel => "Pattern -> Provider/model",
        Text::Delete => "Delete",
        Text::Save => "Save",
        Text::Configuration => "Configuration",
        Text::SkeyManagement => "skey management",
        Text::ListenAddress => "Listen address",
        Text::Port => "Port",
        Text::PublicBinding => "Public binding",
        Text::ReverseProxy => "Reverse proxy",
        Text::Idempotency => "Idempotency",
        Text::Key => "Key",
        Text::Expiration => "Expiration",
        Text::Authentication => "Authentication",
        Text::StopService => "Stop service",
        Text::UninstallService => "Uninstall service",
        Text::InstallService => "Install service",
        Text::StartService => "Start service",
        Text::RestartService => "Restart service",
        Text::QueryStatus => "Query status",
        Text::ServiceCommand => "Service command",
        Text::NeverExpires => "Never expires",
        Text::Mon => "Mon",
        Text::Tue => "Tue",
        Text::Wed => "Wed",
        Text::Thu => "Thu",
        Text::Fri => "Fri",
        Text::Sat => "Sat",
        Text::Sun => "Sun",
        Text::Collapse => "Collapse",
        Text::Calendar => "Calendar",
        Text::YyyyMmDdOrType => "YYYY-MM-DD, or type a date",
        Text::PortOnWhichTheServer => "Port on which the server Gateway listens.",
        Text::HostAddress => "Host address",
        Text::AddressOnWhichTheServer => "Address on which the server Gateway listens.",
        Text::ForNonLocalAddressesUse => {
            "For non-local addresses, use a trusted reverse proxy or tunnel."
        }
        Text::EnableSkeyAuthentication => "Enable skey authentication",
        Text::ProtectedRequestsMustSupplyA => {
            "Protected requests must supply a valid skey through Authorization: Bearer."
        }
        Text::ValidateSkeyThroughAuthorizationBearer => {
            "Validate skey through Authorization: Bearer"
        }
        Text::AllowPublicBinding => "Allow public binding",
        Text::AllowBindingToNonLocal => "Allow binding to non-local addresses.",
        Text::AllowBindingToNonLocalText => "Allow binding to non-local addresses",
        Text::TrustForwardedHeaders => "Trust forwarded headers",
        Text::EnableOnlyBehindATrusted => "Enable only behind a trusted reverse proxy.",
        Text::TrustXForwardedForX => "Trust X-Forwarded-For / X-Real-IP",
        Text::EnableForwardedHeadersOnlyWhen => {
            "Enable forwarded headers only when you fully trust the upstream proxy."
        }
        Text::SkeyName => "skey name",
        Text::NameUsedToIdentifyThis => "Name used to identify this skey in the list.",
        Text::ExampleIphoneCiRunner => "Example: iPhone / CI Runner",
        Text::GenerateSkey => "Generate skey",
        Text::ExpirationDate => "Expiration date",
        Text::TypeYyyyMmDdOr => "Type YYYY-MM-DD or select a date. Leave empty for no expiration.",
        Text::Copy => "Copy",
        Text::OriginalSkey => "Original skey",
        Text::ShownOnceCopyItNow => "Shown once. Copy it now.",
        Text::NoSkeysWithSkeyAuthentication => {
            "No skeys. With skey authentication enabled, requests without a valid skey are rejected."
        }
        Text::Enabled => "Enabled",
        Text::Disabled => "Disabled",
        Text::WebhookRateLimitPerMinute => "Webhook rate limit per minute",
        Text::MaximumWebhookRequestsAcceptedPer => "Maximum /webhook requests accepted per minute.",
        Text::RequestsMin => "requests/min",
        Text::MaximumRateLimitKeys => "Maximum rate limit keys",
        Text::LimitTheSizeOfThe => "Limit the size of the Gateway key map.",
        Text::Keys => "keys",
        Text::IdempotencyTtlSeconds => "Idempotency TTL (seconds)",
        Text::LifetimeOfWebhookIdempotencyKeys => "Lifetime of webhook idempotency keys.",
        Text::Seconds => "seconds",
        Text::MaximumIdempotencyKeys => "Maximum idempotency keys",
        Text::MaximumKeysRetainedInThe => "Maximum keys retained in the idempotency cache.",
        Text::NodeControlEnabled => "Node control enabled",
        Text::EnableTheExperimentalNodeControl => "Enable the experimental node-control API.",
        Text::EnableExperimentalNodeControlApi => "Enable experimental node-control API",
        Text::NodeControlAuthenticationToken => "Node control authentication token",
        Text::AdditionalSharedTokenSentThrough => {
            "Additional shared token sent through X-Node-Control-Token."
        }
        Text::OptionalSharedToken => "Optional shared token",
        Text::NodeControlAllowedNodeIds => "Node control allowed node IDs",
        Text::LeaveEmptyForNoExplicit => {
            "Leave empty for no explicit allowlist; use * to allow all nodes."
        }
        Text::CommaOrNewlineSeparatedSupports => "Comma or newline separated; supports *",
        Text::LeaveAllowedNodeIdsEmpty => {
            "Leave allowed_node_ids empty for no explicit allowlist; use * to allow all nodes."
        }
        Text::ServiceManagement => "Service management",
        Text::ManageOsServicesThroughShiju => {
            "Manage OS services through shiju service on the local Gateway."
        }
        Text::ConfigureTheGatewayHostAnd => {
            "Configure the Gateway host and port. Loopback is the default to prevent accidental public exposure."
        }
        Text::SecurityControls => "Security controls",
        Text::ConfigureSkeyAuthenticationPublicBinding => {
            "Configure skey authentication, public binding and trust of reverse proxy headers."
        }
        Text::RateLimits => "Rate limits",
        Text::LimitWebhookRequestsAndThe => "Limit /webhook requests and the Gateway key map size.",
        Text::LimitWebhookIdempotencyKeyLifetime => {
            "Limit webhook idempotency key lifetime and memory usage."
        }
        Text::ConfigureExperimentalNodeControlAuthentication => {
            "Configure experimental node-control authentication and allowed remote node IDs."
        }
        Text::ManageGatewaySkeysOnlyHashes => {
            "Manage gateway.skeys. Only hashes, masked keys, names and expiration dates are stored, never the original skey."
        }
        Text::ServerGatewaySettings => "Server Gateway settings",
        Text::ConfigureTheGatewayListenAddress => {
            "Configure the Gateway listen address, skey authentication, rate limits and experimental node-control."
        }
        Text::Cancel => "Cancel",
        Text::GatewaySettingsHelp => "Gateway settings help",
        Text::NoGateways => "No Gateways",
        Text::AddGateway => "Add Gateway",
        Text::DeleteSelected => "Delete selected",
        Text::RefreshStatus => "Refresh status",
        Text::GatewayName => "Gateway name",
        Text::NameUsedToDistinguishClient => "Name used to distinguish client Gateways.",
        Text::LocalGateway => "Local Gateway",
        Text::TargetHost => "Target host",
        Text::GatewayHostUsedByDesktop => "Gateway host used by Desktop requests.",
        Text::TargetPort => "Target port",
        Text::GatewayPortUsedByDesktop => "Gateway port used by Desktop requests.",
        Text::PlatformId => "Platform ID",
        Text::PlatformTenantForRequestsTo => "Platform tenant for requests to this Gateway.",
        Text::CompanyId => "Company ID",
        Text::CompanyTenantForRequestsTo => "Company tenant for requests to this Gateway.",
        Text::OptionalSkeySentThroughAuthorization => {
            "Optional skey sent through Authorization: Bearer."
        }
        Text::OptionalSkey => "Optional skey",
        Text::ClientGateway => "Client Gateway",
        Text::ConfigureAddressesAndCredentialsFor => {
            "Configure addresses and credentials for Desktop to access one or more Gateways."
        }
        Text::GatewayServices => "Gateway services",
        Text::ManageMultipleClientGatewaysDesktop => {
            "Manage multiple client Gateways. Desktop requests use the selected Gateway."
        }
        Text::AuthenticationText => "Authentication",
        Text::OptionalSkeySentAsAuthorization => {
            "Optional skey sent as Authorization: Bearer <skey>."
        }
        Text::ConfirmClientGatewayDeletion => "Confirm client Gateway deletion",
        Text::ConfirmDeletion => "Confirm deletion",
        Text::ClientGatewaySettingsHelp => "Client Gateway settings help",
        Text::WebSearch => "Web search",
        Text::HttpRequests => "HTTP requests",
        Text::Browser => "Browser",
        Text::Proxy => "Proxy",
        Text::Tunnel => "Tunnel",
        Text::NetworkSettings => "Network settings",
        Text::SwitchTabsToConfigureWeb => {
            "Switch tabs to configure web search, HTTP requests, browser, proxy and tunnels."
        }
        Text::EditModel => "Edit model",
        Text::AddModel => "Add model",
        Text::ModelId => "Model ID",
        Text::ActualModelIdSentTo => "Actual model_id sent to the provider.",
        Text::ExampleGpt4oMini => "Example: gpt-4o-mini",
        Text::DisplayName => "Display name",
        Text::DisplayOnlyLeaveEmptyTo => "Display only. Leave empty to use model_id.",
        Text::Optional => "Optional",
        Text::ModelFamily => "Model family",
        Text::OptionalFamilyMetadataDistinguishingModel => {
            "Optional family metadata distinguishing model series from the same vendor."
        }
        Text::ExampleQwenGptClaude => "Example: qwen, gpt, claude",
        Text::ContextLimitTokens => "Context limit (tokens)",
        Text::TotalContextWindowLeaveEmpty => {
            "Total context window. Leave empty to use the built-in baseline or safe default."
        }
        Text::Example200000LeaveEmptyFor => "Example: 200000; leave empty for default",
        Text::MaximumInputTokens => "Maximum input (tokens)",
        Text::MaximumInputWindowLeaveEmpty => {
            "Maximum input window. Leave empty to use the built-in baseline or safe default."
        }
        Text::MaximumOutputTokens => "Maximum output (tokens)",
        Text::MaximumOutputWindowZeroCan => {
            "Maximum output window. Zero can indicate no text output."
        }
        Text::Example4096LeaveEmptyFor => "Example: 4096; leave empty for default",
        Text::Reasoning => "Reasoning",
        Text::ToolCalls => "Tool calls",
        Text::Temperature => "Temperature",
        Text::Attachments => "Attachments",
        Text::InterleavedInputOutput => "Interleaved input/output",
        Text::Capabilities => "Capabilities",
        Text::UnsetValuesUseTheBaseline => {
            "Unset values use the baseline. New models default to text and tool calling."
        }
        Text::InputModalities => "Input modalities",
        Text::ChooseProviderInputTypesLeave => {
            "Choose provider input types. Leave empty for text model defaults."
        }
        Text::OutputModalities => "Output modalities",
        Text::ChooseProviderOutputTypesLeave => {
            "Choose provider output types. Leave empty for text model defaults."
        }
        Text::ModelInformation => "Model information",
        Text::ModelIdIsSentIn => {
            "Model ID is sent in requests; display name and family are metadata only."
        }
        Text::Limits => "Limits",
        Text::EmptyValuesUseTheBuilt => {
            "Empty values use the built-in baseline or safe defaults. Values are in tokens."
        }
        Text::CapabilitiesAndModalities => "Capabilities and modalities",
        Text::SaveOnlyExplicitOverridesPreserving => {
            "Save only explicit overrides, preserving other discovered model settings."
        }
        Text::EnterTheModelIdAnd => "Enter the model ID and display name.",
        Text::ModelProviders => "Model providers",
        Text::ManageConnectedPopularAndCustom => {
            "Manage connected, popular and custom model providers."
        }
        Text::Syncing => "Syncing…",
        Text::SyncFromModelsDev => "Sync from models.dev",
        Text::Refreshing => "Refreshing…",
        Text::Refresh => "Refresh",
        Text::ConnectedProviders => "Connected providers",
        Text::ViewAvailableProvidersAndManage => {
            "View available providers and manage credentials, edit or disconnect them."
        }
        Text::NoConnectedProviders => "No connected providers.",
        Text::ConfirmDisconnect => "Confirm disconnect",
        Text::AuthorizeAgain => "Authorize again",
        Text::ChangeApiKey => "Change API key",
        Text::Edit => "Edit",
        Text::Disconnect => "Disconnect",
        Text::PopularProviders => "Popular providers",
        Text::MatchPopularPatternsAgainstThe => {
            "Match popular patterns against the catalog to connect commonly used providers."
        }
        Text::OfficialRecommendation => "Official recommendation",
        Text::Connected => "Connected",
        Text::Connect => "Connect",
        Text::OfficialShijuModelServiceAvailable => {
            "Official ShiJu model service. Available models are discovered after connecting."
        }
        Text::NotFound => "Not found",
        Text::Remove => "Remove",
        Text::DiscoverMoreProviders => "Discover more providers",
        Text::BrowseTheModelCatalogAdd => {
            "Browse the model catalog, add providers to favorites or connect directly."
        }
        Text::MoreModelProviders => "More model providers",
        Text::CustomProvider => "Custom provider",
        Text::ConnectAnOpenaiCompatibleProvider => {
            "Connect an OpenAI-compatible provider by base URL."
        }
        Text::OpenaiCompatibleProvider => "OpenAI-compatible provider",
        Text::AddACustomProviderAnd => {
            "Add a custom provider and edit its connection settings later."
        }
        Text::AddProvider => "Add provider",
        Text::EnterAndSaveTheApi => "Enter and save the API key for this provider.",
        Text::ApiKey => "API key",
        Text::EnterApiKey => "Enter API key",
        Text::SaveApiKey => "Save API key",
        Text::AlreadyInFavorites => "Already in favorites",
        Text::AddToFavorites => "Add to favorites",
        Text::Loading => "Loading…",
        Text::NoModelProvidersAvailable => "No model providers available",
        Text::NoMatchingModelProviders => "No matching model providers",
        Text::SearchAndFilter => "Search and filter",
        Text::FindCatalogProvidersByName => "Find catalog providers by name or provider ID.",
        Text::SearchModelProviders => "Search model providers",
        Text::BrowseTheModelCatalogAnd => {
            "Browse the model catalog and connect providers or add them to favorites."
        }
        Text::EditProvider => "Edit provider",
        Text::EditTheProviderConnectionAddress => {
            "Edit the provider connection address, default credentials and model list."
        }
        Text::ConnectAnOpenaiCompatibleProviderText => {
            "Connect an OpenAI-compatible provider by base URL. Models can be configured later."
        }
        Text::ProviderId => "Provider ID",
        Text::StableConfigurationIdReadOnly => "Stable configuration ID, read-only while editing.",
        Text::StableConfigurationIdUsingLowercase => {
            "Stable configuration ID using lowercase letters, digits, hyphens or underscores."
        }
        Text::ProviderNameShownToUsers => "Provider name shown to users.",
        Text::MyAiProvider => "My AI provider",
        Text::BaseUrl => "Base URL",
        Text::OpenaiCompatibleApiRootFor => {
            "OpenAI-compatible API root, for example https://api.example.com/v1."
        }
        Text::AutomaticAccountAuthorization => "Automatic account authorization",
        Text::SignInToYourShiju => {
            "Sign in to your ShiJu account to configure models automatically without a model API key."
        }
        Text::AccountManagedKeyPlaintextIs => "Account-managed key; plaintext is hidden",
        Text::OptionalDefaultKeySentIn => "Optional default key sent in the Authorization header.",
        Text::AddHeader => "Add header",
        Text::HeaderName => "Header name",
        Text::HeaderValue => "Header value",
        Text::Headers => "Headers",
        Text::AddHeadersForCustomAuthentication => {
            "Add headers for custom authentication, proxy Gateways or token headers."
        }
        Text::Discovering => "Discovering…",
        Text::DiscoverModels => "Discover models",
        Text::All => "All",
        Text::EnabledText => "Enabled",
        Text::DisabledText => "Disabled",
        Text::LeaveEmptyDiscoverModelsAutomatically => {
            "Leave empty, discover models automatically, or add models manually and enable or disable them."
        }
        Text::Filter => "Filter",
        Text::FilterModelsByAvailability => "Filter models by availability.",
        Text::UnnamedModel => "Unnamed model",
        Text::NoModelIdYet => "No model_id yet",
        Text::NoModels => "No models",
        Text::NoEnabledModels => "No enabled models",
        Text::NoDisabledModels => "No disabled models",
        Text::BasicInformation => "Basic information",
        Text::SetTheProviderIdDisplay => {
            "Set the provider ID, display name and OpenAI-compatible endpoint."
        }
        Text::AuthenticationAndHeaders => "Authentication and headers",
        Text::SetADefaultApiKey => {
            "Set a default API key and additional headers required by your Gateway."
        }
        Text::ManageTheProviderModelsUsed => {
            "Manage the provider models used by chat and generation views."
        }
    }
}
