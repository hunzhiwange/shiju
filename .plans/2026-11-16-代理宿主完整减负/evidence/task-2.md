# 任务2：agent_apps 文件族静态迁移证据

代码完成，编译/行为未验证。按用户明确覆盖，未运行任何 Cargo、构建、单元测试或实机测试。仅源码、注册、差异与测试名称守恒核对。

宿主来源 2538 → 1023 行，净减 1515 行（59.7%）。
- 生产/装配/转发：374 → 360 行。
- 测试及夹具：2164 → 663 行。
- 领域新增测试及注册：1585 行；新增真实生产算法 0 行。已有生产下沉不计收益。

## 生产与依赖方向

生产调用仍为宿主 router → gateway-agent-apps::{read,preferences,launch,lifecycle,usage_*}；access 为 app-access-service 既有所有权。OwnedOperation、resolve_operation、Sources、operation_sources、inject_sources 仅连接实际 Prompt/MCP 作业类型，保留在宿主 lifecycle.rs。领域生命周期测试直接测试领域 router，不再注入空的宿主适配。领域库无 vw-agent 反向依赖，也无 path/include 宿主源码。

新增 SDK dev 依赖 gateway-app-settings、gateway-app-lifecycle、gateway-http；gateway-endpoint 从 macOS dev 移为普通 dev，支持原有 unix 测试范围。锁文件只补本任务新依赖边。测试使用原始真实 SQLite/文件系统/PTY/HTTP 夹具及同一个 gateway-types 租户中间件，不复制宿主状态。

## 全文件、符号与用例映射

### `crates/vw-agent/src/gateway/api/handlers/agent_apps/usage_read.rs`
2 → 2 宿主行；目标 `crates/vw-agent/src/gateway/api/handlers/agent_apps/usage_read.rs`。保留实际宿主装配/公共兼容转发。

### `crates/vw-agent/src/gateway/api/handlers/agent_apps/usage_refresh.rs`
2 → 2 宿主行；目标 `crates/vw-agent/src/gateway/api/handlers/agent_apps/usage_refresh.rs`。保留实际宿主装配/公共兼容转发。

### `crates/vw-agent/src/gateway/api/handlers/agent_apps/access.rs`
2 → 2 宿主行；目标 `crates/vw-agent/src/gateway/api/handlers/agent_apps/access.rs`。保留实际宿主装配/公共兼容转发。

### `crates/vw-agent/src/gateway/api/handlers/agent_apps/preferences.rs`
2 → 2 宿主行；目标 `crates/vw-agent/src/gateway/api/handlers/agent_apps/preferences.rs`。保留实际宿主装配/公共兼容转发。

### `crates/vw-agent/src/gateway/api/handlers/agent_apps/local_access_tests.rs`
155 → 117 宿主行；目标 `crates/libs/gateway-agent-apps/src/host_migration_2_tests/local_access_tests.rs`。领域用例迁入；真实宿主注册用例保留原路径。
- `local_apps_generated_grant_uses_existing_authorization_and_rejects_other_scopes` → `crates/libs/gateway-agent-apps/src/host_migration_2_tests/local_access_tests.rs`
- `local_apps_default_router_loads_new_private_grant_without_restart_or_env` → `crates/vw-agent/src/gateway/api/handlers/agent_apps/local_access_tests.rs`
- `verify_local_auth` → `crates/vw-agent/src/gateway/api/handlers/agent_apps/local_access_tests.rs`

### `crates/vw-agent/src/gateway/api/handlers/agent_apps/apps_usage_refresh.rs`
2 → 2 宿主行；目标 `crates/vw-agent/src/gateway/api/handlers/agent_apps/apps_usage_refresh.rs`。保留实际宿主装配/公共兼容转发。

### `crates/vw-agent/src/gateway/api/handlers/agent_apps/preferences_tests.rs`
137 → 0 宿主行；目标 `crates/libs/gateway-agent-apps/src/host_migration_2_tests/preferences_tests.rs`。整组迁入；原名与断言保留。
- `fixture` → `crates/libs/gateway-agent-apps/src/host_migration_2_tests/preferences_tests.rs`
- `router` → `crates/libs/gateway-agent-apps/src/host_migration_2_tests/preferences_tests.rs`
- `call` → `crates/libs/gateway-agent-apps/src/host_migration_2_tests/preferences_tests.rs`
- `cc_apps_preferences_api_scope_persists_and_isolates` → `crates/libs/gateway-agent-apps/src/host_migration_2_tests/preferences_tests.rs`
- `cc_apps_preferences_api_scope_rejects_without_writing` → `crates/libs/gateway-agent-apps/src/host_migration_2_tests/preferences_tests.rs`

### `crates/vw-agent/src/gateway/api/handlers/agent_apps/usage_native.rs`
2 → 2 宿主行；目标 `crates/vw-agent/src/gateway/api/handlers/agent_apps/usage_native.rs`。保留实际宿主装配/公共兼容转发。

### `crates/vw-agent/src/gateway/api/handlers/agent_apps/recovery_tests.rs`
62 → 0 宿主行；目标 `crates/libs/gateway-agent-apps/src/host_migration_2_tests/recovery_tests.rs`。整组迁入；原名与断言保留。
- `cc04_recovery_executor_startup_consumes_trusted_host_policy` → `crates/libs/gateway-agent-apps/src/host_migration_2_tests/recovery_tests.rs`

### `crates/vw-agent/src/gateway/api/handlers/agent_apps/mod.rs`
236 → 226 宿主行；目标 `crates/vw-agent/src/gateway/api/handlers/agent_apps/mod.rs`。保留实际宿主装配/公共兼容转发。
- `router` → `crates/vw-agent/src/gateway/api/handlers/agent_apps/mod.rs`

### `crates/vw-agent/src/gateway/api/handlers/agent_apps/read.rs`
2 → 2 宿主行；目标 `crates/vw-agent/src/gateway/api/handlers/agent_apps/read.rs`。保留实际宿主装配/公共兼容转发。

### `crates/vw-agent/src/gateway/api/handlers/agent_apps/remote_tests_tests.rs`
248 → 0 宿主行；目标 `crates/libs/gateway-agent-apps/src/host_migration_2_tests/remote_tests_tests.rs`。整组迁入；原名与断言保留。
- `terminal` → `crates/libs/gateway-agent-apps/src/host_migration_2_tests/remote_tests_tests.rs`
- `script` → `crates/libs/gateway-agent-apps/src/host_migration_2_tests/remote_tests_tests.rs`
- `new` → `crates/libs/gateway-agent-apps/src/host_migration_2_tests/remote_tests_tests.rs`
- `drop` → `crates/libs/gateway-agent-apps/src/host_migration_2_tests/remote_tests_tests.rs`
- `new` → `crates/libs/gateway-agent-apps/src/host_migration_2_tests/remote_tests_tests.rs`
- `client` → `crates/libs/gateway-agent-apps/src/host_migration_2_tests/remote_tests_tests.rs`
- `settings_client` → `crates/libs/gateway-agent-apps/src/host_migration_2_tests/remote_tests_tests.rs`
- `action` → `crates/libs/gateway-agent-apps/src/host_migration_2_tests/remote_tests_tests.rs`
- `wait` → `crates/libs/gateway-agent-apps/src/host_migration_2_tests/remote_tests_tests.rs`
- `environment` → `crates/libs/gateway-agent-apps/src/host_migration_2_tests/remote_tests_tests.rs`
- `cc_apps_remote_host_isolation_sdk_install_launch_uninstall_and_cancel` → `crates/libs/gateway-agent-apps/src/host_migration_2_tests/remote_tests_tests.rs`

### `crates/vw-agent/src/gateway/api/handlers/agent_apps/apps_usage_script.rs`
2 → 2 宿主行；目标 `crates/vw-agent/src/gateway/api/handlers/agent_apps/apps_usage_script.rs`。保留实际宿主装配/公共兼容转发。

### `crates/vw-agent/src/gateway/api/handlers/agent_apps/usage_codex.rs`
2 → 2 宿主行；目标 `crates/vw-agent/src/gateway/api/handlers/agent_apps/usage_codex.rs`。保留实际宿主装配/公共兼容转发。

### `crates/vw-agent/src/gateway/api/handlers/agent_apps/aggregation_lifecycle_access.rs`
2 → 2 宿主行；目标 `crates/vw-agent/src/gateway/api/handlers/agent_apps/aggregation_lifecycle_access.rs`。既有未挂载两行兼容文件；本轮不计迁移收益。

### `crates/vw-agent/src/gateway/api/handlers/agent_apps/lifecycle_tests.rs`
306 → 0 宿主行；目标 `crates/libs/gateway-agent-apps/src/host_migration_2_tests/lifecycle_tests.rs`。整组迁入；原名与断言保留。
- `cc_apps_lifecycle_api_scope_missing_identity_rejected` → `crates/libs/gateway-agent-apps/src/host_migration_2_tests/lifecycle_tests.rs`
- `script` → `crates/libs/gateway-agent-apps/src/host_migration_2_tests/lifecycle_tests.rs`
- `new` → `crates/libs/gateway-agent-apps/src/host_migration_2_tests/lifecycle_tests.rs`
- `call` → `crates/libs/gateway-agent-apps/src/host_migration_2_tests/lifecycle_tests.rs`
- `preview` → `crates/libs/gateway-agent-apps/src/host_migration_2_tests/lifecycle_tests.rs`
- `submit` → `crates/libs/gateway-agent-apps/src/host_migration_2_tests/lifecycle_tests.rs`
- `wait` → `crates/libs/gateway-agent-apps/src/host_migration_2_tests/lifecycle_tests.rs`
- `environment` → `crates/libs/gateway-agent-apps/src/host_migration_2_tests/lifecycle_tests.rs`
- `cc_apps_lifecycle_api_scope_install_update_uninstall_and_tenant_isolation` → `crates/libs/gateway-agent-apps/src/host_migration_2_tests/lifecycle_tests.rs`
- `cc_apps_lifecycle_api_scope_failure_cancel_and_revocation` → `crates/libs/gateway-agent-apps/src/host_migration_2_tests/lifecycle_tests.rs`
- `cc_apps_lifecycle_api_scope_rejects_unsupported_and_changed_plan` → `crates/libs/gateway-agent-apps/src/host_migration_2_tests/lifecycle_tests.rs`
- `cc_apps_lifecycle_api_scope_read_grants_do_not_grant_writes_or_accept_paths` → `crates/libs/gateway-agent-apps/src/host_migration_2_tests/lifecycle_tests.rs`

### `crates/vw-agent/src/gateway/api/handlers/agent_apps/lifecycle.rs`
108 → 108 宿主行；目标 `crates/vw-agent/src/gateway/api/handlers/agent_apps/lifecycle.rs`。保留实际宿主装配/公共兼容转发。
- `operation` → `crates/vw-agent/src/gateway/api/handlers/agent_apps/lifecycle.rs`
- `response` → `crates/vw-agent/src/gateway/api/handlers/agent_apps/lifecycle.rs`
- `resolve_operation` → `crates/vw-agent/src/gateway/api/handlers/agent_apps/lifecycle.rs`
- `resolve` → `crates/vw-agent/src/gateway/api/handlers/agent_apps/lifecycle.rs`
- `operation_sources` → `crates/vw-agent/src/gateway/api/handlers/agent_apps/lifecycle.rs`
- `inject_sources` → `crates/vw-agent/src/gateway/api/handlers/agent_apps/lifecycle.rs`
- `router` → `crates/vw-agent/src/gateway/api/handlers/agent_apps/lifecycle.rs`
- `router_with_policy` → `crates/vw-agent/src/gateway/api/handlers/agent_apps/lifecycle.rs`

### `crates/vw-agent/src/gateway/api/handlers/agent_apps/read_tests.rs`
364 → 97 宿主行；目标 `crates/libs/gateway-agent-apps/src/host_migration_2_tests/read_tests.rs`。领域用例迁入；真实宿主注册用例保留原路径。
- `router` → `crates/libs/gateway-agent-apps/src/host_migration_2_tests/read_tests.rs`
- `request` → `crates/libs/gateway-agent-apps/src/host_migration_2_tests/read_tests.rs`
- `policy` → `crates/libs/gateway-agent-apps/src/host_migration_2_tests/read_tests.rs`
- `executable` → `crates/libs/gateway-agent-apps/src/host_migration_2_tests/read_tests.rs`
- `cc_apps_read_api_authorization_success_and_real_probe` → `crates/libs/gateway-agent-apps/src/host_migration_2_tests/read_tests.rs`
- `cc_apps_read_api_authorization_denials_never_run_probe` → `crates/libs/gateway-agent-apps/src/host_migration_2_tests/read_tests.rs`
- `cc_apps_read_api_authorization_revocation_and_invalid_files_fail_closed` → `crates/libs/gateway-agent-apps/src/host_migration_2_tests/read_tests.rs`
- `cc_apps_read_api_authorization_tenant_query_and_token_boundaries` → `crates/libs/gateway-agent-apps/src/host_migration_2_tests/read_tests.rs`
- `cc_apps_read_api_authorization_reports_actual_failure_and_desktop_presence` → `crates/libs/gateway-agent-apps/src/host_migration_2_tests/read_tests.rs`
- `cc_apps_read_api_authorization_production_registration_and_default_deny` → `crates/vw-agent/src/gateway/api/handlers/agent_apps/read_tests.rs`
- `cc_apps_read_api_authorization_invalid_registration_never_probes` → `crates/libs/gateway-agent-apps/src/host_migration_2_tests/read_tests.rs`
- `cc_apps_read_api_authorization_managed_package_source_matches_lifecycle` → `crates/libs/gateway-agent-apps/src/host_migration_2_tests/read_tests.rs`

### `crates/vw-agent/src/gateway/api/handlers/agent_apps/launch.rs`
10 → 6 宿主行；目标 `crates/vw-agent/src/gateway/api/handlers/agent_apps/launch.rs`。保留实际宿主装配/公共兼容转发。

### `crates/vw-agent/src/gateway/api/handlers/agent_apps/import_tests.rs`
39 → 0 宿主行；目标 `crates/libs/gateway-agent-apps/src/host_migration_2_tests/import_tests.rs`。整组迁入；原名与断言保留。
- `cc04_import_startup_consumes_only_authorized_empty_catalogs` → `crates/libs/gateway-agent-apps/src/host_migration_2_tests/import_tests.rs`

### `crates/vw-agent/src/gateway/api/handlers/agent_apps/launch_tests.rs`
404 → 0 宿主行；目标 `crates/libs/gateway-agent-apps/src/host_migration_2_tests/launch_tests.rs`。整组迁入；原名与断言保留。
- `terminal` → `crates/libs/gateway-agent-apps/src/host_migration_2_tests/launch_tests.rs`
- `failing_terminal` → `crates/libs/gateway-agent-apps/src/host_migration_2_tests/launch_tests.rs`
- `call` → `crates/libs/gateway-agent-apps/src/host_migration_2_tests/launch_tests.rs`
- `cc_apps_launch_api_target_scope_accepts_and_rejects_before_terminal` → `crates/libs/gateway-agent-apps/src/host_migration_2_tests/launch_tests.rs`
- `attached_terminal` → `crates/libs/gateway-agent-apps/src/host_migration_2_tests/launch_tests.rs`
- `cc_apps_launch_identity_list_sdk_http_tty_lifetime_and_account_rotation` → `crates/libs/gateway-agent-apps/src/host_migration_2_tests/launch_tests.rs`
- `cc_apps_launch_native_terminal_sdk_http_lifetime_and_rotation` → `crates/libs/gateway-agent-apps/src/host_migration_2_tests/launch_tests.rs`
- `identity_roundtrip` → `crates/libs/gateway-agent-apps/src/host_migration_2_tests/launch_tests.rs`

### `crates/vw-agent/src/gateway/api/handlers/agent_apps_acceptance_tests.rs`
361 → 361 宿主行；目标 `crates/vw-agent/src/gateway/api/handlers/agent_apps_acceptance_tests.rs`。保留真实 build_router、PairingGuard、Bearer 解析及断连/取消/PostgreSQL 验收；移至领域将失去真实装配边界。
- `roundtrip` → `crates/vw-agent/src/gateway/api/handlers/agent_apps_acceptance_tests.rs`
- `rejection` → `crates/vw-agent/src/gateway/api/handlers/agent_apps_acceptance_tests.rs`
- `disconnect` → `crates/vw-agent/src/gateway/api/handlers/agent_apps_acceptance_tests.rs`
- `drop` → `crates/vw-agent/src/gateway/api/handlers/agent_apps_acceptance_tests.rs`
- `start` → `crates/vw-agent/src/gateway/api/handlers/agent_apps_acceptance_tests.rs`
- `url` → `crates/vw-agent/src/gateway/api/handlers/agent_apps_acceptance_tests.rs`
- `client` → `crates/vw-agent/src/gateway/api/handlers/agent_apps_acceptance_tests.rs`
- `stop` → `crates/vw-agent/src/gateway/api/handlers/agent_apps_acceptance_tests.rs`
- `drop` → `crates/vw-agent/src/gateway/api/handlers/agent_apps_acceptance_tests.rs`
- 宏展开 6 条 cc_base_icon_headless_{sqlite,postgres}_{roundtrip,rejection_and_scope,disconnect_recovery} 保持原挂载。

### `crates/vw-agent/src/gateway/api/handlers/agent_apps_read_tests.rs`
88 → 88 宿主行；目标 `crates/vw-agent/src/gateway/api/handlers/agent_apps_read_tests.rs`。保留 handler_router、Basic 认证与租户头拒绝的真实宿主验收（含真实存储范围断言）。
- `router` → `crates/vw-agent/src/gateway/api/handlers/agent_apps_read_tests.rs`
- `request` → `crates/vw-agent/src/gateway/api/handlers/agent_apps_read_tests.rs`
- `cc_base_icon_api_read_scoped_sqlite` → `crates/vw-agent/src/gateway/api/handlers/agent_apps_read_tests.rs`
- `cc_base_icon_api_read_scoped_postgres` → `crates/vw-agent/src/gateway/api/handlers/agent_apps_read_tests.rs`
- `check_scopes` → `crates/vw-agent/src/gateway/api/handlers/agent_apps_read_tests.rs`
- `cc_base_icon_api_read_rejects_auth_and_tenant_headers` → `crates/vw-agent/src/gateway/api/handlers/agent_apps_read_tests.rs`

## 减量门槛与继续可迁项

本组净减不足 70%，供收口验收，未宣称达标。逐文件保留原因见上：acceptance 361 行及 agent_apps_read_tests 88 行保留真实全路由/认证边界；local_access_tests 保留默认策略热加载、真实 auth_read 和远端/转发头拒绝；read_tests 保留生产注册子进程与其真实文件探针夹具；lifecycle 108 行保留实际外部作业注册表适配；mod.rs 保留全部领域路由装配和相邻所有者的跨目录挂载；其余每个 2 行入口保留现有调用者兼容。launch.rs 另保留任务1标注的 app_providers/agy_tests 跨目录挂载（其所有者负责），不越界删除。继续可迁项为图标验收内存储领域断言和共享 HTTP 夹具，可由收口与真实网关验收共同拆分，不能简单替换 build_router 为领域路由冒充覆盖。

## 未来验证（本轮均未运行）

`python3 scripts/cargo_slot.py cargo test -p gateway-agent-apps --lib host_migration_2_tests -- --test-threads=1`：macOS 静态预期 21 条，其他 unix 少 3 条（原生终端、导入、恢复）。

宿主使用原全名逐条过滤：
- `python3 scripts/cargo_slot.py cargo test -p vw-agent --lib local_apps_default_router_loads_new_private_grant_without_restart_or_env -- --test-threads=1`，预期 1 条。
- `python3 scripts/cargo_slot.py cargo test -p vw-agent --lib cc_apps_read_api_authorization_production_registration_and_default_deny -- --test-threads=1`，预期 1 条。
- `python3 scripts/cargo_slot.py cargo test -p vw-agent --lib cc_base_icon_api_read_scoped_sqlite -- --test-threads=1`，预期 1 条。
- `python3 scripts/cargo_slot.py cargo test -p vw-agent --lib cc_base_icon_api_read_scoped_postgres -- --test-threads=1`，预期 1 条。
- `python3 scripts/cargo_slot.py cargo test -p vw-agent --lib cc_base_icon_api_read_rejects_auth_and_tenant_headers -- --test-threads=1`，预期 1 条。
- 图标宏展开验收逐条使用上列 6 个真实名称过滤，预期各 1 条；PostgreSQL 需真实服务，不允许跳过冒充通过。
- 未来用宿主测试 list 核实迁出原名称消失；本轮名称守恒静态核对覆盖所有显式测试，断言无删除。无退出码、通过数或编译性能结论。
