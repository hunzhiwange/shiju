# 直接消费者引用快照

静态审计快照；执行时重新搜索。清单直接消费者为 vw-agent、vw-agent-apps、vw-desktop。agent 自身 provider/provider.rs 有独立组装实现，本计划不搬其业务，仅修改其发现与契约依赖。以下每个引用由对应领域卡负责，最终卡查漏。

```text
crates/vw-provider-resolver/src/provider_tests.rs:102:                .join(format!("vw-provider-resolver-tests-{}-{nanos}", std::process::id()))
crates/vw-provider-resolver/Cargo.toml:2:name = "vw-provider-resolver"
crates/vw-agent-apps/Cargo.toml:41:vw-provider-resolver = { path = "../vw-provider-resolver", version = "0.6.0" }
crates/vw-agent-apps/src/models/fill_service.rs:17:use vw_provider_resolver::models::ModelsSnapshot;
crates/vw-agent-apps/src/models/vendor_fallback_tests.rs:15:use vw_provider_resolver::models::ModelsSnapshot;
crates/vw-agent-apps/src/models/fill_service_tests.rs:21:use vw_provider_resolver::models::ModelsSnapshot;
crates/vw-agent-apps/src/models/vendor_fallback.rs:5:use vw_provider_resolver::models::{Model, ModelsSnapshot};
crates/vw-agent-apps/src/models/metadata_tests.rs:19:use vw_provider_resolver::models::ModelsSnapshot;
crates/vw-agent-apps/src/models/metadata.rs:23:use vw_provider_resolver::models::{Model, ModelsSnapshot, ReasoningOption};
crates/vw-agent-apps/src/models/fill_openclaw_tests.rs:169:    use vw_provider_resolver::models::ModelsSnapshot;
crates/vw-agent-apps/src/models/discovery_http.rs:10:pub use vw_provider_resolver::remote_models::DiscoveredModel;
crates/vw-agent-apps/src/models/discovery_http.rs:11:use vw_provider_resolver::remote_models::{discover_catalog_models, is_real_bearer};
crates/vw-desktop/Cargo.toml:71:vw-provider-resolver = { path = "../vw-provider-resolver", version = "0.6.0" }
crates/vw-agent-apps/src/usage/coding/kimi.rs:20:use vw_provider_resolver::remote_models::is_real_bearer;
crates/vw-agent-apps/src/usage/models_selection.rs:5:use vw_provider_resolver::models::{Model, Provider};
crates/vw-agent-apps/src/usage/models_sync.rs:6:use vw_provider_resolver::models::{self, ModelsRefreshStatus};
crates/vw-agent-apps/src/usage/coding/zenmux.rs:17:use vw_provider_resolver::remote_models::is_real_bearer;
crates/vw-agent-apps/src/usage/coding/minimax.rs:17:use vw_provider_resolver::remote_models::is_real_bearer;
crates/vw-agent-apps/src/usage/coding/command_code.rs:17:use vw_provider_resolver::remote_models::is_real_bearer;
crates/vw-agent-apps/src/usage/balances/novita.rs:12:use vw_provider_resolver::remote_models::is_real_bearer;
crates/vw-agent-apps/src/usage/coding/zhipu.rs:20:use vw_provider_resolver::remote_models::is_real_bearer;
crates/vw-agent-apps/src/usage/balances/deepseek.rs:12:use vw_provider_resolver::remote_models::is_real_bearer;
crates/vw-agent-apps/src/usage/coding/opencode_go.rs:16:use vw_provider_resolver::remote_models::is_real_bearer;
crates/vw-agent-apps/src/usage/balances/stepfun.rs:12:use vw_provider_resolver::remote_models::is_real_bearer;
crates/vw-agent-apps/src/usage/balances/siliconflow.rs:12:use vw_provider_resolver::remote_models::is_real_bearer;
crates/vw-agent-apps/src/usage/balances/openrouter.rs:12:use vw_provider_resolver::remote_models::is_real_bearer;
crates/vw-agent/Cargo.toml:97:vw-provider-resolver = { path = "../vw-provider-resolver", version = "0.6.0" }
crates/vw-desktop/src/app/provider.rs:9:    pub use vw_provider_resolver::provider::{
crates/vw-desktop/src/app/provider.rs:25:    pub use vw_provider_resolver::models::{get, invalidate_cache, refresh};
crates/vw-agent/src/app_providers/model_edits.rs:13:use vw_provider_resolver::models::ModelsSnapshot;
crates/vw-agent/src/provider/voice/mod.rs:91:        .filter(|key| vw_provider_resolver::remote_models::is_real_bearer(key))
crates/vw-agent/src/provider/voice/mod.rs:98:                .find(|key| vw_provider_resolver::remote_models::is_real_bearer(key))
crates/vw-agent/src/provider/voice/mod.rs:104:        .filter(|key| vw_provider_resolver::remote_models::is_real_bearer(key))
crates/vw-agent/src/provider/mod.rs:23:    pub use vw_provider_resolver::error::*;
crates/vw-agent/src/provider/mod.rs:27:    pub use vw_provider_resolver::models::*;
crates/vw-agent/src/provider/models.rs:1://! 模型元数据加载统一委托给 `vw-provider-resolver`，确保 agent 与桌面端使用同一份模型目录。
crates/vw-agent/src/provider/models.rs:5:pub use vw_provider_resolver::models::{
crates/vw-agent/src/provider/models.rs:11:    vw_provider_resolver::models::get().await
crates/vw-agent/src/provider/models.rs:15:    vw_provider_resolver::models::refresh().await;
crates/vw-agent/src/provider/models.rs:19:    vw_provider_resolver::models::init();
crates/vw-agent/src/provider/provider.rs:41:use vw_provider_resolver::remote_models::{discover_and_merge_provider, is_real_bearer};
crates/vw-desktop/src/app/message/settings/providers/models.rs:86:    use vw_provider_resolver::remote_models::{
crates/vw-desktop/src/app/message/settings/providers/updates.rs:692:                    async { vw_provider_resolver::provider::invalidate_cache().await },
crates/vw-desktop/src/app/components/input_panel/reasoning_selector_tests.rs:219:        "/../vw-provider-resolver/assets/model/shiju.json"
crates/vw-agent/src/session/llm/acp/model_source/drivers/mod.rs:197:) -> anyhow::Result<vw_provider_resolver::protocol_routes::ProtocolRoute> {
crates/vw-agent/src/session/llm/acp/model_source/drivers/mod.rs:198:    use vw_provider_resolver::protocol_routes::RouteCapability as C;
crates/vw-desktop/src/app/impls/app_update.rs:570:        vw_provider_resolver::config::set_wasm_gateway_endpoint({
crates/vw-agent/src/model_relay/routing.rs:5:use vw_provider_resolver::protocol_routes::{
crates/vw-agent/src/model_relay/routing.rs:140:) -> Result<ProtocolRoute, vw_provider_resolver::protocol_routes::RouteError> {
crates/vw-agent/src/model_relay/routing.rs:141:    use vw_provider_resolver::protocol_routes::{RouteError, RouteRejection};
crates/vw-agent/src/model_relay/messages_request_tests.rs:8:    use vw_provider_resolver::protocol_routes::RouteCapability as C;
crates/vw-agent/src/model_relay/transport.rs:12:use vw_provider_resolver::protocol_routes::RouteCapability as C;
crates/vw-agent/src/gateway/agent_apps_diagnostic_executors/model_check/target.rs:175:        use vw_provider_resolver::protocol_routes::RouteCapability;
crates/vw-agent/src/model_relay/generation.rs:6:use vw_provider_resolver::remote_models::is_real_bearer;
crates/vw-agent/src/model_relay/messages_thinking_tests.rs:10:    use vw_provider_resolver::protocol_routes::RouteCapability as C;
crates/vw-agent/src/model_relay/gemini_tests.rs:578:    use vw_provider_resolver::protocol_routes::RouteCapability as C;
crates/vw-agent/src/session/llm/aisdk/reasoning_tests.rs:11:            "/../vw-provider-resolver/assets/model/shiju.json"
crates/vw-agent/src/gateway/api/handlers/model_catalog_edits.rs:29:use vw_provider_resolver::models::{self, ModelsSnapshot};
crates/vw-agent/src/session/llm/aisdk/convert.rs:331:    required: &[vw_provider_resolver::protocol_routes::RouteCapability],
crates/vw-agent/src/session/llm/aisdk/convert.rs:334:    use vw_provider_resolver::protocol_routes::{ConversionSupport, RouteCapability as C};
crates/vw-agent/src/gateway/api/handlers/model_catalog_acceptance_tests.rs:214:        let cache = vw_provider_resolver::global::paths().cache.join("models.json");
crates/vw-agent/src/gateway/api/handlers/usage/prices.rs:119:    output(u::models_selection::flatten_models(&vw_provider_resolver::models::snapshot().await.providers))
```
