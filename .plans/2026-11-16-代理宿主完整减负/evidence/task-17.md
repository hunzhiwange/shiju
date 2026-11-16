# 任务17证据

代码迁移与静态接线完成；编译/行为未验证。未运行任何 Cargo、构建、单元或实机测试。

## 实际变更与调用方向

状态/工具测试使用 gateway-types 已有泛型 AppState<(),(),()>，不复制宿主状态类型；使用真实 PairingGuard、NoneMemory、共享 require_auth、agent-doctor。test-only 适配函数仅绑定泛型端口，未重写响应算法。宿主原 status/tools 测试挂载删除。middleware 旧默认挂载路径错误随原模块删除而消除，领域直接注册。

工作流纯请求转换、协议事件与 SSE 状态机整函数迁入 gateway-workflow，由 lib.rs 自注册；不挂在 server.rs，避免宿主 server_test_source 再次编译该组。真实数据库/运行时/工具权限集成保留。聊天清洗与日志整组进入 agent-gateway；原宿主挂载文件中删除对应定义。

生产调用保持 bootstrap→宿主具体端口→已有领域实现；此次生产业务迁移收益为0，不能把之前已下沉实现计入。新增依赖仅 agent-gateway 的 dev 依赖，无 vw-agent 反向依赖，无新增 path/include 加载宿主。

## 34个来源逐文件及逐符号映射

|来源|前→后物理行数|归属/保留职责及符号|
|---|---:|---|
|`crates/vw-agent/src/gateway/api/handlers/shiju_skills_tests.rs`|356→356|真实 ShijuSkillClient HTTP 转发及宿主 ShijuSkillService 实现契约，保留本地 TCP 上游与凭证裁剪断言；继续可迁：将服务适配器交 gateway-shiju 所有者后迁整组（不伪造 service）。 符号：response, upstream_fixture_with_response, upstream_fixture, shiju_skill_readme_route_forwards_markdown_content, shiju_skill_routes_forward_maintained_multi_value_filters, shiju_skill_routes_forward_search_filters, shiju_skill_routes_forward_tag_catalogs, shiju_skill_routes_forward_empty_and_version_install_bodies, shiju_skill_routes_reject_missing_credential_and_invalid_filters|
|`crates/vw-agent/src/gateway/api/handlers/skill_market.rs`|24→24|真实宿主装配/具体端口或已有兼容导出；未计作新增领域生产收益。 符号：skills|
|`crates/vw-agent/src/gateway/api/handlers/skill_market_tests.rs`|392→392|专家包安装→真实 AgentContextHost SkillTool 调用、上下文更新与工作流正文的跨层验收；RecordingProvider、签名 ZIP 为该唯一场景夹具。继续可迁：工具宿主适配器下沉后迁此完整场景。 符号：new, record, source, recommended, skills, categories, skill_detail, skill_file, skill_artifact, skill_sets, skill_set_detail, skill_set_artifact, locator, coordinate, skill, skill_set, registry, post_response, expert_fixture_zip, valid_expert_fixture, expert_package_install_succeeds_without_connector_credentials|
|`crates/vw-agent/src/gateway/api/handlers/status.rs`|15→11|真实宿主装配/具体端口或已有兼容导出；未计作新增领域生产收益。 符号：handle_api_status, handle_api_health|
|`crates/vw-agent/src/gateway/api/handlers/status_tests.rs`|168→0|整组迁入 agent-gateway::host_migration_17_tests，原文件/挂载删除。|
|`crates/vw-agent/src/gateway/api/handlers/task_lifecycle.rs`|2→2|真实宿主装配/具体端口或已有兼容导出；未计作新增领域生产收益。|
|`crates/vw-agent/src/gateway/api/handlers/task_plan_run.rs`|47→47|真实宿主装配/具体端口或已有兼容导出；未计作新增领域生产收益。 符号：status, stop, register, observe_event|
|`crates/vw-agent/src/gateway/api/handlers/task_plan_run_tests.rs`|145→145|真实 Host 的控制注册表、child coordinator 取消、租户隔离、幂等停止及 resumable stream 冲突；继续可迁：路径常量、panic_message 与请求校验场景可独立迁 gateway-task。 符号：sample_request, default_tenant, task_plan_run_rejects_empty_root, task_plan_run_rejects_empty_query, task_plan_run_router_is_wired, task_plan_run_stream_path_matches_desktop_client, task_plan_run_control_routes_are_tenant_scoped_and_stop_is_idempotent, panic_message_reads_str_and_string_payloads, task_plan_run_rejects_reused_request_id_with_different_body|
|`crates/vw-agent/src/gateway/api/handlers/task_pool.rs`|2→2|真实宿主装配/具体端口或已有兼容导出；未计作新增领域生产收益。|
|`crates/vw-agent/src/gateway/api/handlers/tools.rs`|43→39|真实宿主装配/具体端口或已有兼容导出；未计作新增领域生产收益。 符号：require_auth, diagnose, severity, handle_api_tools, handle_api_doctor|
|`crates/vw-agent/src/gateway/api/handlers/tools_tests.rs`|182→0|整组迁入 agent-gateway::host_migration_17_tests，原文件/挂载删除。|
|`crates/vw-agent/src/gateway/api/handlers/usage/mod.rs`|38→38|真实宿主装配/具体端口或已有兼容导出；未计作新增领域生产收益。 符号：router|
|`crates/vw-agent/src/gateway/api/handlers/voice.rs`|42→42|真实宿主装配/具体端口或已有兼容导出；未计作新增领域生产收益。 符号：load_session, load_snapshot, approval_manager, collect_chat|
|`crates/vw-agent/src/gateway/api/handlers/voice_handoff.rs`|3→3|真实宿主装配/具体端口或已有兼容导出；未计作新增领域生产收益。|
|`crates/vw-agent/src/gateway/api/handlers/voice_settings.rs`|15→15|真实宿主装配/具体端口或已有兼容导出；未计作新增领域生产收益。|
|`crates/vw-agent/src/gateway/api/handlers/workflow.rs`|57→57|真实宿主装配/具体端口或已有兼容导出；未计作新增领域生产收益。 符号：router, workflow_runtime, workflow_tool_runtime_context|
|`crates/vw-agent/src/gateway/api/handlers/workspace_directory.rs`|4→4|真实宿主装配/具体端口或已有兼容导出；未计作新增领域生产收益。 符号：router|
|`crates/vw-agent/src/gateway/api/handlers/workspace_memory.rs`|15→15|真实宿主装配/具体端口或已有兼容导出；未计作新增领域生产收益。 符号：router, authorize_workspace|
|`crates/vw-agent/src/gateway/api/handlers/workspace_memory_tests.rs`|39→39|真实 access 策略→workspace 路由授权类型转换及 native workspace UTF-8 IO 契约。 符号：workspace_api_denies_foreign_host_and_project_without_io|
|`crates/vw-agent/src/gateway/api/handlers/worktree_management.rs`|33→33|真实宿主装配/具体端口或已有兼容导出；未计作新增领域生产收益。 符号：context, settings|
|`crates/vw-agent/src/gateway/api/handlers/worktree_management_tests.rs`|258→258|真实宿主 project 路由、持久化配置、registry 与 Git worktree 删除/快照/恢复及跨租户验收；不能以临时 mock context 代替。继续可迁：纯 fingerprint/快照策略在 gateway-worktree 继续抽取。 符号：git, call, worktree_management_rejects_cross_scope_operations, worktree_management_deletion_requires_valid_confirmation, worktree_managed_ui_records_ready_checkout_use_in_scope|
|`crates/vw-agent/src/gateway/approval_state.rs`|8→8|真实宿主装配/具体端口或已有兼容导出；未计作新增领域生产收益。 符号：approval_manager_for_current_instance|
|`crates/vw-agent/src/gateway/bootstrap.rs`|421→421|真实宿主装配/具体端口或已有兼容导出；未计作新增领域生产收益。 符号：run_gateway, run_gateway_until, serve_gateway, external_router, client_router, stateful_handler_router|
|`crates/vw-agent/src/gateway/channel_reload.rs`|34→34|真实宿主装配/具体端口或已有兼容导出；未计作新增领域生产收益。 符号：whatsapp, build_gateway_channels, watch_gateway_channels, watch_gateway_channels_with_source, poll|
|`crates/vw-agent/src/gateway/channel_runtime.rs`|27→27|真实宿主装配/具体端口或已有兼容导出；未计作新增领域生产收益。 符号：channel_runtime|
|`crates/vw-agent/src/gateway/chat.rs`|73→71|真实宿主装配/具体端口或已有兼容导出；未计作新增领域生产收益。 符号：tenant, extractor, simple_prompt, forked_session, load_fork_history, new_engine, replace_history, snapshot, restore_snapshot, stats, session_query_engine, session_query_engine_key, invalidate_session_query_engine, invalidate_all_session_query_engines, fork_session_query_engine, run_gateway_chat_simple, run_gateway_chat_simple_with_extractor, run_gateway_chat_with_tools|
|`crates/vw-agent/src/gateway/error.rs`|2→2|真实宿主装配/具体端口或已有兼容导出；未计作新增领域生产收益。|
|`crates/vw-agent/src/gateway/handlers.rs`|132→132|真实宿主装配/具体端口或已有兼容导出；未计作新增领域生产收益。 符号：verify_token, parse_webhook_payload, handle_whatsapp_verify, handle_wati_verify, handle_whatsapp_message, handle_linq_webhook, handle_nextcloud_talk_webhook, handle_qq_webhook, handle_wati_webhook|
|`crates/vw-agent/src/gateway/health.rs`|5→5|真实宿主装配/具体端口或已有兼容导出；未计作新增领域生产收益。 符号：handle_health, handle_metrics|
|`crates/vw-agent/src/gateway/instance.rs`|16→16|真实宿主装配/具体端口或已有兼容导出；未计作新增领域生产收益。 符号：instance_init|
|`crates/vw-agent/src/gateway/limits.rs`|2→2|真实宿主装配/具体端口或已有兼容导出；未计作新增领域生产收益。|
|`crates/vw-agent/src/gateway/middleware.rs`|4→1|真实宿主装配/具体端口或已有兼容导出；未计作新增领域生产收益。|
|`crates/vw-agent/src/gateway/middleware_tests.rs`|17→0|整组迁入 agent-gateway::host_migration_17_tests，原文件/挂载删除。|
|`crates/vw-agent/src/gateway/mod.rs`|112→110|真实宿主装配/具体端口或已有兼容导出；未计作新增领域生产收益。 符号：wecom_acceptance_router|

- `crates/vw-agent/src/gateway/api/handlers/shiju_skills_tests.rs::shiju_skill_readme_route_forwards_markdown_content` → 原挂载保留；真实 ShijuSkillClient HTTP 转发及宿主 ShijuSkillService 实现契约，保留本地 TCP 上游与凭证裁剪断言；继续可迁：将服务适配器交 gateway-shiju 所有者后迁整组（不伪造 service）。

- `crates/vw-agent/src/gateway/api/handlers/shiju_skills_tests.rs::shiju_skill_routes_forward_maintained_multi_value_filters` → 原挂载保留；真实 ShijuSkillClient HTTP 转发及宿主 ShijuSkillService 实现契约，保留本地 TCP 上游与凭证裁剪断言；继续可迁：将服务适配器交 gateway-shiju 所有者后迁整组（不伪造 service）。

- `crates/vw-agent/src/gateway/api/handlers/shiju_skills_tests.rs::shiju_skill_routes_forward_search_filters` → 原挂载保留；真实 ShijuSkillClient HTTP 转发及宿主 ShijuSkillService 实现契约，保留本地 TCP 上游与凭证裁剪断言；继续可迁：将服务适配器交 gateway-shiju 所有者后迁整组（不伪造 service）。

- `crates/vw-agent/src/gateway/api/handlers/shiju_skills_tests.rs::shiju_skill_routes_forward_tag_catalogs` → 原挂载保留；真实 ShijuSkillClient HTTP 转发及宿主 ShijuSkillService 实现契约，保留本地 TCP 上游与凭证裁剪断言；继续可迁：将服务适配器交 gateway-shiju 所有者后迁整组（不伪造 service）。

- `crates/vw-agent/src/gateway/api/handlers/shiju_skills_tests.rs::shiju_skill_routes_forward_empty_and_version_install_bodies` → 原挂载保留；真实 ShijuSkillClient HTTP 转发及宿主 ShijuSkillService 实现契约，保留本地 TCP 上游与凭证裁剪断言；继续可迁：将服务适配器交 gateway-shiju 所有者后迁整组（不伪造 service）。

- `crates/vw-agent/src/gateway/api/handlers/shiju_skills_tests.rs::shiju_skill_routes_reject_missing_credential_and_invalid_filters` → 原挂载保留；真实 ShijuSkillClient HTTP 转发及宿主 ShijuSkillService 实现契约，保留本地 TCP 上游与凭证裁剪断言；继续可迁：将服务适配器交 gateway-shiju 所有者后迁整组（不伪造 service）。

- `crates/vw-agent/src/gateway/api/handlers/skill_market_tests.rs::expert_package_install_succeeds_without_connector_credentials` → 原挂载保留；专家包安装→真实 AgentContextHost SkillTool 调用、上下文更新与工作流正文的跨层验收；RecordingProvider、签名 ZIP 为该唯一场景夹具。继续可迁：工具宿主适配器下沉后迁此完整场景。

- `crates/vw-agent/src/gateway/api/handlers/status_tests.rs::status_handlers_are_available` → `crates/libs/agent-gateway/src/host_migration_17_tests/status.rs::status_handlers_are_available`（原函数断言保留）

- `crates/vw-agent/src/gateway/api/handlers/status_tests.rs::handle_api_status_returns_runtime_status_when_authorized` → `crates/libs/agent-gateway/src/host_migration_17_tests/status.rs::handle_api_status_returns_runtime_status_when_authorized`（原函数断言保留）

- `crates/vw-agent/src/gateway/api/handlers/status_tests.rs::handle_api_status_reports_channel_presence_and_absent_provider` → `crates/libs/agent-gateway/src/host_migration_17_tests/status.rs::handle_api_status_reports_channel_presence_and_absent_provider`（原函数断言保留）

- `crates/vw-agent/src/gateway/api/handlers/status_tests.rs::handle_api_health_returns_health_snapshot_when_authorized` → `crates/libs/agent-gateway/src/host_migration_17_tests/status.rs::handle_api_health_returns_health_snapshot_when_authorized`（原函数断言保留）

- `crates/vw-agent/src/gateway/api/handlers/status_tests.rs::status_and_health_reject_missing_bearer_when_skey_auth_required` → `crates/libs/agent-gateway/src/host_migration_17_tests/status.rs::status_and_health_reject_missing_bearer_when_skey_auth_required`（原函数断言保留）

- `crates/vw-agent/src/gateway/api/handlers/status_tests.rs::status_accepts_valid_bearer_skey` → `crates/libs/agent-gateway/src/host_migration_17_tests/status.rs::status_accepts_valid_bearer_skey`（原函数断言保留）

- `crates/vw-agent/src/gateway/api/handlers/status_tests.rs::health_accepts_valid_bearer_skey` → `crates/libs/agent-gateway/src/host_migration_17_tests/status.rs::health_accepts_valid_bearer_skey`（原函数断言保留）

- `crates/vw-agent/src/gateway/api/handlers/task_plan_run_tests.rs::task_plan_run_rejects_empty_root` → 原挂载保留；真实 Host 的控制注册表、child coordinator 取消、租户隔离、幂等停止及 resumable stream 冲突；继续可迁：路径常量、panic_message 与请求校验场景可独立迁 gateway-task。

- `crates/vw-agent/src/gateway/api/handlers/task_plan_run_tests.rs::task_plan_run_rejects_empty_query` → 原挂载保留；真实 Host 的控制注册表、child coordinator 取消、租户隔离、幂等停止及 resumable stream 冲突；继续可迁：路径常量、panic_message 与请求校验场景可独立迁 gateway-task。

- `crates/vw-agent/src/gateway/api/handlers/task_plan_run_tests.rs::task_plan_run_router_is_wired` → 原挂载保留；真实 Host 的控制注册表、child coordinator 取消、租户隔离、幂等停止及 resumable stream 冲突；继续可迁：路径常量、panic_message 与请求校验场景可独立迁 gateway-task。

- `crates/vw-agent/src/gateway/api/handlers/task_plan_run_tests.rs::task_plan_run_stream_path_matches_desktop_client` → 原挂载保留；真实 Host 的控制注册表、child coordinator 取消、租户隔离、幂等停止及 resumable stream 冲突；继续可迁：路径常量、panic_message 与请求校验场景可独立迁 gateway-task。

- `crates/vw-agent/src/gateway/api/handlers/task_plan_run_tests.rs::task_plan_run_control_routes_are_tenant_scoped_and_stop_is_idempotent` → 原挂载保留；真实 Host 的控制注册表、child coordinator 取消、租户隔离、幂等停止及 resumable stream 冲突；继续可迁：路径常量、panic_message 与请求校验场景可独立迁 gateway-task。

- `crates/vw-agent/src/gateway/api/handlers/task_plan_run_tests.rs::panic_message_reads_str_and_string_payloads` → 原挂载保留；真实 Host 的控制注册表、child coordinator 取消、租户隔离、幂等停止及 resumable stream 冲突；继续可迁：路径常量、panic_message 与请求校验场景可独立迁 gateway-task。

- `crates/vw-agent/src/gateway/api/handlers/task_plan_run_tests.rs::task_plan_run_rejects_reused_request_id_with_different_body` → 原挂载保留；真实 Host 的控制注册表、child coordinator 取消、租户隔离、幂等停止及 resumable stream 冲突；继续可迁：路径常量、panic_message 与请求校验场景可独立迁 gateway-task。

- `crates/vw-agent/src/gateway/api/handlers/tools_tests.rs::tool_handlers_are_available` → `crates/libs/agent-gateway/src/host_migration_17_tests/tools.rs::tool_handlers_are_available`（原函数断言保留）

- `crates/vw-agent/src/gateway/api/handlers/tools_tests.rs::handle_api_tools_returns_registered_specs_when_authorized` → `crates/libs/agent-gateway/src/host_migration_17_tests/tools.rs::handle_api_tools_returns_registered_specs_when_authorized`（原函数断言保留）

- `crates/vw-agent/src/gateway/api/handlers/tools_tests.rs::handle_api_tools_returns_empty_items_for_empty_registry` → `crates/libs/agent-gateway/src/host_migration_17_tests/tools.rs::handle_api_tools_returns_empty_items_for_empty_registry`（原函数断言保留）

- `crates/vw-agent/src/gateway/api/handlers/tools_tests.rs::handle_api_tools_preserves_registry_order_and_capabilities` → `crates/libs/agent-gateway/src/host_migration_17_tests/tools.rs::handle_api_tools_preserves_registry_order_and_capabilities`（原函数断言保留）

- `crates/vw-agent/src/gateway/api/handlers/tools_tests.rs::handle_api_doctor_returns_results_and_summary_when_authorized` → `crates/libs/agent-gateway/src/host_migration_17_tests/tools.rs::handle_api_doctor_returns_results_and_summary_when_authorized`（原函数断言保留）

- `crates/vw-agent/src/gateway/api/handlers/tools_tests.rs::tool_handlers_reject_missing_or_wrong_bearer_when_pairing_required` → `crates/libs/agent-gateway/src/host_migration_17_tests/tools.rs::tool_handlers_reject_missing_or_wrong_bearer_when_pairing_required`（原函数断言保留）

- `crates/vw-agent/src/gateway/api/handlers/tools_tests.rs::tool_handlers_accept_valid_pairing_bearer` → `crates/libs/agent-gateway/src/host_migration_17_tests/tools.rs::tool_handlers_accept_valid_pairing_bearer`（原函数断言保留）

- `crates/vw-agent/src/gateway/api/handlers/workspace_memory_tests.rs::workspace_api_denies_foreign_host_and_project_without_io` → 原挂载保留；真实 access 策略→workspace 路由授权类型转换及 native workspace UTF-8 IO 契约。

- `crates/vw-agent/src/gateway/api/handlers/worktree_management_tests.rs::worktree_management_rejects_cross_scope_operations` → 原挂载保留；真实宿主 project 路由、持久化配置、registry 与 Git worktree 删除/快照/恢复及跨租户验收；不能以临时 mock context 代替。继续可迁：纯 fingerprint/快照策略在 gateway-worktree 继续抽取。

- `crates/vw-agent/src/gateway/api/handlers/worktree_management_tests.rs::worktree_management_deletion_requires_valid_confirmation` → 原挂载保留；真实宿主 project 路由、持久化配置、registry 与 Git worktree 删除/快照/恢复及跨租户验收；不能以临时 mock context 代替。继续可迁：纯 fingerprint/快照策略在 gateway-worktree 继续抽取。

- `crates/vw-agent/src/gateway/api/handlers/worktree_management_tests.rs::worktree_managed_ui_records_ready_checkout_use_in_scope` → 原挂载保留；真实宿主 project 路由、持久化配置、registry 与 Git worktree 删除/快照/恢复及跨租户验收；不能以临时 mock context 代替。继续可迁：纯 fingerprint/快照策略在 gateway-worktree 继续抽取。

- `crates/vw-agent/src/gateway/middleware_tests.rs::basic_auth_production_entry_allows_when_password_flag_is_absent` → `crates/libs/agent-gateway/src/host_migration_17_tests/middleware.rs::basic_auth_production_entry_allows_when_password_flag_is_absent`（原函数断言保留）

## 跨目录挂载逐用例清单

以下保留宿主编译归属者不计作领域减量；server.rs 是宿主私有可见性测试绑定，不是新生产迁移。

### `crates/libs/agent-gateway/src/bootstrap_tests.rs`
物理行数 394→394；保留具体宿主启动路由、租户、会话缓存/OCR/钩子、native voice 存储/交接、真实工作流运行与工具端口集成。
- `external_router_registers_only_declared_webhook_methods` → 原宿主挂载同名函数；真实组合路径保留。
- `gateway_route_summary_lists_tenant_policy_and_client_api_entries` → 原宿主挂载同名函数；真实组合路径保留。
- `gateway_route_summary_lists_all_documented_routes` → 原宿主挂载同名函数；真实组合路径保留。
- `stateful_handler_router_registers_channel_qr_routes` → 原宿主挂载同名函数；真实组合路径保留。
- `stateful_handler_router_registers_shiju_gateway_routes` → 原宿主挂载同名函数；真实组合路径保留。
- `gateway_route_summary_hides_disabled_optional_routes` → 原宿主挂载同名函数；真实组合路径保留。
- `unavailable_default_provider_keeps_health_route_available_and_rejects_inference` → 原宿主挂载同名函数；真实组合路径保留。
- `supported_default_provider_is_initialized_normally` → 原宿主挂载同名函数；真实组合路径保留。
- `dynamic_tools_gateway_bootstrap_mounts_authenticated_host_routes` → 原宿主挂载同名函数；真实组合路径保留。
- `cc_apps_production_gateway_routes_preserve_authorization` → 原宿主挂载同名函数；真实组合路径保留。
- `cc_apps_omp_provider_stateful_gateway_persists_and_isolates_tenant_records` → 原宿主挂载同名函数；真实组合路径保留。

### `crates/libs/agent-gateway/src/chat_tests.rs`
物理行数 474→383；保留具体宿主启动路由、租户、会话缓存/OCR/钩子、native voice 存储/交接、真实工作流运行与工具端口集成。
- `run_gateway_chat_simple_uses_state_model_temperature_and_system_prompt` → 原宿主挂载同名函数；真实组合路径保留。
- `run_gateway_chat_simple_propagates_provider_error` → 原宿主挂载同名函数；真实组合路径保留。
- `run_gateway_chat_simple_uses_ocr_text_for_non_vision_provider` → 原宿主挂载同名函数；真实组合路径保留。
- `invalidate_session_query_engine_allows_missing_session` → 原宿主挂载同名函数；真实组合路径保留。
- `fork_session_query_engine_allows_missing_source_session` → 原宿主挂载同名函数；真实组合路径保留。
- `session_query_engine_caches_forks_and_invalidates_engines` → 原宿主挂载同名函数；真实组合路径保留。
- `fork_session_query_engine_copies_source_history_and_replaces_target_engine` → 原宿主挂载同名函数；真实组合路径保留。
- `fork_session_query_engine_reuses_injected_provider_after_config_changes` → 原宿主挂载同名函数；真实组合路径保留。
- `run_gateway_chat_with_tools_reuses_injected_provider` → 原宿主挂载同名函数；真实组合路径保留。
- `gateway_chat_uses_shared_hook_runtime` → 原宿主挂载同名函数；真实组合路径保留。
- `sanitize_gateway_response_keeps_plain_text` → `crates/libs/agent-gateway/src/host_migration_17_tests/chat.rs`，同名函数
- `sanitize_gateway_response_keeps_empty_and_drops_blank_text` → `crates/libs/agent-gateway/src/host_migration_17_tests/chat.rs`，同名函数
- `sanitize_gateway_response_returns_safe_message_when_nonempty_response_is_stripped` → `crates/libs/agent-gateway/src/host_migration_17_tests/chat.rs`，同名函数
- `sanitize_gateway_response_removes_known_isolated_tool_json` → `crates/libs/agent-gateway/src/host_migration_17_tests/chat.rs`，同名函数
- `sanitize_gateway_response_truncates_long_text` → `crates/libs/agent-gateway/src/host_migration_17_tests/chat.rs`，同名函数
- `sanitize_gateway_response_keeps_text_at_limit_without_ellipsis` → `crates/libs/agent-gateway/src/host_migration_17_tests/chat.rs`，同名函数
- `sanitize_gateway_response_normalizes_short_trailing_ellipsis` → `crates/libs/agent-gateway/src/host_migration_17_tests/chat.rs`，同名函数
- `log_channel_message_accepts_long_content` → `crates/libs/agent-gateway/src/host_migration_17_tests/chat.rs`，同名函数

### `crates/libs/agent-gateway/src/deepagents_window_tests.rs`
物理行数 112→112；保留具体宿主启动路由、租户、会话缓存/OCR/钩子、native voice 存储/交接、真实工作流运行与工具端口集成。

### `crates/libs/agent-gateway/src/handlers_tests.rs`
物理行数 1221→1221；保留具体宿主启动路由、租户、会话缓存/OCR/钩子、native voice 存储/交接、真实工作流运行与工具端口集成。
- `process_channel_messages_sends_fallback_with_thread_when_gateway_chat_fails` → 原宿主挂载同名函数；真实组合路径保留。
- `process_channel_messages_with_chat_sends_successful_reply` → 原宿主挂载同名函数；真实组合路径保留。
- `send_successful_channel_reply_sanitizes_and_omits_thread_when_disabled` → 原宿主挂载同名函数；真实组合路径保留。
- `send_successful_channel_reply_logs_send_errors_without_panicking` → 原宿主挂载同名函数；真实组合路径保留。
- `whatsapp_signature_accepts_valid_hmac` → 原宿主挂载同名函数；真实组合路径保留。
- `whatsapp_signature_rejects_wrong_signature` → 原宿主挂载同名函数；真实组合路径保留。
- `whatsapp_signature_rejects_missing_prefix` → 原宿主挂载同名函数；真实组合路径保留。
- `whatsapp_signature_rejects_valid_hex_with_wrong_length` → 原宿主挂载同名函数；真实组合路径保留。
- `whatsapp_verify_query_deserializes_fields` → 原宿主挂载同名函数；真实组合路径保留。
- `whatsapp_verify_returns_not_found_when_unconfigured` → 原宿主挂载同名函数；真实组合路径保留。
- `whatsapp_verify_echoes_challenge_for_matching_token` → 原宿主挂载同名函数；真实组合路径保留。
- `whatsapp_verify_rejects_bad_or_incomplete_verification` → 原宿主挂载同名函数；真实组合路径保留。
- `whatsapp_verify_requires_subscribe_mode_and_token` → 原宿主挂载同名函数；真实组合路径保留。
- `whatsapp_message_validates_configuration_signature_json_and_empty_payloads` → 原宿主挂载同名函数；真实组合路径保留。
- `whatsapp_message_valid_signature_does_not_hide_invalid_json` → 原宿主挂载同名函数；真实组合路径保留。
- `whatsapp_message_rejects_present_invalid_signature` → 原宿主挂载同名函数；真实组合路径保留。
- `whatsapp_message_acknowledges_nonempty_payload_after_local_send_failure` → 原宿主挂载同名函数；真实组合路径保留。
- `linq_webhook_handles_configuration_signature_json_and_empty_messages` → 原宿主挂载同名函数；真实组合路径保留。
- `linq_webhook_accepts_valid_signature_before_empty_message_ack` → 原宿主挂载同名函数；真实组合路径保留。
- `linq_webhook_rejects_present_invalid_signature` → 原宿主挂载同名函数；真实组合路径保留。
- `linq_webhook_acknowledges_unactionable_message_received_event` → 原宿主挂载同名函数；真实组合路径保留。
- `wati_verify_query_deserializes_challenge_and_defaults_missing_value` → 原宿主挂载同名函数；真实组合路径保留。
- `wati_verify_and_webhook_cover_error_and_empty_paths` → 原宿主挂载同名函数；真实组合路径保留。
- `wati_verify_echoes_empty_challenge` → 原宿主挂载同名函数；真实组合路径保留。
- `wati_webhook_processes_messages_and_handles_provider_errors` → 原宿主挂载同名函数；真实组合路径保留。
- `nextcloud_talk_webhook_handles_security_and_empty_payloads` → 原宿主挂载同名函数；真实组合路径保留。
- `nextcloud_talk_webhook_accepts_valid_signature_and_processes_messages` → 原宿主挂载同名函数；真实组合路径保留。
- `nextcloud_talk_valid_signature_does_not_hide_invalid_json` → 原宿主挂载同名函数；真实组合路径保留。
- `nextcloud_talk_webhook_rejects_present_invalid_signature` → 原宿主挂载同名函数；真实组合路径保留。
- `qq_webhook_handles_configuration_validation_and_empty_events` → 原宿主挂载同名函数；真实组合路径保留。
- `qq_webhook_accepts_matching_app_id_with_whitespace` → 原宿主挂载同名函数；真实组合路径保留。
- `qq_webhook_matching_app_id_does_not_hide_invalid_json` → 原宿主挂载同名函数；真实组合路径保留。
- `channel_reload_saved_credentials_update_existing_webhook_router` → 原宿主挂载同名函数；真实组合路径保留。
- `channel_command_cancel_webhook_uses_common_stop_and_help` → 原宿主挂载同名函数；真实组合路径保留。
- `channel_command_cancel_qq_webhook_stops_previous_message_id` → 原宿主挂载同名函数；真实组合路径保留。
- `channel_command_cancel_webhook_failed_send_preserves_state` → 原宿主挂载同名函数；真实组合路径保留。

### `crates/libs/agent-gateway/src/instance_tests.rs`
物理行数 281→281；保留具体宿主启动路由、租户、会话缓存/OCR/钩子、native voice 存储/交接、真实工作流运行与工具端口集成。
- `instance_query_deserializes_explicit_and_missing_directory` → 原宿主挂载同名函数；真实组合路径保留。
- `resolve_directory_prefers_query_over_header` → 原宿主挂载同名函数；真实组合路径保留。
- `resolve_directory_preserves_non_empty_query_value` → 原宿主挂载同名函数；真实组合路径保留。
- `resolve_directory_uses_header_when_query_is_blank` → 原宿主挂载同名函数；真实组合路径保留。
- `resolve_directory_ignores_blank_header` → 原宿主挂载同名函数；真实组合路径保留。
- `resolve_directory_ignores_invalid_header_value` → 原宿主挂载同名函数；真实组合路径保留。
- `resolve_directory_uses_current_dir_without_query_or_header` → 原宿主挂载同名函数；真实组合路径保留。
- `display_directory_falls_back_to_dot_when_current_dir_fails` → 原宿主挂载同名函数；真实组合路径保留。
- `display_directory_returns_supplied_path` → 原宿主挂载同名函数；真实组合路径保留。
- `with_instance_runs_callback_inside_requested_directory` → 原宿主挂载同名函数；真实组合路径保留。
- `with_instance_returns_callback_error` → 原宿主挂载同名函数；真实组合路径保留。
- `with_instance_ignores_legacy_invalid_project_json` → 原宿主挂载同名函数；真实组合路径保留。
- `with_instance_returns_callback_error_when_legacy_project_json_is_invalid` → 原宿主挂载同名函数；真实组合路径保留。
- `instance_init_noops_without_project_context` → 原宿主挂载同名函数；真实组合路径保留。
- `load_local_project_rejects_blank_project_id` → 原宿主挂载同名函数；真实组合路径保留。
- `load_local_project_returns_not_found_for_missing_project` → 原宿主挂载同名函数；真实组合路径保留。
- `load_local_project_returns_stored_project` → 原宿主挂载同名函数；真实组合路径保留。
- `normalize_rel_path_handles_empty_root_and_dot` → 原宿主挂载同名函数；真实组合路径保留。
- `normalize_rel_path_strips_root_from_absolute_path` → 原宿主挂载同名函数；真实组合路径保留。
- `normalize_rel_path_maps_absolute_root_to_empty_path` → 原宿主挂载同名函数；真实组合路径保留。
- `normalize_rel_path_keeps_relative_path_without_leading_slashes` → 原宿主挂载同名函数；真实组合路径保留。
- `normalize_rel_path_rejects_absolute_path_outside_root` → 原宿主挂载同名函数；真实组合路径保留。
- `normalize_rel_path_rejects_sibling_with_shared_text_prefix` → 原宿主挂载同名函数；真实组合路径保留。
- `normalize_rel_path_normalizes_backslashes_after_stripping_root` → 原宿主挂载同名函数；真实组合路径保留。

### `crates/libs/agent-gateway/src/recovery_window_tests.rs`
物理行数 168→168；保留具体宿主启动路由、租户、会话缓存/OCR/钩子、native voice 存储/交接、真实工作流运行与工具端口集成。
- `recovery_window_gateway_fixture` → 原宿主挂载同名函数；真实组合路径保留。

### `crates/libs/gateway-voice/src/host_tests/voice_realtime_http_tests.rs`
物理行数 92→92；保留具体宿主启动路由、租户、会话缓存/OCR/钩子、native voice 存储/交接、真实工作流运行与工具端口集成。
- `voice_task_10_upgrade_auth_and_tenant_desktop_isolation` → 原宿主挂载同名函数；真实组合路径保留。

### `crates/libs/gateway-voice/src/host_tests/voice_settings_tests.rs`
物理行数 50→50；保留具体宿主启动路由、租户、会话缓存/OCR/钩子、native voice 存储/交接、真实工作流运行与工具端口集成。
- `voice_task_13_gateway_patch_rejects_secrets_devices_and_invalid_values` → 原宿主挂载同名函数；真实组合路径保留。
- `voice_task_13_gateway_save_persists_language_dictionary_and_voice_without_changing_other_fields` → 原宿主挂载同名函数；真实组合路径保留。

### `crates/libs/gateway-voice/src/host_tests/voice_tests.rs`
物理行数 327→327；保留具体宿主启动路由、租户、会话缓存/OCR/钩子、native voice 存储/交接、真实工作流运行与工具端口集成。
- `voice_task_06_transcription_rejects_cross_tenant_request` → 原宿主挂载同名函数；真实组合路径保留。
- `voice_task_06_transcription_rejects_oversized_body` → 原宿主挂载同名函数；真实组合路径保留。
- `voice_task_06_transcription_counts_chunked_body_without_content_length` → 原宿主挂载同名函数；真实组合路径保留。
- `voice_task_06_transcription_rejects_anonymous_and_disabled_auth` → 原宿主挂载同名函数；真实组合路径保留。
- `voice_task_06_transcription_rejects_media_and_unknown_fields` → 原宿主挂载同名函数；真实组合路径保留。
- `voice_task_06_authenticated_foreign_session_is_not_found` → 原宿主挂载同名函数；真实组合路径保留。
- `voice_task_06_transcription_round_trip_deduplicates_and_cancels` → 原宿主挂载同名函数；真实组合路径保留。
- `voice_task_07_config_requires_owner_and_excludes_credentials` → 原宿主挂载同名函数；真实组合路径保留。

### `crates/libs/gateway-workflow/src/host_tests.rs`
物理行数 1589→977；保留具体宿主启动路由、租户、会话缓存/OCR/钩子、native voice 存储/交接、真实工作流运行与工具端口集成。
- `workflow_router_is_wired` → 原宿主挂载同名函数；真实组合路径保留。
- `workflow_chat_handlers_reject_missing_user_or_query_before_runtime` → 原宿主挂载同名函数；真实组合路径保留。
- `workflow_handlers_cover_list_and_uuid_hydration_short_circuits` → 原宿主挂载同名函数；真实组合路径保留。
- `workflow_streaming_mode_returns_sse_response_without_blocking_on_run` → 原宿主挂载同名函数；真实组合路径保留。
- `workflow_blocking_mode_reports_missing_uuid_from_store` → 原宿主挂载同名函数；真实组合路径保留。
- `workflow_runtime_and_gateway_tool_provider_execute_successful_tool_call` → 原宿主挂载同名函数；真实组合路径保留。
- `workflow_response_to_sse_response_wraps_static_events` → 原宿主挂载同名函数；真实组合路径保留。
- `workflow_handlers_keep_shared_uuid_isolated_by_request_tenant` → 原宿主挂载同名函数；真实组合路径保留。
- `workflow_uuid_hydration_is_isolated_by_tenant_and_ignores_legacy_table` → 原宿主挂载同名函数；真实组合路径保留。
- `task_pool_actions_explicit_message_requires_authorized_target` → 原宿主挂载同名函数；真实组合路径保留。
- `chat_response_maps_workflow_answer_and_conversation` → `crates/libs/gateway-workflow/src/host_migration_17_tests.rs`，同名函数
- `streaming_events_keep_conversation_id` → `crates/libs/gateway-workflow/src/host_migration_17_tests.rs`，同名函数
- `chat_request_to_workflow_request_accepts_uuid_and_strips_max_steps` → `crates/libs/gateway-workflow/src/host_migration_17_tests.rs`，同名函数
- `chat_request_to_workflow_request_rejects_legacy_input_fields` → `crates/libs/gateway-workflow/src/host_migration_17_tests.rs`，同名函数
- `chat_request_to_workflow_request_requires_uuid_or_yaml_and_defaults_steps` → `crates/libs/gateway-workflow/src/host_migration_17_tests.rs`，同名函数
- `path_workflow_uuid_is_trimmed_and_blocks_body_overrides` → `crates/libs/gateway-workflow/src/host_migration_17_tests.rs`，同名函数
- `path_workflow_uuid_rejects_legacy_input_aliases_and_allows_blank_inline_yaml` → `crates/libs/gateway-workflow/src/host_migration_17_tests.rs`，同名函数
- `optional_string_normalization_drops_blank_values` → `crates/libs/gateway-workflow/src/host_migration_17_tests.rs`，同名函数
- `runtime_inputs_default_to_restricted_context` → 原宿主挂载同名函数；真实组合路径保留。
- `workflow_answer_prefers_explicit_answer_then_known_outputs` → `crates/libs/gateway-workflow/src/host_migration_17_tests.rs`，同名函数
- `output_preview_uses_known_text_keys_before_json_fallback` → `crates/libs/gateway-workflow/src/host_migration_17_tests.rs`，同名函数
- `compact_outputs_preserve_usage_and_mark_truncation` → `crates/libs/gateway-workflow/src/host_migration_17_tests.rs`，同名函数
- `truncation_keeps_short_text_and_splits_long_text` → `crates/libs/gateway-workflow/src/host_migration_17_tests.rs`，同名函数
- `node_and_workflow_statuses_match_protocol_values` → `crates/libs/gateway-workflow/src/host_migration_17_tests.rs`，同名函数
- `node_data_maps_started_and_finished_payloads` → `crates/libs/gateway-workflow/src/host_migration_17_tests.rs`，同名函数
- `event_node_started_data_omits_runtime_inputs` → `crates/libs/gateway-workflow/src/host_migration_17_tests.rs`，同名函数
- `workflow_started_and_finished_data_match_protocol_shape` → `crates/libs/gateway-workflow/src/host_migration_17_tests.rs`，同名函数
- `workflow_sse_preserves_human_pause` → `crates/libs/gateway-workflow/src/host_migration_17_tests.rs`，同名函数
- `delta_streaming_rules_only_emit_supported_text_once` → `crates/libs/gateway-workflow/src/host_migration_17_tests.rs`，同名函数
- `text_chunk_uses_node_text_selector` → `crates/libs/gateway-workflow/src/host_migration_17_tests.rs`，同名函数
- `sse_events_include_node_lifecycle_message_and_end` → `crates/libs/gateway-workflow/src/host_migration_17_tests.rs`，同名函数
- `sse_events_skip_empty_message` → `crates/libs/gateway-workflow/src/host_migration_17_tests.rs`，同名函数
- `sse_context_ignores_node_events_until_run_starts` → `crates/libs/gateway-workflow/src/host_migration_17_tests.rs`，同名函数
- `sse_context_emits_node_started_and_finished_after_run_starts` → `crates/libs/gateway-workflow/src/host_migration_17_tests.rs`，同名函数
- `sse_context_streams_progress_delta_then_emits_distinct_finished_message` → `crates/libs/gateway-workflow/src/host_migration_17_tests.rs`，同名函数
- `sse_context_streams_delta_then_suppresses_duplicate_finished_message` → `crates/libs/gateway-workflow/src/host_migration_17_tests.rs`，同名函数
- `sse_context_streams_final_llm_then_suppresses_finished_message` → `crates/libs/gateway-workflow/src/host_migration_17_tests.rs`，同名函数
- `sse_context_finished_emits_message_when_no_text_was_streamed` → `crates/libs/gateway-workflow/src/host_migration_17_tests.rs`，同名函数
- `sse_context_error_uses_current_run_or_default_task` → `crates/libs/gateway-workflow/src/host_migration_17_tests.rs`，同名函数
- `tool_call_error_text_keeps_denied_or_failed_message_only` → 原宿主挂载同名函数；真实组合路径保留。
- `talking_head_api_blocking_returns_validated_plan` → 原宿主挂载同名函数；真实组合路径保留。
- `talking_head_api_streaming_reports_nodes_and_join` → 原宿主挂载同名函数；真实组合路径保留。
- `talking_head_api_streaming_preserves_model_error_without_fallback` → 原宿主挂载同名函数；真实组合路径保留。
- `talking_head_api_saved_sample_requires_auth_and_isolates_tenants` → 原宿主挂载同名函数；真实组合路径保留。
- `video_edit_template_uuid_hydration_reads_tenant_record_and_rejects_override` → 原宿主挂载同名函数；真实组合路径保留。

### `crates/libs/gateway-workflow/src/server.rs`
物理行数 1076→1076；保留具体宿主启动路由、租户、会话缓存/OCR/钩子、native voice 存储/交接、真实工作流运行与工具端口集成。

### `crates/libs/workflow-engine/testdata/workflow/talking_head.yaml`
物理行数 70→70；保留具体宿主启动路由、租户、会话缓存/OCR/钩子、native voice 存储/交接、真实工作流运行与工具端口集成。

## 行数与门槛
来源物理 Rust 2733→2351，净减382（14.0%），未达到70%。上表逐文件给出保留符号、行数与继续可迁项，交任务59按全局残留门槛验收，不能声称本组减量达标。跨目录测试迁移另计，不混入2739来源分母。
本轮新领域测试及夹具 1111 行；生产算法新增/迁出0行；宿主其余源码多为组合，handlers.rs 含既有 cfg(test) 消息处理夹具，未作为生产算法收益。

## 未来验证

- `python3 scripts/cargo_slot.py cargo test -p agent-gateway --lib host_migration_17_tests -- --test-threads=1`；静态预期 23 条，尚未运行。
- `python3 scripts/cargo_slot.py cargo test -p gateway-workflow --lib host_migration_17_tests -- --test-threads=1`；静态预期 28 条，尚未运行。
- 宿主保留测试采用上面完整模块/函数名称逐项过滤；迁出测试未来须通过真实宿主 list 确认消失，不以零测试结果充当通过。
- 静态核对：迁出函数名称/断言未删减，领域根模块注册，旧挂载移除，依赖无反向宿主；仅 diff whitespace 检查，不构成 Rust 编译或行为通过。
