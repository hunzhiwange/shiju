# 任务46：模型中继协议与传输

代码静态迁移完成；编译/行为未验证。未运行任何 Cargo、单元测试、构建、实机或格式化命令。静态脚本核对全部原测试名称多重集，迁移前后完全一致，无断言场景删除。

## 归属与调用方向

既有生产逻辑已经在 model-protocol、model-transport、model-relay-http；本轮生产算法迁移收益为 0，不能计作新增下沉。ProviderRelayBackend::execute → model_transport::provider_transport::ProviderTransport::execute；RelayServer::start → model_relay_http::RelayServer::start；RelayContext 保持真实目录、租户、目录模型鉴权及 usage 数据库注入。领域不依赖 vw-agent，无宿主源码 path/include 挂载。

新增 model-relay-http::host_migration_46_tests 自注册，共 32 条测试，直接使用真实 model-protocol 和 model-transport 入口。协议组合及真实回环网络夹具归 HTTP 边界测试；未添加生产抽象或测试专用公共接口。diagnostic_stream_fixtures 从既有领域夹具复制最小 samples/source，避免跨库私有源码挂载。宿主 stream_observer 测试挂载和四个完整测试模块原注册已删除；混合文件按整个函数移动。

## 行数（splitlines）

本任务78个来源：4080 → 898 行，净减 3182 行（78.0%），达到70%门槛。生产/装配/转发文件含 mod.rs 的物理行与测试文件分项如下（仅导航分类，实际生产职责见逐文件表）：

- 宿主测试：3634 → 490 行。
- 宿主生产/装配/转发：446 → 408 行。
- 领域新增测试及夹具：3328 行；注册模块 24 行；生产算法新增0行。

## 全文件映射与保留原因

第二轮恢复后完整迁出 Chat、Gemini、Messages、Responses HTTP 场景及 thinking/request 回放夹具。使用 agent-project、agent-config、agent-provider 的真实存储和配置更新，不以固定响应替代 catalog。测试局部 task-local 租户是 RelayHost 端口输入；fixture.rs 仅组合已存在的领域入口与真实 usage ledger，不复制 App/AppState，不新增生产端口。agent-project/config/database 启用既有 test-support，沿用临时数据库隔离。供应商目录安装沿用 agent-provider 任务56 的真实 AppProviderStore 与 app_models::edits::read 模式。

宿主仅保留 server_tests.rs 的4条原用例，直接验证生产 RelayContext::{execute,control} 的当前租户/项目、真实目录模型禁用、监听器撤销/过期及底层跳转拒绝。领域 fixture 的上下文组合不能替代这些实际宿主接线证据，故保留；其余文件是表中注明的生产端口或直接调用者兼容导出。

|来源（model_relay/）|前→后行数|目标或具体保留职责|
|---|---:|---|
|alpha_search.rs|2→2|现有直接调用者兼容重导出 model_protocol::alpha_search；无业务算法|
|binding.rs|1→1|现有直接调用者兼容重导出 model_transport::binding；无业务算法|
|chat.rs|2→2|现有直接调用者兼容重导出 model_protocol::chat；无业务算法|
|chat_tests.rs|259→0|完整迁至 crates/libs/model-relay-http/src/host_migration_46_tests/chat_tests.rs|
|↳ `acp_model_source_relay_chat_roundtrip_stream_tools_and_account_isolation`|测试名称不变|`crates/libs/model-relay-http/src/host_migration_46_tests/chat_tests.rs`|
|codex_oauth_transport.rs|1→1|现有直接调用者兼容重导出 model_transport::codex_oauth_transport；无业务算法|
|compaction.rs|2→2|现有直接调用者兼容重导出 model_protocol::compaction；无业务算法|
|compaction_v2.rs|2→2|现有直接调用者兼容重导出 model_protocol::compaction_v2；无业务算法|
|compaction_v2_tests.rs|9→0|完整迁至 crates/libs/model-relay-http/src/host_migration_46_tests/compaction_v2_tests.rs|
|↳ `cc_route_078_anthropic_summary_request_is_supported`|测试名称不变|`crates/libs/model-relay-http/src/host_migration_46_tests/compaction_v2_tests.rs`|
|copilot_transport.rs|1→1|现有直接调用者兼容重导出 model_transport::copilot_transport；无业务算法|
|forced_sse.rs|2→2|现有直接调用者兼容重导出 model_protocol::forced_sse；无业务算法|
|gemini.rs|2→2|现有直接调用者兼容重导出 model_protocol::gemini；无业务算法|
|gemini_metadata.rs|2→2|现有直接调用者兼容重导出 model_protocol::gemini_metadata；无业务算法|
|gemini_oauth_transport.rs|1→1|现有直接调用者兼容重导出 model_transport::gemini_oauth_transport；无业务算法|
|gemini_shadow.rs|2→2|现有直接调用者兼容重导出 model_protocol::gemini_shadow；无业务算法|
|gemini_stream.rs|2→2|现有直接调用者兼容重导出 model_protocol::gemini_stream；无业务算法|
|gemini_tests.rs|616→0|完整迁至 crates/libs/model-relay-http/src/host_migration_46_tests/gemini_http_tests.rs, crates/libs/model-relay-http/src/host_migration_46_tests/gemini_tests.rs|
|↳ `model_protocol_018_gemini_http_tools_stream_metadata_errors_and_cancel`|测试名称不变|`crates/libs/model-relay-http/src/host_migration_46_tests/gemini_http_tests.rs`|
|↳ `acp_model_source_relay_gemini_real_cli`|测试名称不变|`crates/libs/model-relay-http/src/host_migration_46_tests/gemini_http_tests.rs`|
|↳ `acp_model_source_relay_gemini_rejects_lossy_state_and_ambiguous_tools`|测试名称不变|`crates/libs/model-relay-http/src/host_migration_46_tests/gemini_tests.rs`|
|↳ `acp_model_source_relay_gemini_matches_idless_tools_and_preserves_reasoning_usage`|测试名称不变|`crates/libs/model-relay-http/src/host_migration_46_tests/gemini_tests.rs`|
|↳ `model_protocol_018_gemini_native_requirements_are_selected_explicitly`|测试名称不变|`crates/libs/model-relay-http/src/host_migration_46_tests/gemini_tests.rs`|
|↳ `model_protocol_018_gemini_tool_turn_preserves_text_after_call`|测试名称不变|`crates/libs/model-relay-http/src/host_migration_46_tests/gemini_tests.rs`|
|generation.rs|2→2|现有直接调用者兼容重导出 model_protocol::generation；无业务算法|
|hosted_search.rs|2→2|现有直接调用者兼容重导出 model_protocol::hosted_search；无业务算法|
|hosted_search_tests.rs|50→0|完整迁至 crates/libs/model-relay-http/src/host_migration_46_tests/hosted_search_tests.rs|
|↳ `cc_route_044_hosted_search_expected_behavior`|测试名称不变|`crates/libs/model-relay-http/src/host_migration_46_tests/hosted_search_tests.rs`|
|↳ `cc_route_044_failure_and_limit_are_returned_to_client`|测试名称不变|`crates/libs/model-relay-http/src/host_migration_46_tests/hosted_search_tests.rs`|
|↳ `cc_route_044_rejects_invalid_limits_and_execution_modes`|测试名称不变|`crates/libs/model-relay-http/src/host_migration_46_tests/hosted_search_tests.rs`|
|image_edit.rs|2→2|现有直接调用者兼容重导出 model_protocol::image_edit；无业务算法|
|image_generation.rs|2→2|现有直接调用者兼容重导出 model_protocol::image_generation；无业务算法|
|inline_think.rs|2→2|现有直接调用者兼容重导出 model_protocol::inline_think；无业务算法|
|kiro.rs|2→2|现有直接调用者兼容重导出 model_protocol::kiro；无业务算法|
|late_arguments.rs|2→2|现有直接调用者兼容重导出 model_protocol::late_arguments；无业务算法|
|messages.rs|2→2|现有直接调用者兼容重导出 model_protocol::messages；无业务算法|
|messages_chat_request.rs|2→2|现有直接调用者兼容重导出 model_protocol::messages_chat_request；无业务算法|
|messages_chat_result.rs|2→2|现有直接调用者兼容重导出 model_protocol::messages_chat_result；无业务算法|
|messages_chat_stream_tests.rs|133→0|完整迁至 crates/libs/model-relay-http/src/host_migration_46_tests/messages_chat_stream_tests.rs|
|↳ `cc_route_040_messages_chat_stream_expected_behavior`|测试名称不变|`crates/libs/model-relay-http/src/host_migration_46_tests/messages_chat_stream_tests.rs`|
|↳ `cc_route_040_invalid_or_incomplete_stream_fails_once`|测试名称不变|`crates/libs/model-relay-http/src/host_migration_46_tests/messages_chat_stream_tests.rs`|
|↳ `cc_route_040_duplicate_terminal_emits_one_stop`|测试名称不变|`crates/libs/model-relay-http/src/host_migration_46_tests/messages_chat_stream_tests.rs`|
|↳ `cc_route_040_session_usage_remains_strict`|测试名称不变|`crates/libs/model-relay-http/src/host_migration_46_tests/messages_chat_stream_tests.rs`|
|messages_gemini_request.rs|2→2|现有直接调用者兼容重导出 model_protocol::messages_gemini_request；无业务算法|
|messages_gemini_result.rs|2→2|现有直接调用者兼容重导出 model_protocol::messages_gemini_result；无业务算法|
|messages_gemini_stream.rs|2→2|现有直接调用者兼容重导出 model_protocol::messages_gemini_stream；无业务算法|
|messages_request_http_tests.rs|156→0|完整迁至 |
|messages_request_tests.rs|21→0|完整迁至 crates/libs/model-relay-http/src/host_migration_46_tests/messages_request_tests.rs, crates/libs/model-relay-http/src/host_migration_46_tests/routing_tests.rs|
|↳ `model_protocol_014_shared_selector_advertises_only_executable_messages_capabilities`|测试名称不变|`crates/libs/model-relay-http/src/host_migration_46_tests/routing_tests.rs`|
|↳ `model_protocol_014_http_request_reaches_bound_chat_and_finishes_tool_round`|测试名称不变|`crates/libs/model-relay-http/src/host_migration_46_tests/messages_request_tests.rs`|
|messages_responses_request.rs|2→2|现有直接调用者兼容重导出 model_protocol::messages_responses_request；无业务算法|
|messages_responses_result.rs|2→2|现有直接调用者兼容重导出 model_protocol::messages_responses_result；无业务算法|
|messages_responses_stream.rs|2→2|现有直接调用者兼容重导出 model_protocol::messages_responses_stream；无业务算法|
|messages_stream.rs|6→2|现有直接调用者兼容重导出 model_protocol::messages_stream；无业务算法|
|messages_stream_tests.rs|94→0|完整迁至 crates/libs/model-relay-http/src/host_migration_46_tests/messages_stream_tests.rs, crates/libs/model-relay-http/src/host_migration_46_tests/thinking_stream_tests.rs|
|↳ `model_protocol_016_http_sse_thinking_receipt_replays`|测试名称不变|`crates/libs/model-relay-http/src/host_migration_46_tests/messages_stream_tests.rs`|
|↳ `model_protocol_016_thinking_http_errors_never_issue_success`|测试名称不变|`crates/libs/model-relay-http/src/host_migration_46_tests/thinking_stream_tests.rs`|
|messages_tests.rs|327→0|完整迁至 crates/libs/model-relay-http/src/host_migration_46_tests/messages_tests.rs|
|↳ `acp_model_source_relay_messages_http_stream_cancellation_errors_and_isolation`|测试名称不变|`crates/libs/model-relay-http/src/host_migration_46_tests/messages_tests.rs`|
|messages_thinking.rs|1→1|现有直接调用者兼容重导出 model_protocol::messages_thinking；无业务算法|
|messages_thinking_config.rs|1→1|现有直接调用者兼容重导出 model_protocol::messages_thinking_config；无业务算法|
|messages_thinking_http_tests.rs|253→0|完整迁至 |
|messages_thinking_tests.rs|16→0|完整迁至 crates/libs/model-relay-http/src/host_migration_46_tests/messages_thinking_tests.rs, crates/libs/model-relay-http/src/host_migration_46_tests/routing_tests.rs|
|↳ `model_protocol_015_http_tool_round_and_sse_preserve_reasoning`|测试名称不变|`crates/libs/model-relay-http/src/host_migration_46_tests/messages_thinking_tests.rs`|
|↳ `model_protocol_015_selector_exposes_thinking_and_replay`|测试名称不变|`crates/libs/model-relay-http/src/host_migration_46_tests/routing_tests.rs`|
|mod.rs|173→146|模块注册及公开宿主入口，非生产算法|
|models.rs|2→2|现有直接调用者兼容重导出 model_protocol::models；无业务算法|
|native_chat.rs|2→2|现有直接调用者兼容重导出 model_protocol::native_chat；无业务算法|
|native_gemini.rs|2→2|现有直接调用者兼容重导出 model_protocol::native_gemini；无业务算法|
|native_messages.rs|2→2|现有直接调用者兼容重导出 model_protocol::native_messages；无业务算法|
|native_responses.rs|2→2|现有直接调用者兼容重导出 model_protocol::native_responses；无业务算法|
|protocol_bridge.rs|2→2|现有直接调用者兼容重导出 model_protocol::protocol_bridge；无业务算法|
|protocol_bridge_stream.rs|1→1|现有直接调用者兼容重导出 model_protocol::protocol_bridge_stream；无业务算法|
|quota_gate.rs|1→1|现有直接调用者兼容重导出 model_transport::quota_gate；无业务算法|
|reasoning_bridge.rs|2→2|现有直接调用者兼容重导出 model_protocol::reasoning_bridge；无业务算法|
|responses.rs|2→2|现有直接调用者兼容重导出 model_protocol::responses；无业务算法|
|responses_anthropic_request.rs|2→2|现有直接调用者兼容重导出 model_protocol::responses_anthropic_request；无业务算法|
|responses_anthropic_result.rs|2→2|现有直接调用者兼容重导出 model_protocol::responses_anthropic_result；无业务算法|
|responses_anthropic_stream.rs|2→2|现有直接调用者兼容重导出 model_protocol::responses_anthropic_stream；无业务算法|
|responses_chat_request.rs|2→2|现有直接调用者兼容重导出 model_protocol::responses_chat_request；无业务算法|
|responses_chat_result.rs|2→2|现有直接调用者兼容重导出 model_protocol::responses_chat_result；无业务算法|
|responses_chat_stream.rs|2→2|现有直接调用者兼容重导出 model_protocol::responses_chat_stream；无业务算法|
|responses_compaction.rs|2→2|现有直接调用者兼容重导出 model_protocol::responses_compaction；无业务算法|
|responses_history.rs|2→2|现有直接调用者兼容重导出 model_protocol::responses_history；无业务算法|
|responses_http_tests.rs|904→0|完整迁至 crates/libs/model-relay-http/src/host_migration_46_tests/responses_http_tests.rs|
|↳ `aggregation_t25_authorized_codex_chat_replays_compaction_over_http`|测试名称不变|`crates/libs/model-relay-http/src/host_migration_46_tests/responses_http_tests.rs`|
|↳ `model_protocol_017_responses_http_roundtrip_stream_errors_cancel_and_ownership`|测试名称不变|`crates/libs/model-relay-http/src/host_migration_46_tests/responses_http_tests.rs`|
|↳ `model_protocol_017_responses_chat_preserves_reasoning_tools_json_and_sse`|测试名称不变|`crates/libs/model-relay-http/src/host_migration_46_tests/responses_http_tests.rs`|
|responses_stream.rs|2→2|现有直接调用者兼容重导出 model_protocol::responses_stream；无业务算法|
|routing.rs|27→27|configured_protocol：目录 Model 到协议投影；其余 routing_selection 重导出供现有调用者|
|search_citations.rs|2→2|现有直接调用者兼容重导出 model_protocol::search_citations；无业务算法|
|search_citations_markdown.rs|1→1|既有迁移说明，无代码或注册；不将删除注释计收益|
|server.rs|89→89|RelayServer::{start,address,shutdown} 与 From：HTTP 监听器生命周期组合；RelayContext::{usage,error,control,execute}：真实数据库/目录模型/租户端口；RelayUsageObserver::{push,finish}：usage 端口适配|
|server_tests.rs|490→490|保留真实 RelayContext 当前租户/项目、目录权限、撤销与监听器生命周期边界|
|↳ `kiro_protocol_flag_is_local_but_catalog_still_requires_selected_model`|测试名称不变|`crates/vw-agent/src/model_relay/server_tests.rs`|
|↳ `acp_model_source_relay_binding_http_isolation_limits_and_lifecycle`|测试名称不变|`crates/vw-agent/src/model_relay/server_tests.rs`|
|↳ `acp_model_source_relay_expiry_cancels_an_established_http_stream`|测试名称不变|`crates/vw-agent/src/model_relay/server_tests.rs`|
|↳ `acp_model_source_relay_binding_upstream_redirect_is_not_followed`|测试名称不变|`crates/vw-agent/src/model_relay/server_tests.rs`|
|signature_rectify.rs|2→2|现有直接调用者兼容重导出 model_protocol::signature_rectify；无业务算法|
|sse_framing.rs|2→2|现有直接调用者兼容重导出 model_protocol::sse_framing；无业务算法|
|sse_framing_tests.rs|105→0|完整迁至 crates/libs/model-relay-http/src/host_migration_46_tests/sse_framing_tests.rs|
|↳ `cc_route_033_sse_framing_expected_behavior`|测试名称不变|`crates/libs/model-relay-http/src/host_migration_46_tests/sse_framing_tests.rs`|
|↳ `cc_route_033_resident_tail_requires_terminal_and_preserves_json`|测试名称不变|`crates/libs/model-relay-http/src/host_migration_46_tests/sse_framing_tests.rs`|
|↳ `cc_route_033_loopback_http_tail_and_truncation`|测试名称不变|`crates/libs/model-relay-http/src/host_migration_46_tests/sse_framing_tests.rs`|
|stream_observer.rs|6→2|现有直接调用者兼容重导出 model_transport::stream_observer；无业务算法|
|stream_observer_tests.rs|201→0|完整迁至 crates/libs/model-relay-http/src/host_migration_46_tests/stream_observer_tests.rs|
|↳ `shared_stream_observer_validates_four_protocols_and_fragmentation`|测试名称不变|`crates/libs/model-relay-http/src/host_migration_46_tests/stream_observer_tests.rs`|
|↳ `shared_stream_observer_production_body_preserves_bytes_and_cancels`|测试名称不变|`crates/libs/model-relay-http/src/host_migration_46_tests/stream_observer_tests.rs`|
|↳ `shared_stream_observer_rejects_illegal_end_and_model_changes`|测试名称不变|`crates/libs/model-relay-http/src/host_migration_46_tests/stream_observer_tests.rs`|
|↳ `shared_stream_observer_native_converters_preserve_client_model_mapping`|测试名称不变|`crates/libs/model-relay-http/src/host_migration_46_tests/stream_observer_tests.rs`|
|tool_media.rs|2→2|现有直接调用者兼容重导出 model_protocol::tool_media；无业务算法|
|tool_media_request.rs|2→2|现有直接调用者兼容重导出 model_protocol::tool_media_request；无业务算法|
|tool_namespace.rs|2→2|现有直接调用者兼容重导出 model_protocol::tool_namespace；无业务算法|
|transport.rs|40→37|ProviderRelayBackend::{new,execute}：当前任务租户与项目身份校验；credential/URL/configure_messages_chat 为直接调用者兼容重导出|

## 未来验证

- `python3 scripts/cargo_slot.py cargo test -p model-relay-http --lib host_migration_46_tests -- --test-threads=1`
  预期 32 条非零测试；未运行，无通过证据。
- `python3 scripts/cargo_slot.py cargo test -p vw-agent --lib model_relay:: -- --list`：确认上表迁走的完整名称不再挂载宿主；不能以零测试当成功。
- 保留的宿主用例按上表名称逐个精确过滤（model_relay 对应模块路径）；真实 Gemini CLI 用例仍 ignored，需未来明确启用并取得供应商证据。
- Cargo.lock 仅更新 model-relay-http 条目的直接依赖，保留其它任务暂存/工作区内容；reqwest 使用既有 0.13.4 版本限定，未执行依赖解析命令。

## 恢复轮静态证据

对照 evidence/final.md 与 remaining-host.json 的任务46记录：本轮来源残留3363→898行，再净减2465行；累计4080→898，净减3182行（78.0%）。生产算法迁移收益仍为0，未把测试组合fixture计入生产减负。

逐测试名称多重集静态比对：原36条 = 领域32条 + 宿主4条。领域32条中31条默认启用，真实Gemini CLI的1条仍ignored。领域入口由 cfg(test) host_migration_46_tests 注册，新增9个子模块注册；旧宿主9个测试/夹具模块及 messages_stream 内嵌挂载已删。所有断言和完整测试函数保留，仅替换已有领域类型与函数路径、环境锁和局部 RelayHost 组合端口。gemini_http_tests 保留两个HTTP/CLI入口，原纯转换测试仍在 gemini_tests。

必要直接调用：测试 RelayServer::start→crate::RelayServer::start；Scoped RelayBackend→agent_project::instance::provide_context→agent_provider::provider::get_model→model_transport::ProviderTransport::execute；配置写入→agent_config::runtime::update_project_for_tenant；UsageStreamObserver→model_protocol::stream_observer。测试fixture无 vw-agent 依赖与宿主Rust源码加载。真实宿主生产调用方向保持本文开头所述。

所有编译/行为验证继续禁止，静态名称集合与差异检查不能证明运行通过。未来先按本任务32条过滤获取非零注册，默认运行预期31条非忽略用例；真实CLI单独授权执行。
