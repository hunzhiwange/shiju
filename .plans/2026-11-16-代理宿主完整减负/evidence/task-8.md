# 任务8静态交付证据

代码完成，编译/行为未验证。按公共计划禁止运行任何 Cargo、构建、测试、实机验证；本轮仅源码与差异核对，不宣称测试通过或构建耗时收益。

## 职责与真实入口

生产逻辑在本轮前已属于 gateway-config::handlers，本轮没有新增生产迁移收益。宿主 ConfigHost 保留具体 AppState 的 config/pairing、真实 require_auth、密钥 mask_sensitive_fields/hydrate_config_for_save 的领域接线、resolve_directory/with_instance 的实例生命周期、list_providers 的目录实例上下文、build_hook_candidate/publish_hook_candidate 与 invalidate_session_query_engines。router/stateful_router 与 handle_api_config_get/put 为实际宿主路由装配入口；config_patch_stateful/config_providers 仅供保留的边界测试。删除无调用的 config_get/config_patch/config_get_stateful/apply_config_patch 测试转发。

接收库通过 lib.rs 自注册 host_migration_8_tests，四个行为子模块完整保留原测试函数与断言，调用本库真实 handlers；不反向依赖 vw-agent，也不通过 path/include 加载宿主源码。dev 依赖 agent-database、agent-gateway（真实密钥及目录处理）、config-agent-profile、config-gateway，Cargo.lock 只补对应 gateway-config 条目。真实 SQLite 与配置领域入口继续承担租户/项目持久化。最小 TestState 只有配置和配对守卫，未克隆 AppState；真实认证仍由宿主验证；无实例依赖的领域操作端口直接执行回调，hook/session 空端口不冒充真实生命周期验收。所有迁入异步用例共用本模块环境锁，保留 EnvGuard 恢复环境变量。

原 config_tests 挂载保留且只包含真实宿主用例；迁出的每条用例在宿主文件中消失，接收库自行编译。已读取任务1 evidence，external-mounts.json 未列出本文件族额外挂载。

## 全文件行数（splitlines）

- `crates/vw-agent/src/gateway/api/handlers/config.rs`：147 → 109 行。
- `crates/vw-agent/src/gateway/api/handlers/config_tests.rs`：1619 → 498 行。

宿主合计 1766 → 607，净减 1159（65.6%）。config.rs 的生产部分未变，仅清理测试转发；config_tests.rs 全部为测试与夹具。
- `crates/libs/gateway-config/src/host_migration_8_tests.rs`：新增 105 行，全部测试/夹具，无生产算法。
- `crates/libs/gateway-config/src/host_migration_8_tests/patch.rs`：新增 345 行，全部测试/夹具，无生产算法。
- `crates/libs/gateway-config/src/host_migration_8_tests/profiles.rs`：新增 217 行，全部测试/夹具，无生产算法。
- `crates/libs/gateway-config/src/host_migration_8_tests/read.rs`：新增 243 行，全部测试/夹具，无生产算法。

不足70%的逐文件原因：config.rs 保留真实宿主端口与路由装配；config_tests.rs 保留实际认证、provider 目录上下文、真实 hook 构建/发布/失败原子性、查询引擎失效及实例重启后的 live agent 解析。继续可迁项：在对应实例、聊天和 hook 领域的真实装配端口稳定后，进一步拆分四条跨子系统生命周期用例的领域断言；不能用本任务的空端口替代安全边界。由收口任务复核减量门槛，不将本任务静态完成视为整体达标。

## 全用例映射（保留原名称）

- `routers_build_for_stateless_and_stateful_config_routes` → `宿主 config::config_tests::routers_build_for_stateless_and_stateful_config_routes`
- `config_get_returns_serialized_instance_config` → `gateway_config::host_migration_8_tests::read::config_get_returns_serialized_instance_config`
- `config_get_stateful_returns_serialized_instance_config` → `gateway_config::host_migration_8_tests::read::config_get_stateful_returns_serialized_instance_config`
- `config_get_without_directory_restores_the_requested_tenant_after_restart` → `gateway_config::host_migration_8_tests::read::config_get_without_directory_restores_the_requested_tenant_after_restart`
- `config_patch_updates_global_config_without_explicit_directory` → `gateway_config::host_migration_8_tests::patch::config_patch_updates_global_config_without_explicit_directory`
- `config_patch_without_directory_updates_only_the_requested_tenant` → `gateway_config::host_migration_8_tests::patch::config_patch_without_directory_updates_only_the_requested_tenant`
- `config_patch_uses_explicit_header_for_project_override` → `gateway_config::host_migration_8_tests::patch::config_patch_uses_explicit_header_for_project_override`
- `config_patch_stateful_updates_active_runtime_config_without_explicit_directory` → `gateway_config::host_migration_8_tests::patch::config_patch_stateful_updates_active_runtime_config_without_explicit_directory`
- `config_patch_stateful_leaves_runtime_config_when_directory_is_explicit` → `gateway_config::host_migration_8_tests::patch::config_patch_stateful_leaves_runtime_config_when_directory_is_explicit`
- `config_patch_stateful_rejects_invalid_global_patch_without_mutating_runtime_config` → `gateway_config::host_migration_8_tests::patch::config_patch_stateful_rejects_invalid_global_patch_without_mutating_runtime_config`
- `project_config_patch_writes_only_database_override` → `gateway_config::host_migration_8_tests::patch::project_config_patch_writes_only_database_override`
- `project_config_patch_isolates_requested_tenant` → `gateway_config::host_migration_8_tests::patch::project_config_patch_isolates_requested_tenant`
- `project_config_patch_never_persists_effective_base_fields` → `gateway_config::host_migration_8_tests::patch::project_config_patch_never_persists_effective_base_fields`
- `config_providers_returns_defaults_for_available_models` → `宿主 config::config_tests::config_providers_returns_defaults_for_available_models`
- `handle_api_config_get_requires_auth_when_pairing_is_enabled` → `宿主 config::config_tests::handle_api_config_get_requires_auth_when_pairing_is_enabled`
- `handle_api_config_get_returns_pretty_toml_with_masked_secrets` → `gateway_config::host_migration_8_tests::read::handle_api_config_get_returns_pretty_toml_with_masked_secrets`
- `handle_api_config_get_reports_gateway_skey_store_failure` → `gateway_config::host_migration_8_tests::read::handle_api_config_get_reports_gateway_skey_store_failure`
- `handle_api_config_put_rejects_unauthorized_before_parsing_body` → `宿主 config::config_tests::handle_api_config_put_rejects_unauthorized_before_parsing_body`
- `handle_api_config_put_reports_gateway_skey_store_failure_without_mutating_runtime_state` → `gateway_config::host_migration_8_tests::read::handle_api_config_put_reports_gateway_skey_store_failure_without_mutating_runtime_state`
- `handle_api_config_put_reports_config_save_failure_without_mutating_runtime_state` → `宿主 config::config_tests::handle_api_config_put_reports_config_save_failure_without_mutating_runtime_state`
- `config_put_reloads_hooks_and_invalidates_query_engines` → `宿主 config::config_tests::config_put_reloads_hooks_and_invalidates_query_engines`
- `config_put_hook_build_failure_is_atomic` → `宿主 config::config_tests::config_put_hook_build_failure_is_atomic`
- `handle_api_config_put_hydrates_validates_saves_and_updates_runtime_config` → `gateway_config::host_migration_8_tests::read::handle_api_config_put_hydrates_validates_saves_and_updates_runtime_config`
- `dashboard_config_skeys_are_isolated_by_tenant_scope` → `gateway_config::host_migration_8_tests::read::dashboard_config_skeys_are_isolated_by_tenant_scope`
- `handle_api_config_get_accepts_valid_bearer_token_when_pairing_is_enabled` → `宿主 config::config_tests::handle_api_config_get_accepts_valid_bearer_token_when_pairing_is_enabled`
- `config_patch_stateful_rejects_invalid_project_patch_without_mutating_runtime_config` → `gateway_config::host_migration_8_tests::patch::config_patch_stateful_rejects_invalid_project_patch_without_mutating_runtime_config`
- `manual_agent_save_updates_live_resolution_and_survives_instance_restart` → `宿主 config::config_tests::manual_agent_save_updates_live_resolution_and_survives_instance_restart`
- `agent_profile_api_rejects_builtin_delete_and_backend_change` → `gateway_config::host_migration_8_tests::profiles::agent_profile_api_rejects_builtin_delete_and_backend_change`
- `agent_profile_api_persists_changes_after_reload` → `gateway_config::host_migration_8_tests::profiles::agent_profile_api_persists_changes_after_reload`
- `agent_profile_api_rejects_invalid_identity_and_model_conflicts` → `gateway_config::host_migration_8_tests::profiles::agent_profile_api_rejects_invalid_identity_and_model_conflicts`
- `agent_profile_api_native_metadata_and_empty_rules_are_editable` → `gateway_config::host_migration_8_tests::profiles::agent_profile_api_native_metadata_and_empty_rules_are_editable`
- `agent_profile_api_full_replacement_cannot_remove_saved_builtin` → `gateway_config::host_migration_8_tests::profiles::agent_profile_api_full_replacement_cannot_remove_saved_builtin`
- `agent_runtime_lock_patch_preserves_saved_engines_and_is_atomic` → `gateway_config::host_migration_8_tests::runtime::agent_runtime_lock_patch_preserves_saved_engines_and_is_atomic`
- `agent_runtime_lock_patch_is_scoped_to_each_tenant` → `gateway_config::host_migration_8_tests::runtime::agent_runtime_lock_patch_is_scoped_to_each_tenant`
- `agent_runtime_lock_put_rejects_replacement_and_implicit_switches` → `gateway_config::host_migration_8_tests::runtime::agent_runtime_lock_put_rejects_replacement_and_implicit_switches`
- `agent_runtime_lock_put_accepts_first_save_and_locks_native` → `gateway_config::host_migration_8_tests::runtime::agent_runtime_lock_put_accepts_first_save_and_locks_native`
- `worktree_settings_patch_round_trips_and_rejects_project_override` → `gateway_config::host_migration_8_tests::runtime::worktree_settings_patch_round_trips_and_rejects_project_override`

## 未来验证

- `python3 scripts/cargo_slot.py cargo test -p gateway-config --lib host_migration_8_tests -- --test-threads=1`，预期28条，必须非零且全部通过。
- `python3 scripts/cargo_slot.py cargo test -p vw-agent --lib gateway::api::handlers::config::config_tests -- --test-threads=1`，预期9条真实宿主用例（使用 --list 确认完整模块路径后精确过滤）。
- 未来宿主 `-- --list` 核实上述28条迁入用例已消失；不得以零测试当通过。

当前全部编译/行为检查未运行。静态核对37个原名称一一映射、28迁出/9保留，无丢失或重名；生产路由调用仍使用同一 ConfigHost，接收库注册和依赖显式存在。补充原宿主测试缺失的 with_instance 导入，使保留的实例生命周期用例显式使用真实入口。

- `crates/libs/gateway-config/src/host_migration_8_tests/runtime.rs`：新增 323 行，全部测试。


## 专项残留复审补充（取代上文28/9数量）

复审进一步迁出3条鉴权用例至 `gateway_config::host_migration_8_tests::auth`：
- `handle_api_config_get_requires_auth_when_pairing_is_enabled`
- `handle_api_config_put_rejects_unauthorized_before_parsing_body`
- `handle_api_config_get_accepts_valid_bearer_token_when_pairing_is_enabled`

本次没有模拟认证。将 agent-gateway::api::auth 原真实策略提取为 `require_pairing_auth(&PairingGuard, &HeaderMap)`；原 `require_auth` 与宿主 ConfigHost、领域 TestHost 均调用该同一生产入口，校验/错误内容保持原样。直接复用真实 PairingGuard，未复制 AppState。现有 gateway-pairing 生产依赖已存在，无需新依赖或 Cargo.lock 改动。

原三条用例全部原断言随迁；宿主新增薄用例 `config_host_enforces_pairing_before_parsing_put` 验证实际 ConfigHost 和真实 PUT 接线，保持先认证后解析的安全边界。现接收库31条，宿主7条（6条原用例+1条新契约），原37条名称均保留且唯一。静态比较本次原9个名称全部存在，断言数未减少；新增2条宿主契约断言。

未来领域过滤 `host_migration_8_tests` 预期31条；宿主 `gateway::api::handlers::config::config_tests` 预期7条。补充 `python3 scripts/cargo_slot.py cargo test -p agent-gateway --lib api::auth::auth_tests -- --test-threads=1`，以 --list 确认实际非零数量。全部命令仍未运行，编译/行为未验证。

继续保留的领域/宿主边界：router 双形态注册；provider 目录上下文真实装配；三条 HookRuntime 和聊天缓存的发布/失败原子性；manual_agent_save 的实际 live agent/provider 解析与实例销毁重启。后四条需要真实宿主依赖，不能移到空端口 fixture 冒充同等覆盖；复审没有复制这些状态或删断言。

复审后 `crates/vw-agent/src/gateway/api/handlers/config.rs`：109 行。

复审后 `crates/vw-agent/src/gateway/api/handlers/config_tests.rs`：455 行。

复审后 `crates/libs/gateway-config/src/host_migration_8_tests/auth.rs`：55 行。

宿主文件族最终 564 行，相对原1766行净减1202（68.1%），仍低于70%，原因如上。
