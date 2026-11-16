# 任务56：提供者完整领域迁移证据

代码完成，编译/行为未验证。遵守公共计划，本次未运行任何 Cargo、构建、单元测试或实机检查；只进行源码、注册、路径、依赖和差异静态核对。

## 生产与边界

原生产算法早已在 agent-provider，不计本次成果。本次真实迁移 model_catalog::read 中的绑定解析、provider 身份校验、保留 ID/endpoint/adapter 拒绝策略和目录组装到 agent_provider::model_catalog::read_with；宿主 read 仍被 ensure_host 的 read_catalogs 闭包调用，并实际调用 read_with。宿主保留当前租户、隔离线程、真实数据库打开与 AppProviderStore::preview_record/app_models::edits::read 端口。先校验所有绑定再打开数据库；无绑定不打开数据库。没有放宽权限或重写目录投影。

依赖方向：vw-agent → agent-provider → app-provider-validation；领域测试 dev 依赖 agent-database、agent-paths、app-provider-services、app-models 及 API 类型，无反向依赖 vw-agent、无 path/include 宿主 Rust。lib.rs 自注册 host_migration_56_tests，旧挂载只留下下列四个宿主用例。cfg(test) 同时启用原 test-support 配置读取钩子，未来默认 lib 测试无需外部 feature。

## 全文件行数

按 splitlines；宿主测试文件全计测试，适配器内 cfg(test) 另计测试。

|文件|前|后|保留职责|
|---|---:|---:|---|
|auth.rs|1|1|认证公共兼容导出|
|mod.rs|40|40|公共 provider/error/models/voice 命名空间及宿主测试注册|
|model_catalog.rs|68|27|真实租户、数据库和隔离线程端口|
|model_catalog_tests.rs|452|270|网关 picker 租户鉴权与执行前能力拒绝，两条完整真实端到端用例及其 SQLite 夹具|
|provider.rs|59|55|ensure_host 与需安装端口的公共操作包装；cfg(test) 入口供宿主薄用例|
|provider_tests.rs|577|143|LLM 请求限制接线与工作线程租户恢复，两条完整用例及环境夹具|
|transform.rs|1|1|transform 公共兼容导出|

宿主本组 1198 → 537，净减 661（55.2%），未达到70%。未删除断言凑比例。剩余主体是四条真实宿主跨系统验收与所需夹具；不能用领域 mock 替代。可继续迁移项：EnvGuard 与 SQLite fixture 若以后存在第三个真实调用者可按三次原则提取独立 test-support；本次不为行数暴露领域测试私有模块。mod/auth/transform 保持公共消费者兼容；provider 包装负责安装端口，不能直接改成无初始化导出。减量门槛留收口验收，不宣称整轮达标。

## 全用例映射

- `provider_tests.rs::agent_provider_without_models_is_kept_for_settings_and_dropped_at_runtime` → `crates/libs/agent-provider/src/host_migration_56_tests/provider.rs::agent_provider_without_models_is_kept_for_settings_and_dropped_at_runtime`（完整函数及断言保留）。
- `provider_tests.rs::agent_provider_state_includes_remote_models` → `crates/libs/agent-provider/src/host_migration_56_tests/provider.rs::agent_provider_state_includes_remote_models`（完整函数及断言保留）。
- `provider_tests.rs::agent_provider_new_model_inherits_unique_builtin_api` → `crates/libs/agent-provider/src/host_migration_56_tests/provider.rs::agent_provider_new_model_inherits_unique_builtin_api`（完整函数及断言保留）。
- `provider_tests.rs::agent_provider_unknown_model_uses_shared_limit_and_capability_defaults` → `crates/libs/agent-provider/src/host_migration_56_tests/provider.rs::agent_provider_unknown_model_uses_shared_limit_and_capability_defaults`（完整函数及断言保留）。
- `provider_tests.rs::agent_cross_layer_fixture_uses_the_same_default_limit` → `crates/libs/agent-provider/src/host_migration_56_tests/provider.rs::agent_cross_layer_fixture_uses_the_same_default_limit`（完整函数及断言保留）。
- `provider_tests.rs::agent_cross_layer_explicit_model_reaches_the_next_request_with_metadata` → `crates/vw-agent/src/provider/provider_tests.rs::agent_cross_layer_explicit_model_reaches_the_next_request_with_metadata`（完整函数及断言保留）。
- `provider_tests.rs::agent_provider_explicit_model_capabilities_match_resolver_contract` → `crates/libs/agent-provider/src/host_migration_56_tests/provider.rs::agent_provider_explicit_model_capabilities_match_resolver_contract`（完整函数及断言保留）。
- `provider_tests.rs::agent_custom_provider_reasoning_inherits_catalog_for_shiju_and_token_plan` → `crates/libs/agent-provider/src/host_migration_56_tests/provider.rs::agent_custom_provider_reasoning_inherits_catalog_for_shiju_and_token_plan`（完整函数及断言保留）。
- `provider_tests.rs::provider_catalog_read_failure_preserves_config_error_and_recovers` → `crates/libs/agent-provider/src/host_migration_56_tests/provider.rs::provider_catalog_read_failure_preserves_config_error_and_recovers`（完整函数及断言保留）。
- `provider_tests.rs::provider_catalog_interleaved_config_uses_one_snapshot` → `crates/libs/agent-provider/src/host_migration_56_tests/provider.rs::provider_catalog_interleaved_config_uses_one_snapshot`（完整函数及断言保留）。
- `provider_tests.rs::provider_catalog_disable_and_delete_revoke_cached_models` → `crates/libs/agent-provider/src/host_migration_56_tests/provider.rs::provider_catalog_disable_and_delete_revoke_cached_models`（完整函数及断言保留）。
- `provider_tests.rs::provider_catalog_same_path_tenants_stay_isolated_on_worker` → `crates/vw-agent/src/provider/provider_tests.rs::provider_catalog_same_path_tenants_stay_isolated_on_worker`（完整函数及断言保留）。
- `model_catalog_tests.rs::catalog_resolver_projection_keeps_selected_deployment` → `crates/libs/agent-provider/src/host_migration_56_tests/catalog.rs::catalog_resolver_projection_keeps_selected_deployment`（完整函数及断言保留）。
- `model_catalog_tests.rs::catalog_resolver_projection_respects_native_default` → `crates/libs/agent-provider/src/host_migration_56_tests/catalog.rs::catalog_resolver_projection_respects_native_default`（完整函数及断言保留）。
- `model_catalog_tests.rs::catalog_resolver_projection_uses_only_exact_host_and_endpoint` → `crates/libs/agent-provider/src/host_migration_56_tests/catalog.rs::catalog_resolver_projection_uses_only_exact_host_and_endpoint`（完整函数及断言保留）。
- `model_catalog_tests.rs::catalog_resolver_projection_refreshes_after_commit_without_reconfiguring` → `crates/libs/agent-provider/src/host_migration_56_tests/catalog.rs::catalog_resolver_projection_refreshes_after_commit_without_reconfiguring`（完整函数及断言保留）。
- `model_catalog_tests.rs::catalog_resolver_projection_preserves_declared_metadata` → `crates/libs/agent-provider/src/host_migration_56_tests/catalog.rs::catalog_resolver_projection_preserves_declared_metadata`（完整函数及断言保留）。
- `model_catalog_tests.rs::catalog_resolver_projection_keeps_gateway_picker_in_tenant` → `crates/vw-agent/src/provider/model_catalog_tests.rs::catalog_resolver_projection_keeps_gateway_picker_in_tenant`（完整函数及断言保留）。
- `model_catalog_tests.rs::catalog_execution_gate_rejects_gateway_image_before_ocr` → `crates/vw-agent/src/provider/model_catalog_tests.rs::catalog_execution_gate_rejects_gateway_image_before_ocr`（完整函数及断言保留）。

夹具映射：EnvGuard 领域/宿主各保留直接使用的小型环境恢复器；Fixture::new/with_endpoint/store/bind/edit/select 领域与网关分别使用真实 SQLite；run_isolated 分别在各自测试模块中按 module_path 精确子进程执行，隔离 HOME/OnceLock/环境快照。领域目录测试 task-local 仅注入 Host 的当前租户端口，不模拟存储、路由或投影算法。任务1 external-mounts 未发现本组另外的跨目录挂载。

## 未来验证

- `python3 scripts/cargo_slot.py cargo test -p agent-provider --lib host_migration_56_tests -- --test-threads=1`：预期15条（provider 10，catalog 5），必须非零且失败数0；catalog 每条子进程另断言1 passed。
- 宿主未来分别以 `provider::provider::provider_tests`（2条）及 `provider::model_catalog_tests`（2条）过滤 `cargo test -p vw-agent --lib`，同样经 cargo_slot 入口、串行。上面逐条映射的四个保留完整名称可用 --exact 单独验证。
- 用宿主 `--list` 核对上面15条迁出测试不再挂载；零条只能证明消失，不能作为行为通过。所有未来命令本次均未运行。

## 生产/测试分项

- `crates/vw-agent/src/provider/auth.rs`：生产/文档/注册（含原有 cfg(test) 入口） 1 → 1 行。
- `crates/vw-agent/src/provider/mod.rs`：生产/文档/注册（含原有 cfg(test) 入口） 40 → 40 行。
- `crates/vw-agent/src/provider/model_catalog.rs`：生产/文档/注册（含原有 cfg(test) 入口） 68 → 27 行。
- `crates/vw-agent/src/provider/model_catalog_tests.rs`：测试 452 → 270 行。
- `crates/vw-agent/src/provider/provider.rs`：生产/文档/注册（含原有 cfg(test) 入口） 59 → 55 行。
- `crates/vw-agent/src/provider/provider_tests.rs`：测试 577 → 143 行。
- `crates/vw-agent/src/provider/transform.rs`：生产/文档/注册（含原有 cfg(test) 入口） 1 → 1 行。
- `crates/libs/agent-provider/src/model_catalog.rs`：生产/文档/注册（含原有 cfg(test) 入口） 144 → 179 行。
- `crates/libs/agent-provider/src/provider.rs`：生产/文档/注册（含原有 cfg(test) 入口） 816 → 816 行。
- `crates/libs/agent-provider/src/lib.rs`：生产/文档/注册（含原有 cfg(test) 入口） 12 → 15 行。
- `crates/libs/agent-provider/src/host_migration_56_tests.rs`：测试 0 → 32 行。
- `crates/libs/agent-provider/src/host_migration_56_tests/provider.rs`：测试 0 → 485 行。
- `crates/libs/agent-provider/src/host_migration_56_tests/catalog.rs`：测试 0 → 344 行。

静态核对：19条原用例名称唯一映射，迁移前后断言数量一致（目录隔离运行器因分属两个目标而各保留一条子进程断言）；include_str 资源路径存在；git diff --check 通过。此为静态证据，不代表编译或行为通过。
