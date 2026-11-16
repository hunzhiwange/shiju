# 任务18静态迁移证据

代码完成，编译/行为未验证。本轮遵守禁止 Cargo、构建、测试与实机执行的约束。

## 归属与依赖

生产入口此前已经下沉，本轮生产算法迁移收益为 0。宿主 bootstrap/router/runtime、WebSocket Host 以及 webhook 聊天闭包继续调用 agent-gateway；领域库不反向依赖 vw-agent。测试使用现有 gateway-types::AppState<(), (), ()>，未复制宿主状态；这些用例不读取扩展字段。配对沿用真实临时目录与配置落盘。路由策略用真实 gateway-global 路由验证共享中间件，真实宿主注册测试仍留宿主。

领域自注册：lib.rs → host_migration_18_tests → policy、router、node_service、acceptance 八模块。沿用既有 dev 依赖，无新增依赖。删除八个原 path 挂载、节点控制 path 挂载；其余真实宿主测试不冒充领域测试。

## 全来源文件映射

|来源|前行数|后行数|职责/保留原因|

|---|---:|---:|---|

|mod_tests.rs|357|138|StaticProvider/chat_with_system/auth_state/bootstrap_rest_app/tenant_request 保留以验证真实 bootstrap 注册、公网绑定和租户鉴权；auth_app/bearer_request 与完整策略用例迁入 policy。|

|node_control.rs|16|4|只保留具体 AppState 的节点处理器绑定；完整服务测试迁入 node_service。|

|openai_compat/mod.rs|29|29|两条处理器绑定具体状态及 client key，现有协议测试仍含宿主 provider 与工具执行。|

|options.rs|2|2|兼容重导出：保留现有直接调用路径，无业务实现；不计生产迁移。|

|pairing.rs|20|20|兼容重导出：保留现有直接调用路径，无业务实现；不计生产迁移。|

|router.rs|175|175|management_auth_router 读取真实 flag/env；handler_router 组合实际宿主处理器；build_router/app_routing_router 是真实装配。|

|router_tests.rs|221|89|保留 handler_router 及 build_router 的真实路由注册与健康检查；CORS、租户与前缀策略迁入 router。|

|runtime.rs|54|54|runtime_router 初始化真实数据库及同步生命周期；serve/start/serve_until/start_until 绑定真实宿主路由，现有 runtime_tests 含真实 peer 验收。|

|settings_store.rs|2|2|兼容重导出：保留现有直接调用路径，无业务实现；不计生产迁移。|

|sse.rs|33|33|兼容重导出：保留现有直接调用路径，无业务实现；不计生产迁移。|

|state.rs|23|23|具体 QueryEngine、ChannelRuntimeContext、WhatsAppChannel、HookRuntime 类型绑定。|

|state_tests.rs|10|10|真实宿主类型 Send/Sync 编译期契约。|

|tests/mod.rs|448|432|原共享夹具服务 agent、nextcloud、qq、webhook 的真实宿主聊天与通道装配；八个纯领域模块及其夹具改由领域注册。|

|types.rs|2|2|兼容重导出：保留现有直接调用路径，无业务实现；不计生产迁移。|

|util.rs|2|2|兼容重导出：保留现有直接调用路径，无业务实现；不计生产迁移。|

|webhook.rs|76|76|handle_webhook 注入真实 run_gateway_chat_simple；保留真实闭包验收。|

|webhook_ingress/mod.rs|15|15|handle_webhook_inner 注入真实聊天入口，用例仍含真实查询流程。|

|ws/mod.rs|103|103|Host::initial_history、run_loop、run_websocket_tool_loop、handle_ws_chat 使用真实工作区提示/技能/钩子/工具循环；其测试含完整工具与钩子断言。|

宿主来源总计 1588 → 1209，净减 379（23.9%），未达 70%。上表逐文件说明保留原因。继续可迁：将 OpenAI/WS/webhook/runtime 跨目录测试拆成领域策略与真实宿主端口契约；四个通道/聊天 acceptance 模块还需沿现有 Host 端口分离，不删除断言。收口须复核该缺口。

宿主按文件分项：生产文件（含测试挂载）552 → 540；测试/夹具文件 1036 → 671。生产分支未增加或迁出。

## 每条用例映射

- `mod_tests.rs::workflow_chat_messages_path_matches_collection_and_application_routes` → `host_migration_18_tests/policy.rs`

- `mod_tests.rs::workflow_chat_messages_path_rejects_empty_or_partial_matches` → `host_migration_18_tests/policy.rs`

- `mod_tests.rs::timeout_for_path_uses_extended_window_for_long_running_sse` → `host_migration_18_tests/policy.rs`

- `mod_tests.rs::run_gateway_rejects_public_bind_without_tunnel_or_opt_in` → `宿主原路径保留：真实装配`

- `mod_tests.rs::bootstrap_router_registers_v1_agent_scheduler_project_routes` → `宿主原路径保留：真实装配`

- `mod_tests.rs::bootstrap_router_rejects_client_routes_without_tenant_headers` → `宿主原路径保留：真实装配`

- `mod_tests.rs::bootstrap_hooks_routes_remain_behind_gateway_authentication` → `宿主原路径保留：真实装配`

- `mod_tests.rs::bootstrap_router_keeps_v1_health_and_webhook_outside_tenant_middleware` → `宿主原路径保留：真实装配`

- `mod_tests.rs::request_timeout_middleware_returns_inner_response_before_deadline` → `host_migration_18_tests/policy.rs`

- `mod_tests.rs::request_timeout_middleware_returns_timeout_after_deadline` → `host_migration_18_tests/policy.rs`

- `mod_tests.rs::request_timeout_middleware_preserves_workflow_response_past_default_deadline` → `host_migration_18_tests/policy.rs`

- `mod_tests.rs::request_timeout_middleware_times_out_workflow_after_extended_deadline` → `host_migration_18_tests/policy.rs`

- `mod_tests.rs::public_skey_auth_path_matches_only_global_health_routes` → `host_migration_18_tests/policy.rs`

- `mod_tests.rs::skey_auth_middleware_allows_options_and_public_paths_without_token` → `host_migration_18_tests/policy.rs`

- `mod_tests.rs::skey_auth_middleware_rejects_public_path_lookalikes_without_token` → `host_migration_18_tests/policy.rs`

- `mod_tests.rs::skey_auth_middleware_accepts_valid_skey` → `host_migration_18_tests/policy.rs`

- `mod_tests.rs::skey_auth_middleware_rejects_missing_or_invalid_skey` → `host_migration_18_tests/policy.rs`

- `mod_tests.rs::acp_install_timeout_covers_installer_deadline_only_for_single_agent_routes` → `host_migration_18_tests/policy.rs`

- `router_tests.rs::handler_router_serves_unversioned_handler_routes` → `宿主原路径保留：真实装配`

- `router_tests.rs::build_router_serves_unversioned_handler_routes` → `宿主原路径保留：真实装配`

- `router_tests.rs::build_router_serves_v1_handler_routes` → `宿主原路径保留：真实装配`

- `router_tests.rs::build_router_applies_cors_to_unversioned_handler_routes` → `host_migration_18_tests/router.rs`

- `router_tests.rs::build_router_registers_v1_desktop_service_route` → `宿主原路径保留：真实装配`

- `router_tests.rs::build_router_registers_v1_shiju_app_routes` → `宿主原路径保留：真实装配`

- `router_tests.rs::build_router_allows_whitelisted_cors_origin` → `host_migration_18_tests/router.rs`

- `router_tests.rs::build_router_allows_default_localhost_cors_origin` → `host_migration_18_tests/router.rs`

- `router_tests.rs::build_router_answers_whitelisted_cors_preflight` → `host_migration_18_tests/router.rs`

- `router_tests.rs::build_router_omits_cors_header_for_unlisted_origin` → `host_migration_18_tests/router.rs`

- `router_tests.rs::build_router_adds_cors_header_to_tenant_rejection` → `host_migration_18_tests/router.rs`

- `router_tests.rs::build_router_returns_not_found_for_unknown_v1_route` → `host_migration_18_tests/router.rs`

- `router_tests.rs::build_router_does_not_mount_handlers_under_repeated_v1_prefix` → `host_migration_18_tests/router.rs`

- `router_tests.rs::handler_router_reports_method_not_allowed_for_registered_route` → `宿主原路径保留：真实装配`

- `router_tests.rs::build_router_rejects_client_routes_without_tenant_headers` → `host_migration_18_tests/router.rs`

- `router_tests.rs::build_router_rejects_unversioned_routes_without_tenant_headers` → `host_migration_18_tests/router.rs`

- `state_tests.rs::app_state_type_is_send_sync` → `宿主原路径保留：真实装配`

- `tests::nextcloud::nextcloud_talk_webhook_returns_not_found_when_not_configured` → 宿主原挂载保留（真实聊天/通道端口）

- `tests::nextcloud::nextcloud_talk_webhook_rejects_invalid_signature` → 宿主原挂载保留（真实聊天/通道端口）

- `tests::qq::qq_webhook_returns_not_found_when_not_configured` → 宿主原挂载保留（真实聊天/通道端口）

- `tests::qq::qq_webhook_validation_returns_signed_challenge` → 宿主原挂载保留（真实聊天/通道端口）

- `tests::webhook::webhook_idempotency_skips_duplicate_provider_calls` → 宿主原挂载保留（真实聊天/通道端口）

- `tests::webhook::webhook_rejects_public_traffic_without_auth_layers` → 宿主原挂载保留（真实聊天/通道端口）

- `tests::webhook::webhook_autosave_stores_distinct_keys_per_request` → 宿主原挂载保留（真实聊天/通道端口）

- `tests::webhook::webhook_secret_hash_is_deterministic_and_nonempty` → 宿主原挂载保留（真实聊天/通道端口）

- `tests::webhook::webhook_secret_hash_rejects_missing_header` → 宿主原挂载保留（真实聊天/通道端口）

- `tests::webhook::webhook_secret_hash_rejects_invalid_header` → 宿主原挂载保留（真实聊天/通道端口）

- `tests::webhook::webhook_secret_hash_accepts_valid_header` → 宿主原挂载保留（真实聊天/通道端口）

- `tests::agent::agent_endpoint_requires_bearer_token_when_pairing_enabled` → 宿主原挂载保留（真实聊天/通道端口）

- 原 `node_control_tests::node_control_request_deserializes_method_and_node` → `host_migration_18_tests/node_service.rs`

- 原 `node_control_tests::node_control_request_deserializes_capability_and_arguments` → `host_migration_18_tests/node_service.rs`

- 原 `node_control_tests::node_control_request_deserializes_defaults` → `host_migration_18_tests/node_service.rs`

- 原 `node_control_tests::node_id_allowed_accepts_empty_allowlist` → `host_migration_18_tests/node_service.rs`

- 原 `node_control_tests::node_id_allowed_accepts_matching_node_or_wildcard` → `host_migration_18_tests/node_service.rs`

- 原 `node_control_tests::node_id_allowed_rejects_non_matching_node` → `host_migration_18_tests/node_service.rs`

- 原 `node_control_tests::node_id_allowed_rejects_partial_wildcard` → `host_migration_18_tests/node_service.rs`

- 原 `node_control_tests::handle_node_control_rejects_missing_pairing_bearer` → `host_migration_18_tests/node_service.rs`

- 原 `node_control_tests::handle_node_control_accepts_valid_pairing_bearer` → `host_migration_18_tests/node_service.rs`

- 原 `node_control_tests::handle_node_control_rejects_invalid_json_body` → `host_migration_18_tests/node_service.rs`

- 原 `node_control_tests::handle_node_control_returns_not_found_when_disabled` → `host_migration_18_tests/node_service.rs`

- 原 `node_control_tests::handle_node_control_rejects_invalid_node_token` → `host_migration_18_tests/node_service.rs`

- 原 `node_control_tests::handle_node_control_accepts_trimmed_node_token` → `host_migration_18_tests/node_service.rs`

- 原 `node_control_tests::handle_node_control_ignores_blank_node_token_config` → `host_migration_18_tests/node_service.rs`

- 原 `node_control_tests::handle_node_control_rejects_non_utf8_node_token` → `host_migration_18_tests/node_service.rs`

- 原 `node_control_tests::handle_node_control_lists_configured_nodes` → `host_migration_18_tests/node_service.rs`

- 原 `node_control_tests::handle_node_control_describe_requires_node_id` → `host_migration_18_tests/node_service.rs`

- 原 `node_control_tests::handle_node_control_describe_rejects_disallowed_node` → `host_migration_18_tests/node_service.rs`

- 原 `node_control_tests::handle_node_control_describes_allowed_node` → `host_migration_18_tests/node_service.rs`

- 原 `node_control_tests::handle_node_control_describes_node_when_allowlist_is_empty` → `host_migration_18_tests/node_service.rs`

- 原 `node_control_tests::handle_node_control_invoke_requires_node_id` → `host_migration_18_tests/node_service.rs`

- 原 `node_control_tests::handle_node_control_invoke_rejects_disallowed_node` → `host_migration_18_tests/node_service.rs`

- 原 `node_control_tests::handle_node_control_invoke_returns_scaffold_error` → `host_migration_18_tests/node_service.rs`

- 原 `node_control_tests::handle_node_control_invoke_preserves_missing_optional_fields` → `host_migration_18_tests/node_service.rs`

- 原 `node_control_tests::handle_node_control_rejects_unsupported_method` → `host_migration_18_tests/node_service.rs`

- 原 `tests::node_control::node_control_returns_not_found_when_disabled` → `host_migration_18_tests/acceptance/node_control.rs`

- 原 `tests::node_control::node_control_list_returns_stub_nodes_when_enabled` → `host_migration_18_tests/acceptance/node_control.rs`

- 原 `tests::client_key::normalize_max_keys_uses_fallback_for_zero` → `host_migration_18_tests/acceptance/client_key.rs`

- 原 `tests::client_key::normalize_max_keys_preserves_nonzero_values` → `host_migration_18_tests/acceptance/client_key.rs`

- 原 `tests::auth::require_auth_pairing_disabled_always_allows` → `host_migration_18_tests/acceptance/auth.rs`

- 原 `tests::auth::require_auth_pairing_disabled_ignores_bearer_token` → `host_migration_18_tests/acceptance/auth.rs`

- 原 `tests::auth::require_auth_valid_bearer_token_returns_ok` → `host_migration_18_tests/acceptance/auth.rs`

- 原 `tests::auth::require_auth_missing_token_returns_401` → `host_migration_18_tests/acceptance/auth.rs`

- 原 `tests::auth::require_auth_wrong_token_returns_401` → `host_migration_18_tests/acceptance/auth.rs`

- 原 `tests::auth::require_auth_malformed_auth_header_returns_401` → `host_migration_18_tests/acceptance/auth.rs`

- 原 `tests::auth::require_auth_empty_bearer_returns_401` → `host_migration_18_tests/acceptance/auth.rs`

- 原 `tests::metrics::metrics_endpoint_returns_hint_when_prometheus_is_disabled` → `host_migration_18_tests/acceptance/metrics.rs`

- 原 `tests::metrics::metrics_endpoint_renders_prometheus_output` → `host_migration_18_tests/acceptance/metrics.rs`

- 原 `tests::metrics::metrics_endpoint_rejects_public_clients_when_pairing_is_disabled` → `host_migration_18_tests/acceptance/metrics.rs`

- 原 `tests::metrics::metrics_endpoint_requires_bearer_token_when_pairing_is_enabled` → `host_migration_18_tests/acceptance/metrics.rs`

- 原 `tests::memory_keys::webhook_memory_key_is_unique` → `host_migration_18_tests/acceptance/memory_keys.rs`

- 原 `tests::memory_keys::whatsapp_memory_key_includes_sender_and_message_id` → `host_migration_18_tests/acceptance/memory_keys.rs`

- 原 `tests::memory_keys::qq_memory_key_includes_sender_and_message_id` → `host_migration_18_tests/acceptance/memory_keys.rs`

- 原 `tests::state::app_state_is_clone` → `host_migration_18_tests/acceptance/state.rs`

- 原 `tests::pairing::persist_pairing_tokens_writes_config_tokens` → `host_migration_18_tests/acceptance/pairing.rs`

- 原 `tests::pairing::handle_pair_returns_200_with_token_on_valid_code` → `host_migration_18_tests/acceptance/pairing.rs`

- 原 `tests::pairing::handle_pair_token_authenticates_after_pairing` → `host_migration_18_tests/acceptance/pairing.rs`

- 原 `tests::pairing::handle_pair_code_returns_loopback_bootstrap_code_when_unpaired` → `host_migration_18_tests/acceptance/pairing.rs`

- 原 `tests::pairing::handle_pair_code_rejects_non_loopback_clients` → `host_migration_18_tests/acceptance/pairing.rs`

- 原 `tests::pairing::handle_pair_returns_403_on_wrong_code` → `host_migration_18_tests/acceptance/pairing.rs`

- 原 `tests::pairing::handle_pair_returns_403_when_no_code_header` → `host_migration_18_tests/acceptance/pairing.rs`

- 原 `tests::pairing::handle_pair_returns_403_on_empty_code` → `host_migration_18_tests/acceptance/pairing.rs`

- 原 `tests::pairing::handle_pair_returns_403_when_pairing_mode_disabled` → `host_migration_18_tests/acceptance/pairing.rs`

- 原 `tests::pairing::handle_pair_returns_429_when_rate_limit_exceeded` → `host_migration_18_tests/acceptance/pairing.rs`

- 原 `tests::pairing::handle_pair_returns_429_after_brute_force_lockout` → `host_migration_18_tests/acceptance/pairing.rs`

- 原 `tests::pairing::handle_pair_accepts_pre_seeded_token_immediately` → `host_migration_18_tests/acceptance/pairing.rs`

- 原 `tests::pairing::handle_pair_success_body_has_paired_true_and_token` → `host_migration_18_tests/acceptance/pairing.rs`

- 原 `tests::pairing::handle_pair_failure_body_has_error_field` → `host_migration_18_tests/acceptance/pairing.rs`

- 原 `tests::basics::security_body_limit_is_32mib` → `host_migration_18_tests/acceptance/basics.rs`

- 原 `tests::basics::security_timeout_is_30_seconds` → `host_migration_18_tests/acceptance/basics.rs`

- 原 `tests::basics::workflow_chat_messages_timeout_is_one_hour` → `host_migration_18_tests/acceptance/basics.rs`

## 未来验证

领域新增注册范围含 84 个测试，预期非零：

`python3 scripts/cargo_slot.py cargo test -p agent-gateway --lib host_migration_18_tests -- --test-threads=1`

宿主保留过滤：`app::agent::gateway::mod_tests`、`app::agent::gateway::router::router_tests`、`app::agent::gateway::state::state_tests`；各自预期 [5, 6, 1] 条。未来使用相同槽位入口、包 vw-agent、--lib。迁出原完整名称须对宿主 --list 确认消失，不能用零测试结果当通过。

当前仅源码映射、挂载和依赖方向核对；Cargo、编译、行为、测试 list、性能均未运行；不宣称验证通过。

## 其余跨目录宿主挂载逐用例

- `openai_compat/mod.rs → tests.rs::chat_completions_request_deserializes_minimal`：宿主原挂载保留；对应上述文件真实端口职责，尚可继续拆分纯领域断言。
- `openai_compat/mod.rs → tests.rs::chat_completions_request_deserializes_full`：宿主原挂载保留；对应上述文件真实端口职责，尚可继续拆分纯领域断言。
- `openai_compat/mod.rs → tests.rs::chat_completions_request_rejects_missing_messages`：宿主原挂载保留；对应上述文件真实端口职责，尚可继续拆分纯领域断言。
- `openai_compat/mod.rs → tests.rs::chat_completions_request_rejects_message_without_content`：宿主原挂载保留；对应上述文件真实端口职责，尚可继续拆分纯领域断言。
- `openai_compat/mod.rs → tests.rs::chat_completions_request_rejects_message_without_role`：宿主原挂载保留；对应上述文件真实端口职责，尚可继续拆分纯领域断言。
- `openai_compat/mod.rs → tests.rs::chat_completions_request_accepts_null_optional_fields`：宿主原挂载保留；对应上述文件真实端口职责，尚可继续拆分纯领域断言。
- `openai_compat/mod.rs → tests.rs::chat_completions_request_ignores_unknown_compatibility_fields`：宿主原挂载保留；对应上述文件真实端口职责，尚可继续拆分纯领域断言。
- `openai_compat/mod.rs → tests.rs::chat_completions_response_serializes`：宿主原挂载保留；对应上述文件真实端口职责，尚可继续拆分纯领域断言。
- `openai_compat/mod.rs → tests.rs::models_response_serializes`：宿主原挂载保留；对应上述文件真实端口职责，尚可继续拆分纯领域断言。
- `openai_compat/mod.rs → tests.rs::models_response_serializes_empty_data_array`：宿主原挂载保留；对应上述文件真实端口职责，尚可继续拆分纯领域断言。
- `openai_compat/mod.rs → tests.rs::streaming_chunk_serializes`：宿主原挂载保留；对应上述文件真实端口职责，尚可继续拆分纯领域断言。
- `openai_compat/mod.rs → tests.rs::streaming_chunk_omits_none_fields`：宿主原挂载保留；对应上述文件真实端口职责，尚可继续拆分纯领域断言。
- `openai_compat/mod.rs → tests.rs::streaming_chunk_omits_each_absent_delta_field_independently`：宿主原挂载保留；对应上述文件真实端口职责，尚可继续拆分纯领域断言。
- `openai_compat/mod.rs → tests.rs::chunk_bytes_done_emits_json_frame_then_done_marker`：宿主原挂载保留；对应上述文件真实端口职责，尚可继续拆分纯领域断言。
- `openai_compat/mod.rs → tests.rs::chunk_bytes_preserves_escaped_content_in_single_sse_frame`：宿主原挂载保留；对应上述文件真实端口职责，尚可继续拆分纯领域断言。
- `openai_compat/mod.rs → tests.rs::unix_timestamp_is_within_call_boundaries`：宿主原挂载保留；对应上述文件真实端口职责，尚可继续拆分纯领域断言。
- `openai_compat/mod.rs → mod_tests.rs::handle_v1_chat_completions_rejects_rate_limited_request`：宿主原挂载保留；对应上述文件真实端口职责，尚可继续拆分纯领域断言。
- `openai_compat/mod.rs → mod_tests.rs::handle_v1_chat_completions_rejects_missing_pairing_bearer`：宿主原挂载保留；对应上述文件真实端口职责，尚可继续拆分纯领域断言。
- `openai_compat/mod.rs → mod_tests.rs::handle_v1_chat_completions_rejects_invalid_json_body`：宿主原挂载保留；对应上述文件真实端口职责，尚可继续拆分纯领域断言。
- `openai_compat/mod.rs → mod_tests.rs::handle_v1_chat_completions_rejects_empty_messages`：宿主原挂载保留；对应上述文件真实端口职责，尚可继续拆分纯领域断言。
- `openai_compat/mod.rs → mod_tests.rs::handle_v1_chat_completions_passes_large_body_to_provider`：宿主原挂载保留；对应上述文件真实端口职责，尚可继续拆分纯领域断言。
- `openai_compat/mod.rs → mod_tests.rs::handle_v1_chat_completions_returns_non_streaming_completion`：宿主原挂载保留；对应上述文件真实端口职责，尚可继续拆分纯领域断言。
- `openai_compat/mod.rs → mod_tests.rs::handle_v1_chat_completions_uses_state_defaults_for_empty_model_and_temperature`：宿主原挂载保留；对应上述文件真实端口职责，尚可继续拆分纯领域断言。
- `openai_compat/mod.rs → mod_tests.rs::handle_v1_chat_completions_uses_unknown_provider_label_when_config_has_no_default`：宿主原挂载保留；对应上述文件真实端口职责，尚可继续拆分纯领域断言。
- `openai_compat/mod.rs → mod_tests.rs::handle_v1_chat_completions_returns_provider_error_without_leaking_details`：宿主原挂载保留；对应上述文件真实端口职责，尚可继续拆分纯领域断言。
- `openai_compat/mod.rs → mod_tests.rs::handle_v1_chat_completions_streams_single_chunk_when_provider_lacks_streaming`：宿主原挂载保留；对应上述文件真实端口职责，尚可继续拆分纯领域断言。
- `openai_compat/mod.rs → mod_tests.rs::handle_v1_chat_completions_streams_sanitized_fallback_error`：宿主原挂载保留；对应上述文件真实端口职责，尚可继续拆分纯领域断言。
- `openai_compat/mod.rs → mod_tests.rs::handle_v1_chat_completions_streams_native_chunks_and_done`：宿主原挂载保留；对应上述文件真实端口职责，尚可继续拆分纯领域断言。
- `openai_compat/mod.rs → mod_tests.rs::handle_v1_chat_completions_streams_native_error_chunk`：宿主原挂载保留；对应上述文件真实端口职责，尚可继续拆分纯领域断言。
- `openai_compat/mod.rs → mod_tests.rs::handle_v1_chat_completions_streams_done_after_native_error_without_success_event`：宿主原挂载保留；对应上述文件真实端口职责，尚可继续拆分纯领域断言。
- `openai_compat/mod.rs → mod_tests.rs::handle_v1_models_rejects_missing_pairing_bearer`：宿主原挂载保留；对应上述文件真实端口职责，尚可继续拆分纯领域断言。
- `openai_compat/mod.rs → mod_tests.rs::handle_v1_models_allows_request_when_pairing_is_disabled`：宿主原挂载保留；对应上述文件真实端口职责，尚可继续拆分纯领域断言。
- `openai_compat/mod.rs → mod_tests.rs::handle_v1_models_returns_configured_model_for_valid_request`：宿主原挂载保留；对应上述文件真实端口职责，尚可继续拆分纯领域断言。
- `openai_compat/mod.rs → mod_tests.rs::chunk_bytes_without_done_omits_done_marker_and_empty_fields`：宿主原挂载保留；对应上述文件真实端口职责，尚可继续拆分纯领域断言。
- `openai_compat/mod.rs → mod_tests.rs::helper_record_functions_accept_success_and_failure_events`：宿主原挂载保留；对应上述文件真实端口职责，尚可继续拆分纯领域断言。
- `openai_compat/mod.rs → mod_tests.rs::response_helpers_read_empty_body_text`：宿主原挂载保留；对应上述文件真实端口职责，尚可继续拆分纯领域断言。
- `runtime.rs → runtime_tests.rs::runtime_entrypoints_are_available`：宿主原挂载保留；对应上述文件真实端口职责，尚可继续拆分纯领域断言。
- `runtime.rs → runtime_tests.rs::bind_prefer_uses_requested_available_port`：宿主原挂载保留；对应上述文件真实端口职责，尚可继续拆分纯领域断言。
- `runtime.rs → runtime_tests.rs::bind_prefer_rejects_occupied_requested_port`：宿主原挂载保留；对应上述文件真实端口职责，尚可继续拆分纯领域断言。
- `runtime.rs → runtime_tests.rs::bind_prefer_rejects_invalid_host`：宿主原挂载保留；对应上述文件真实端口职责，尚可继续拆分纯领域断言。
- `runtime.rs → runtime_tests.rs::bind_prefer_zero_port_prefers_4099_then_falls_back_when_occupied`：宿主原挂载保留；对应上述文件真实端口职责，尚可继续拆分纯领域断言。
- `runtime.rs → runtime_tests.rs::start_binds_service_and_returns_actual_address`：宿主原挂载保留；对应上述文件真实端口职责，尚可继续拆分纯领域断言。
- `runtime.rs → runtime_tests.rs::start_until_uses_custom_shutdown_future`：宿主原挂载保留；对应上述文件真实端口职责，尚可继续拆分纯领域断言。
- `runtime.rs → runtime_tests.rs::start_until_keeps_service_running_until_shutdown_is_requested`：宿主原挂载保留；对应上述文件真实端口职责，尚可继续拆分纯领域断言。
- `runtime.rs → runtime_tests.rs::start_returns_bind_error_for_occupied_requested_port`：宿主原挂载保留；对应上述文件真实端口职责，尚可继续拆分纯领域断言。
- `runtime.rs → runtime_tests.rs::start_until_returns_bind_error_without_polling_shutdown`：宿主原挂载保留；对应上述文件真实端口职责，尚可继续拆分纯领域断言。
- `runtime.rs → runtime_tests.rs::serve_returns_actual_address_after_graceful_shutdown`：宿主原挂载保留；对应上述文件真实端口职责，尚可继续拆分纯领域断言。
- `runtime.rs → runtime_tests.rs::serve_until_returns_address_when_shutdown_future_panics`：宿主原挂载保留；对应上述文件真实端口职责，尚可继续拆分纯领域断言。
- `runtime.rs → runtime_tests.rs::serve_returns_bind_error_for_occupied_port`：宿主原挂载保留；对应上述文件真实端口职责，尚可继续拆分纯领域断言。
- `runtime.rs → runtime_tests.rs::local_import_runtime_supplies_real_peer_address`：宿主原挂载保留；对应上述文件真实端口职责，尚可继续拆分纯领域断言。
- `state.rs → state_tests.rs::app_state_type_is_send_sync`：宿主原挂载保留；对应上述文件真实端口职责，尚可继续拆分纯领域断言。
- `webhook.rs → webhook_tests.rs::webhook_handler_is_available`：宿主原挂载保留；对应上述文件真实端口职责，尚可继续拆分纯领域断言。
- `webhook.rs → webhook_tests.rs::handle_webhook_accepts_loopback_request`：宿主原挂载保留；对应上述文件真实端口职责，尚可继续拆分纯领域断言。
- `webhook.rs → webhook_tests.rs::handle_webhook_rejects_public_request_without_auth`：宿主原挂载保留；对应上述文件真实端口职责，尚可继续拆分纯领域断言。
- `webhook.rs → webhook_tests.rs::handle_webhook_accepts_valid_secret_header`：宿主原挂载保留；对应上述文件真实端口职责，尚可继续拆分纯领域断言。
- `webhook.rs → webhook_tests.rs::handle_webhook_returns_bad_request_for_json_rejection`：宿主原挂载保留；对应上述文件真实端口职责，尚可继续拆分纯领域断言。
- `webhook.rs → webhook_tests.rs::handle_webhook_maps_provider_errors_to_internal_server_error`：宿主原挂载保留；对应上述文件真实端口职责，尚可继续拆分纯领域断言。
- `webhook_ingress/mod.rs → tests.rs::extract_idempotency_key_handles_missing_blank_trimmed_and_invalid_values`：宿主原挂载保留；对应上述文件真实端口职责，尚可继续拆分纯领域断言。
- `webhook_ingress/mod.rs → tests.rs::extract_webhook_secret_header_hash_trims_and_hashes_nonempty_secret`：宿主原挂载保留；对应上述文件真实端口职责，尚可继续拆分纯领域断言。
- `webhook_ingress/mod.rs → tests.rs::parse_webhook_body_accepts_valid_json_body`：宿主原挂载保留；对应上述文件真实端口职责，尚可继续拆分纯领域断言。
- `webhook_ingress/mod.rs → tests.rs::parse_webhook_body_rejects_invalid_json_body`：宿主原挂载保留；对应上述文件真实端口职责，尚可继续拆分纯领域断言。
- `webhook_ingress/mod.rs → tests.rs::enforce_idempotency_allows_missing_and_first_key_then_rejects_duplicate`：宿主原挂载保留；对应上述文件真实端口职责，尚可继续拆分纯领域断言。
- `webhook_ingress/mod.rs → tests.rs::enforce_idempotency_treats_trimmed_equivalent_keys_as_duplicates`：宿主原挂载保留；对应上述文件真实端口职责，尚可继续拆分纯领域断言。
- `webhook_ingress/mod.rs → tests.rs::enforce_rate_limit_returns_retry_after_after_limit_is_exceeded`：宿主原挂载保留；对应上述文件真实端口职责，尚可继续拆分纯领域断言。
- `webhook_ingress/mod.rs → tests.rs::enforce_rate_limit_scopes_limits_to_each_peer_address`：宿主原挂载保留；对应上述文件真实端口职责，尚可继续拆分纯领域断言。
- `webhook_ingress/mod.rs → tests.rs::authorize_webhook_request_allows_loopback_without_auth_and_rejects_remote`：宿主原挂载保留；对应上述文件真实端口职责，尚可继续拆分纯领域断言。
- `webhook_ingress/mod.rs → tests.rs::authorize_webhook_request_requires_valid_bearer_when_pairing_is_enabled`：宿主原挂载保留；对应上述文件真实端口职责，尚可继续拆分纯领域断言。
- `webhook_ingress/mod.rs → tests.rs::authorize_webhook_request_requires_matching_webhook_secret_when_configured`：宿主原挂载保留；对应上述文件真实端口职责，尚可继续拆分纯领域断言。
- `webhook_ingress/mod.rs → tests.rs::authorize_webhook_request_requires_both_credentials_when_both_are_configured`：宿主原挂载保留；对应上述文件真实端口职责，尚可继续拆分纯领域断言。
- `webhook_ingress/mod.rs → tests.rs::maybe_persist_inbound_message_skips_when_auto_save_is_disabled`：宿主原挂载保留；对应上述文件真实端口职责，尚可继续拆分纯领域断言。
- `webhook_ingress/mod.rs → tests.rs::maybe_persist_inbound_message_stores_conversation_when_auto_save_is_enabled`：宿主原挂载保留；对应上述文件真实端口职责，尚可继续拆分纯领域断言。
- `webhook_ingress/mod.rs → tests.rs::handle_webhook_inner_continues_when_auto_save_store_fails`：宿主原挂载保留；对应上述文件真实端口职责，尚可继续拆分纯领域断言。
- `webhook_ingress/mod.rs → tests.rs::handle_webhook_inner_returns_success_payload_for_authorized_loopback_request`：宿主原挂载保留；对应上述文件真实端口职责，尚可继续拆分纯领域断言。
- `webhook_ingress/mod.rs → tests.rs::handle_webhook_inner_returns_rate_limit_before_provider_call`：宿主原挂载保留；对应上述文件真实端口职责，尚可继续拆分纯领域断言。
- `webhook_ingress/mod.rs → tests.rs::handle_webhook_inner_returns_auth_error_before_provider_call`：宿主原挂载保留；对应上述文件真实端口职责，尚可继续拆分纯领域断言。
- `webhook_ingress/mod.rs → tests.rs::handle_webhook_inner_returns_bad_request_before_provider_call`：宿主原挂载保留；对应上述文件真实端口职责，尚可继续拆分纯领域断言。
- `webhook_ingress/mod.rs → tests.rs::handle_webhook_inner_returns_duplicate_without_second_provider_call`：宿主原挂载保留；对应上述文件真实端口职责，尚可继续拆分纯领域断言。
- `webhook_ingress/mod.rs → tests.rs::handle_webhook_inner_returns_provider_error_without_leaking_details`：宿主原挂载保留；对应上述文件真实端口职责，尚可继续拆分纯领域断言。
- `webhook_ingress/mod.rs → tests.rs::handle_webhook_inner_records_success_telemetry`：宿主原挂载保留；对应上述文件真实端口职责，尚可继续拆分纯领域断言。
- `webhook_ingress/mod.rs → tests.rs::handle_webhook_inner_records_error_telemetry_with_unknown_provider`：宿主原挂载保留；对应上述文件真实端口职责，尚可继续拆分纯领域断言。
- `webhook_ingress/mod.rs → tests.rs::unavailable_webhook_credentials_reject_local_unauthenticated_requests`：宿主原挂载保留；对应上述文件真实端口职责，尚可继续拆分纯领域断言。
- `ws/mod.rs → mod_tests.rs::gateway_websocket_passes_hook_runtime_to_tool_loop`：宿主原挂载保留；对应上述文件真实端口职责，尚可继续拆分纯领域断言。
- `ws/mod.rs → mod_tests.rs::websocket_entrypoints_and_constants_are_available`：宿主原挂载保留；对应上述文件真实端口职责，尚可继续拆分纯领域断言。
- `ws/mod.rs → mod_tests.rs::extract_ws_bearer_token_falls_back_when_authorization_is_not_bearer`：宿主原挂载保留；对应上述文件真实端口职责，尚可继续拆分纯领域断言。
- `ws/mod.rs → mod_tests.rs::parse_ws_private_event_uses_plain_payload_defaults`：宿主原挂载保留；对应上述文件真实端口职责，尚可继续拆分纯领域断言。
- `ws/mod.rs → mod_tests.rs::parse_ws_private_event_prefers_explicit_name_over_result_tool_id`：宿主原挂载保留；对应上述文件真实端口职责，尚可继续拆分纯领域断言。
- `ws/mod.rs → mod_tests.rs::parse_ws_delta_event_maps_empty_hint_to_none_and_bad_duration_to_none`：宿主原挂载保留；对应上述文件真实端口职责，尚可继续拆分纯领域断言。
- `ws/mod.rs → mod_tests.rs::websocket_upgrade_requires_real_hyper_upgrade_extension`：宿主原挂载保留；对应上述文件真实端口职责，尚可继续拆分纯领域断言。
