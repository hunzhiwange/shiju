# 任务53 静态迁移证据

代码完成，编译/行为未验证。本次遵守计划禁止任何 Cargo、构建、单元/实机执行；以下数量均为源码静态计数。

## 真实生产接线

- NativeTurnPersistence::persist_turn → agent_acp::persistence::persist_turn → agent_project::instance / session_store::ui_store / session_engine::message / ordered_id。完整保留历史锁、四次写入顺序、时间、token 饱和加法、错误及取消终态。
- 宿主 prepare_session_tools → agent_acp::mcp::validate_server_names → supported_server_name；验证先于连接创建，错误文本与传输匹配不变。
- 新增正常依赖及锁文件条目，session-store 从 native 条件依赖提升为正常依赖；静态遍历 280 个可达清单，未发现反向 agent-acp 边。未用 Cargo 验证解析。
- lib.rs 自注册 persistence/mcp 与 host_migration_53_tests；无 path/include 回挂宿主、无 vw-agent 反向依赖。原 backend 的事件映射重导出及原测试函数删除。

## 全文件与保留职责

| 文件 | 前行 | 后行 | 实际归属、保留原因 |
|---|---:|---:|---|
| mcp_tests.rs | 152 | 152 | 真实 ToolRuntimeContext 目录隔离、进程启动、环境脱敏、原子失败与外租户禁止启动。 |
| mounts.rs | 103 | 103 | prepare/prepare_revision 连接宿主项目配置、原生技能发现、安全策略与 MCP 连接；checkpoint_revision 从真实项目上下文推导 scope。 |
| model_tests.rs | 83 | 83 | 配置持久化、禁用模型、跨租户隔离及真实 Request.model 接线。 |
| mount_acceptance_tests.rs | 98 | 98 | 生产 CLI stdio 实机入口及独立进程租户/同目录隔离，不能用模拟 Host 代替。 |
| mount_update_tests.rs | 238 | 238 | 真实 MCP 回滚/退出/撤销、修订并发及协议忙状态阻止原生操作。 |
| backend.rs | 300 | 168 | NativeAcpHost 的 create_session/load_session 连接原生生命周期；NativeManagedMounts 保存真实目录租约；runtime_request 构造宿主 Request；start_prompt_core/with_runtime_runner 连接真实配置、审批、运行器；NativeTurnPersistence 只适配既有端口。persist_turn 全体迁入 persistence.rs。 |
| catalog_mounts.rs | 16 | 16 | 仅注入当前宿主 home 与 validate_session_scope；既有生产下沉不计收益。 |
| models.rs | 39 | 39 | 注入 project instance/tenant 的 provider::list；既有 model_state 和 selection 存储下沉不计本次收益。 |
| backend_tests.rs | 196 | 166 | 真实 session identity/history、原生 Request 模型/工具路由、真实运行器取消传播与历史持久化；领域事件映射整函数迁出。 |
| mod.rs | 35 | 35 | 真实宿主注册和公共后端导出；已删除事件映射测试的旧函数而非整文件挂载。 |
| catalog_mounts_tests.rs | 158 | 158 | 真实当前 home、catalog 数据库、原生会话鉴权、MCP 进程不重启及篡改拒绝。 |
| mount_recovery_tests.rs | 420 | 420 | 原生技能调用、checkpoint、真实进程重启、凭证不落盘、损坏/外租户/权限撤销及磁盘提交后退出恢复。 |
| mount_store.rs | 27 | 27 | save 在写入前验证真实技能目录租约；load/key 为兼容重导出，存储算法已属领域。 |
| mounts_tests.rs | 364 | 364 | 真实 SkillTool/FileReadTool 的权限、附件可读性、MCP 共执行、拒绝发布、来源隔离与清空。 |
| mcp.rs | 102 | 76 | prepare_session_tools 创建宿主 McpSessionConnections/Tool 并发现目录；prune_revoked_mounts 读取真实撤销租约。supported_server_name/validate_server_names 迁入领域。 |

宿主原文件总计 2331 → 2143 行，净减 188 行（8.1%）。
宿主生产文件（含 models.rs 的 3 行 cfg(test) 注册）622 → 464；独立测试文件 1709 → 1679。
领域本次新增生产：persistence.rs 133 行、mcp.rs 32 行；新增测试 33 行。lib.rs 增加 7 行，其中 cfg(test) 注册 2 行，其余生产注册及空行。既有领域文件未作为迁移收益。

## 全用例及夹具映射

以下逐符号列出；保留测试原因见上表对应文件。所有原测试名称/断言保留，无去重或删除场景。夹具保留在原文件供真实宿主测试调用。

### mcp_tests.rs
- `fixture_server` → `宿主原模块::fixture_server`。保留：真实 ToolRuntimeContext 目录隔离、进程启动、环境脱敏、原子失败与外租户禁止启动。
- `mcp_servers_are_scoped_to_bound_session` → `宿主原模块::mcp_servers_are_scoped_to_bound_session`。保留：真实 ToolRuntimeContext 目录隔离、进程启动、环境脱敏、原子失败与外租户禁止启动。
- `mcp_conversion_preserves_supported_transport_without_logging_env` → `宿主原模块::mcp_conversion_preserves_supported_transport_without_logging_env`。保留：真实 ToolRuntimeContext 目录隔离、进程启动、环境脱敏、原子失败与外租户禁止启动。
- `unsupported_or_failed_mcp_is_rejected_atomically` → `宿主原模块::unsupported_or_failed_mcp_is_rejected_atomically`。保留：真实 ToolRuntimeContext 目录隔离、进程启动、环境脱敏、原子失败与外租户禁止启动。
- `loaded_session_rebuilds_requested_mcp_scope` → `宿主原模块::loaded_session_rebuilds_requested_mcp_scope`。保留：真实 ToolRuntimeContext 目录隔离、进程启动、环境脱敏、原子失败与外租户禁止启动。
- `foreign_session_load_rejects_before_starting_mcp_processes` → `宿主原模块::foreign_session_load_rejects_before_starting_mcp_processes`。保留：真实 ToolRuntimeContext 目录隔离、进程启动、环境脱敏、原子失败与外租户禁止启动。

### model_tests.rs
- `ids` → `宿主原模块::ids`。保留：配置持久化、禁用模型、跨租户隔离及真实 Request.model 接线。
- `model_selection_reaches_runtime_and_preserves_other_sessions` → `宿主原模块::model_selection_reaches_runtime_and_preserves_other_sessions`。保留：配置持久化、禁用模型、跨租户隔离及真实 Request.model 接线。

### mount_acceptance_tests.rs
- `acp_server_mount_acceptance_production_stdio_tools_update_and_restart` → `宿主原模块::acp_server_mount_acceptance_production_stdio_tools_update_and_restart`。保留：生产 CLI stdio 实机入口及独立进程租户/同目录隔离，不能用模拟 Host 代替。
- `acp_server_mount_acceptance_tenant_and_same_directory_session_isolation` → `宿主原模块::acp_server_mount_acceptance_tenant_and_same_directory_session_isolation`。保留：生产 CLI stdio 实机入口及独立进程租户/同目录隔离，不能用模拟 Host 代替。
- `isolation_worker` → `宿主原模块::isolation_worker`。保留：生产 CLI stdio 实机入口及独立进程租户/同目录隔离，不能用模拟 Host 代替。

### mount_update_tests.rs
- `update` → `宿主原模块::update`。保留：真实 MCP 回滚/退出/撤销、修订并发及协议忙状态阻止原生操作。
- `skill` → `宿主原模块::skill`。保留：真实 MCP 回滚/退出/撤销、修订并发及协议忙状态阻止原生操作。
- `server` → `宿主原模块::server`。保留：真实 MCP 回滚/退出/撤销、修订并发及协议忙状态阻止原生操作。
- `assert_exited` → `宿主原模块::assert_exited`。保留：真实 MCP 回滚/退出/撤销、修订并发及协议忙状态阻止原生操作。
- `acp_mount_update_rollback_replace_clear_and_revoke_real_processes` → `宿主原模块::acp_mount_update_rollback_replace_clear_and_revoke_real_processes`。保留：真实 MCP 回滚/退出/撤销、修订并发及协议忙状态阻止原生操作。
- `acp_mount_update_same_revision_has_one_winner` → `宿主原模块::acp_mount_update_same_revision_has_one_winner`。保留：真实 MCP 回滚/退出/撤销、修订并发及协议忙状态阻止原生操作。
- `acp_mount_update_protocol_blocks_prompt_load_and_second_update_during_prepare` → `宿主原模块::acp_mount_update_protocol_blocks_prompt_load_and_second_update_during_prepare`。保留：真实 MCP 回滚/退出/撤销、修订并发及协议忙状态阻止原生操作。

### backend_tests.rs
- `tenant` → `宿主原模块::tenant`。保留：真实 session identity/history、原生 Request 模型/工具路由、真实运行器取消传播与历史持久化；领域事件映射整函数迁出。
- `backend_reuses_native_session_identity_and_history` → `宿主原模块::backend_reuses_native_session_identity_and_history`。保留：真实 session identity/history、原生 Request 模型/工具路由、真实运行器取消传播与历史持久化；领域事件映射整函数迁出。
- `backend_prompt_persists_turn_once` → `宿主原模块::backend_prompt_persists_turn_once`。保留：真实 session identity/history、原生 Request 模型/工具路由、真实运行器取消传播与历史持久化；领域事件映射整函数迁出。
- `backend_prompt_uses_native_model_and_local_tools` → `宿主原模块::backend_prompt_uses_native_model_and_local_tools`。保留：真实 session identity/history、原生 Request 模型/工具路由、真实运行器取消传播与历史持久化；领域事件映射整函数迁出。
- `backend_maps_runtime_events_without_parsing_display_text` → `agent_acp::host_migration_53_tests::backend_maps_runtime_events_without_parsing_display_text`。完整迁移。
- `backend_cancel_propagates_to_runtime_turn` → `宿主原模块::backend_cancel_propagates_to_runtime_turn`。保留：真实 session identity/history、原生 Request 模型/工具路由、真实运行器取消传播与历史持久化；领域事件映射整函数迁出。

### catalog_mounts_tests.rs
- `cc_mcp_t29_read_mounts_checks_exact_identity_and_exposes_only_committed_references` → `宿主原模块::cc_mcp_t29_read_mounts_checks_exact_identity_and_exposes_only_committed_references`。保留：真实当前 home、catalog 数据库、原生会话鉴权、MCP 进程不重启及篡改拒绝。

### mount_recovery_tests.rs
- `request` → `宿主原模块::request`。保留：原生技能调用、checkpoint、真实进程重启、凭证不落盘、损坏/外租户/权限撤销及磁盘提交后退出恢复。
- `recovery_process_worker` → `宿主原模块::recovery_process_worker`。保留：原生技能调用、checkpoint、真实进程重启、凭证不落盘、损坏/外租户/权限撤销及磁盘提交后退出恢复。
- `worker` → `宿主原模块::worker`。保留：原生技能调用、checkpoint、真实进程重启、凭证不落盘、损坏/外租户/权限撤销及磁盘提交后退出恢复。
- `fixture` → `宿主原模块::fixture`。保留：原生技能调用、checkpoint、真实进程重启、凭证不落盘、损坏/外租户/权限撤销及磁盘提交后退出恢复。
- `acp_mount_recovery_independent_processes_restore_and_clear` → `宿主原模块::acp_mount_recovery_independent_processes_restore_and_clear`。保留：原生技能调用、checkpoint、真实进程重启、凭证不落盘、损坏/外租户/权限撤销及磁盘提交后退出恢复。
- `acp_mount_recovery_replaced_source_rejected` → `宿主原模块::acp_mount_recovery_replaced_source_rejected`。保留：原生技能调用、checkpoint、真实进程重启、凭证不落盘、损坏/外租户/权限撤销及磁盘提交后退出恢复。
- `phase_is_foreign` → `宿主原模块::phase_is_foreign`。保留：原生技能调用、checkpoint、真实进程重启、凭证不落盘、损坏/外租户/权限撤销及磁盘提交后退出恢复。
- `record_path` → `宿主原模块::record_path`。保留：原生技能调用、checkpoint、真实进程重启、凭证不落盘、损坏/外租户/权限撤销及磁盘提交后退出恢复。
- `acp_mount_recovery_corrupt_and_foreign_records_rejected` → `宿主原模块::acp_mount_recovery_corrupt_and_foreign_records_rejected`。保留：原生技能调用、checkpoint、真实进程重启、凭证不落盘、损坏/外租户/权限撤销及磁盘提交后退出恢复。
- `acp_mount_recovery_persistence_failure_retains_old_revision` → `宿主原模块::acp_mount_recovery_persistence_failure_retains_old_revision`。保留：原生技能调用、checkpoint、真实进程重启、凭证不落盘、损坏/外租户/权限撤销及磁盘提交后退出恢复。
- `mcp` → `宿主原模块::mcp`。保留：原生技能调用、checkpoint、真实进程重启、凭证不落盘、损坏/外租户/权限撤销及磁盘提交后退出恢复。
- `assert_native_call` → `宿主原模块::assert_native_call`。保留：原生技能调用、checkpoint、真实进程重启、凭证不落盘、损坏/外租户/权限撤销及磁盘提交后退出恢复。
- `acp_mount_recovery_mcp_retransmission_and_no_credentials_on_disk` → `宿主原模块::acp_mount_recovery_mcp_retransmission_and_no_credentials_on_disk`。保留：原生技能调用、checkpoint、真实进程重启、凭证不落盘、损坏/外租户/权限撤销及磁盘提交后退出恢复。
- `acp_mount_recovery_explicit_load_replaces_and_clears` → `宿主原模块::acp_mount_recovery_explicit_load_replaces_and_clears`。保留：原生技能调用、checkpoint、真实进程重启、凭证不落盘、损坏/外租户/权限撤销及磁盘提交后退出恢复。
- `acp_mount_recovery_revoked_and_edited_sources_rejected` → `宿主原模块::acp_mount_recovery_revoked_and_edited_sources_rejected`。保留：原生技能调用、checkpoint、真实进程重启、凭证不落盘、损坏/外租户/权限撤销及磁盘提交后退出恢复。
- `invoke_native_skill` → `宿主原模块::invoke_native_skill`。保留：原生技能调用、checkpoint、真实进程重启、凭证不落盘、损坏/外租户/权限撤销及磁盘提交后退出恢复。
- `checkpoint` → `宿主原模块::checkpoint`。保留：原生技能调用、checkpoint、真实进程重启、凭证不落盘、损坏/外租户/权限撤销及磁盘提交后退出恢复。
- `acp_mount_recovery_committed_revision_survives_exit_before_publication` → `宿主原模块::acp_mount_recovery_committed_revision_survives_exit_before_publication`。保留：原生技能调用、checkpoint、真实进程重启、凭证不落盘、损坏/外租户/权限撤销及磁盘提交后退出恢复。

### mounts_tests.rs
- `acp_server_skill_overlay_accepts_native_directory` → `宿主原模块::acp_server_skill_overlay_accepts_native_directory`。保留：真实 SkillTool/FileReadTool 的权限、附件可读性、MCP 共执行、拒绝发布、来源隔离与清空。
- `source` → `宿主原模块::source`。保留：真实 SkillTool/FileReadTool 的权限、附件可读性、MCP 共执行、拒绝发布、来源隔离与清空。
- `request` → `宿主原模块::request`。保留：真实 SkillTool/FileReadTool 的权限、附件可读性、MCP 共执行、拒绝发布、来源隔离与清空。
- `acp_server_skill_overlay_invokes_reads_attachment_and_rediscovers_without_full_access` → `宿主原模块::acp_server_skill_overlay_invokes_reads_attachment_and_rediscovers_without_full_access`。保留：真实 SkillTool/FileReadTool 的权限、附件可读性、MCP 共执行、拒绝发布、来源隔离与清空。
- `acp_server_skill_overlay_rejects_conflicts_and_failed_mcp_without_publishing` → `宿主原模块::acp_server_skill_overlay_rejects_conflicts_and_failed_mcp_without_publishing`。保留：真实 SkillTool/FileReadTool 的权限、附件可读性、MCP 共执行、拒绝发布、来源隔离与清空。
- `acp_server_skill_overlay_and_real_mcp_execute_in_one_prepared_session` → `宿主原模块::acp_server_skill_overlay_and_real_mcp_execute_in_one_prepared_session`。保留：真实 SkillTool/FileReadTool 的权限、附件可读性、MCP 共执行、拒绝发布、来源隔离与清空。
- `acp_server_skill_overlay_permission_change_revokes_read_and_invocation` → `宿主原模块::acp_server_skill_overlay_permission_change_revokes_read_and_invocation`。保留：真实 SkillTool/FileReadTool 的权限、附件可读性、MCP 共执行、拒绝发布、来源隔离与清空。
- `acp_server_skill_overlay_retains_native_skill_permission_denial` → `宿主原模块::acp_server_skill_overlay_retains_native_skill_permission_denial`。保留：真实 SkillTool/FileReadTool 的权限、附件可读性、MCP 共执行、拒绝发布、来源隔离与清空。
- `acp_server_skill_overlay_same_name_sources_are_isolated_and_explicit_empty_clears` → `宿主原模块::acp_server_skill_overlay_same_name_sources_are_isolated_and_explicit_empty_clears`。保留：真实 SkillTool/FileReadTool 的权限、附件可读性、MCP 共执行、拒绝发布、来源隔离与清空。

## 减量例外及继续可迁项

未达到 70%，不宣称减量达标，提交收口任务验收。上表覆盖每个保留文件及行数。可继续拆分：mounts.rs 的版本判断、请求响应组装；mcp.rs 的工具发现编排；models.rs 的选中模型可用性判断；mount_update_tests 的修订竞争纯领域断言。后续应保留真实端口验收，不能将整套 NativeAcpHost 复制到领域测试或用 mock 冒充原生鉴权/权限/进程覆盖。恢复及 acceptance 两组保持真实 subprocess 路径字符串不变。

## 未来验证（本次全部未运行）

```sh
python3 scripts/cargo_slot.py cargo test -p agent-acp --lib host_migration_53_tests -- --test-threads=1
python3 scripts/cargo_slot.py cargo test -p vw-agent --lib acp::backend_tests:: -- --test-threads=1
```

静态预期领域过滤 1 个；宿主 backend_tests 过滤 4 个（包含真实持久化与取消）。全部剩余宿主用例按上表的原模块与名称执行；macOS catalog、Unix 权限测试遵循原 cfg，acceptance 需同槽位预先存在生产 shiju。未来宿主 --list 应不再含 acp::backend_tests::backend_maps_runtime_events_without_parsing_display_text；不能把零测试算通过。

未取得编译、运行、退出码或构建性能证据。RustCodeGraph status/explore 用于导航；静态读取/差异检查不能代替行为验收。外部挂载清单未命中 acp/ 来源，未发现本族跨目录 path 挂载。
