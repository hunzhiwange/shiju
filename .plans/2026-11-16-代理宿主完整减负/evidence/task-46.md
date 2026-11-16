# 任务46：模型中继协议与传输

代码静态迁移完成；编译/行为未验证。未运行任何 Cargo、单元测试、构建、实机或格式化命令。静态脚本核对全部原测试名称多重集，迁移前后完全一致，无断言场景删除。

## 归属与调用方向

既有生产逻辑已经在 model-protocol、model-transport、model-relay-http；本轮生产算法迁移收益为 0，不能计作新增下沉。ProviderRelayBackend::execute → model_transport::provider_transport::ProviderTransport::execute；RelayServer::start → model_relay_http::RelayServer::start；RelayContext 保持真实目录、租户、目录模型鉴权及 usage 数据库注入。领域不依赖 vw-agent，无宿主源码 path/include 挂载。

新增 model-relay-http::host_migration_46_tests 自注册，共 22 条测试，直接使用真实 model-protocol 和 model-transport 入口。协议组合及真实回环网络夹具归 HTTP 边界测试；未添加生产抽象或测试专用公共接口。diagnostic_stream_fixtures 从既有领域夹具复制最小 samples/source，避免跨库私有源码挂载。宿主 stream_observer 测试挂载和四个完整测试模块原注册已删除；混合文件按整个函数移动。

## 行数（splitlines）

本任务78个来源：4080 → 3363 行，净减 717 行（17.6%），未达70%。生产/装配/转发文件含 mod.rs 的物理行与测试文件分项如下（仅导航分类，实际生产职责见逐文件表）：

- 宿主测试：3634 → 2935 行。
- 宿主生产/装配/转发：446 → 428 行。
- 领域新增测试及夹具：747 行；注册模块 11 行；生产算法新增0行。

## 全文件映射与保留原因

保留的大型 HTTP 用例直接操作真实项目配置、租户隔离和热更新，不能用模拟 catalog 端口冒充宿主验收。继续可迁项：在保持每条真实宿主鉴权断言的前提下，将 chat、Gemini、Messages、Responses 的混合大函数拆为领域传输场景与薄宿主契约；当前按“整个测试函数为最小移动单位”保留完整函数，未声称减量门槛达标，交收口审查这些缺口。以下逐文件同时列出所有原用例，名称不变；未列测试的文件是夹具、转发或实际端口。

|来源（model_relay/）|前→后行数|目标或具体保留职责|
|---|---:|---|
|alpha_search.rs|2→2|现有直接调用者兼容重导出 model_protocol::alpha_search；无业务算法|
|binding.rs|1→1|现有直接调用者兼容重导出 model_transport::binding；无业务算法|
|chat.rs|2→2|现有直接调用者兼容重导出 model_protocol::chat；无业务算法|
|chat_tests.rs|259→259|保留真实 Chat 配置热更新、账户凭证和工具权限边界|
|↳ `acp_model_source_relay_chat_roundtrip_stream_tools_and_account_isolation`|测试名称不变|`crates/vw-agent/src/model_relay/chat_tests.rs`|
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
|gemini_tests.rs|616→526|保留真实 Gemini 项目配置热更新、真实CLI调用及目录模型权限边界；独立断言迁至 crates/libs/model-relay-http/src/host_migration_46_tests/gemini_tests.rs|
|↳ `model_protocol_018_gemini_http_tools_stream_metadata_errors_and_cancel`|测试名称不变|`crates/vw-agent/src/model_relay/gemini_tests.rs`|
|↳ `acp_model_source_relay_gemini_real_cli`|测试名称不变|`crates/vw-agent/src/model_relay/gemini_tests.rs`|
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
|messages_request_http_tests.rs|156→156|保留真实 项目配置/租户上下文 HTTP 回放夹具及调用入口|
|messages_request_tests.rs|21→9|保留真实 项目配置/租户上下文 HTTP 回放夹具及调用入口；独立断言迁至 crates/libs/model-relay-http/src/host_migration_46_tests/routing_tests.rs|
|↳ `model_protocol_014_shared_selector_advertises_only_executable_messages_capabilities`|测试名称不变|`crates/libs/model-relay-http/src/host_migration_46_tests/routing_tests.rs`|
|↳ `model_protocol_014_http_request_reaches_bound_chat_and_finishes_tool_round`|测试名称不变|`crates/vw-agent/src/model_relay/messages_request_tests.rs`|
|messages_responses_request.rs|2→2|现有直接调用者兼容重导出 model_protocol::messages_responses_request；无业务算法|
|messages_responses_result.rs|2→2|现有直接调用者兼容重导出 model_protocol::messages_responses_result；无业务算法|
|messages_responses_stream.rs|2→2|现有直接调用者兼容重导出 model_protocol::messages_responses_stream；无业务算法|
|messages_stream.rs|6→6|现有直接调用者兼容重导出 model_protocol::messages_stream；无业务算法|
|messages_stream_tests.rs|94→6|保留真实 项目配置/租户上下文 HTTP 回放夹具及调用入口；独立断言迁至 crates/libs/model-relay-http/src/host_migration_46_tests/thinking_stream_tests.rs|
|↳ `model_protocol_016_http_sse_thinking_receipt_replays`|测试名称不变|`crates/vw-agent/src/model_relay/messages_stream_tests.rs`|
|↳ `model_protocol_016_thinking_http_errors_never_issue_success`|测试名称不变|`crates/libs/model-relay-http/src/host_migration_46_tests/thinking_stream_tests.rs`|
|messages_tests.rs|327→327|保留真实 Messages 项目配置热更新、账户与工具权限边界|
|↳ `acp_model_source_relay_messages_http_stream_cancellation_errors_and_isolation`|测试名称不变|`crates/vw-agent/src/model_relay/messages_tests.rs`|
|messages_thinking.rs|1→1|现有直接调用者兼容重导出 model_protocol::messages_thinking；无业务算法|
|messages_thinking_config.rs|1→1|现有直接调用者兼容重导出 model_protocol::messages_thinking_config；无业务算法|
|messages_thinking_http_tests.rs|253→253|保留真实 项目配置/租户上下文 HTTP 回放夹具及调用入口|
|messages_thinking_tests.rs|16→5|保留真实 项目配置/租户上下文 HTTP 回放夹具及调用入口；独立断言迁至 crates/libs/model-relay-http/src/host_migration_46_tests/routing_tests.rs|
|↳ `model_protocol_015_http_tool_round_and_sse_preserve_reasoning`|测试名称不变|`crates/vw-agent/src/model_relay/messages_thinking_tests.rs`|
|↳ `model_protocol_015_selector_exposes_thinking_and_replay`|测试名称不变|`crates/libs/model-relay-http/src/host_migration_46_tests/routing_tests.rs`|
|mod.rs|173→162|模块注册及公开宿主入口，非生产算法|
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
|responses_http_tests.rs|904→904|保留真实 Responses 项目配置、账户隔离、compaction配置及所有权边界|
|↳ `aggregation_t25_authorized_codex_chat_replays_compaction_over_http`|测试名称不变|`crates/vw-agent/src/model_relay/responses_http_tests.rs`|
|↳ `model_protocol_017_responses_http_roundtrip_stream_errors_cancel_and_ownership`|测试名称不变|`crates/vw-agent/src/model_relay/responses_http_tests.rs`|
|↳ `model_protocol_017_responses_chat_preserves_reasoning_tools_json_and_sse`|测试名称不变|`crates/vw-agent/src/model_relay/responses_http_tests.rs`|
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
  预期 22 条非零测试；未运行，无通过证据。
- `python3 scripts/cargo_slot.py cargo test -p vw-agent --lib model_relay:: -- --list`：确认上表迁走的完整名称不再挂载宿主；不能以零测试当成功。
- 保留的宿主用例按上表名称逐个精确过滤（model_relay 对应模块路径）；真实 Gemini CLI 用例仍 ignored，需未来明确启用并取得供应商证据。
- Cargo.lock 已有其它任务的工作区变更，本任务不覆盖；未来首次授权 Cargo 验证需更新 model-relay-http dev 依赖解析。
