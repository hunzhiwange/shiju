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

## 深度恢复补充（覆盖上文相应旧归属）

读取 final.md 与 remaining-host.json，原任务53保留2143行；本次继续拆分，不把早先保留说明当作验收结论。

- mcp::prepare_session_tools 的同步、全量发现、去重标识、错误脱敏、SessionToolset 租约和目录组装整体归 agent-acp::mcp。宿主只创建携带真实 ConnectionHost/MountLease 的连接对象，调用领域入口。validate_server_names 仍在同步/发现之前。
- mounts::prepare_skills 归 agent-acp::mounts，持有版本验证、selection 解析、scope binding、SessionSkillView 准备、SessionSkillSources 构造/扩展。宿主注入现有 config.skills.security 与延迟的原生技能发现闭包；先建立 overlay 租约再调用原生发现，保持既有顺序。PreparedMounts 类型归领域。宿主保留项目配置获取、managed leases 解包和真实 MCP 接线及最后租约校验。
- mcp_conversion_preserves_supported_transport_without_logging_env / unsupported_or_failed_mcp_is_rejected_atomically：完整断言及 stdio fixture 迁入 agent-acp::host_migration_53_mcp_contracts（test-support）。原名宿主测试仅创建临时目录、注入真实 PluginConnectionHost 后调用；无 mock、无反向源码挂载，测试由宿主原注册驱动，不能声称这两项已经由 agent-acp --lib 自运行。
- 其余原用例未修改，映射沿用前表。新增生产依赖 plugin-mcp/tool-types/tool-assembly/acp-config/agent-skills/config-skills/mcp-client，锁文件只更新 agent-acp 依赖块。config-security 原有 dev 依赖保留。

当前物理宿主 2143 → 2063 行，本轮再减 80 行；初始2331行累计净减 268 行，仍低于70%。未将迁移的 test-support 断言算生产算法。

| 本轮文件 | 前行 | 后行 |
|---|---:|---:|
| mcp_tests.rs | 152 | 136 |
| mounts.rs | 103 | 72 |
| mcp.rs | 76 | 43 |

领域 mcp.rs 当前 78 行。

领域 mounts.rs 当前 52 行。

领域 host_migration_53_mcp_contracts.rs 当前 70 行。

未运行任何 Cargo、构建、单元/实机测试；未取得通过数。未来原生精确过滤：

```sh
python3 scripts/cargo_slot.py cargo test -p vw-agent --lib acp::mcp_tests:: -- --test-threads=1
python3 scripts/cargo_slot.py cargo test -p vw-agent --lib acp::mounts_tests:: -- --test-threads=1
```

静态预期分别5和7项（后者含Unix条件用例）。领域自注册事件映射仍预期1项。需要未来编译校验新增依赖与跨库类型一致性。

剩余大头仍为真实 NativeAcpHost/SkillTool/FileReadTool/CLI 独立进程验收；继续可迁项是测试协议的纯修订/状态断言，而非把原生Host复制进领域。该恢复没有关闭整轮35000行门槛，交收口按本次实际数字验收。

追加：mount_store::save 的完整真实租约校验与存储参数组装也归 agent-acp::mounts::save，宿主仅5行重导出。恢复当前总行数以宿主物理源码为准：2041行。

## 再恢复：整组恢复、更新与验收领域契约

再次读取 final.md 与 remaining-host.json，本任务在该快照残留2041行。新增三个 test-support 模块，由宿主原测试注册调用，不反向依赖或读取宿主源码：

| 宿主原文件 | 领域接收模块 | 原行 | 现行 | 保留的真实端口 |
|---|---|---:|---:|---|
| mount_recovery_tests.rs | host_migration_53_recovery_contracts | 420 | 142 | 独立进程 worker、真实 SkillTool/ToolUseContext、检查点 project/tenant 上下文和原生目录查询 |

完整用例映射：

- `acp::mount_recovery_tests::recovery_process_worker` → `agent_acp::host_migration_53_recovery_contracts::recovery_process_worker`；宿主原名保留薄注册。
- `acp::mount_recovery_tests::acp_mount_recovery_independent_processes_restore_and_clear` → `agent_acp::host_migration_53_recovery_contracts::acp_mount_recovery_independent_processes_restore_and_clear`；宿主原名保留薄注册。
- `acp::mount_recovery_tests::acp_mount_recovery_replaced_source_rejected` → `agent_acp::host_migration_53_recovery_contracts::acp_mount_recovery_replaced_source_rejected`；宿主原名保留薄注册。
- `acp::mount_recovery_tests::acp_mount_recovery_corrupt_and_foreign_records_rejected` → `agent_acp::host_migration_53_recovery_contracts::acp_mount_recovery_corrupt_and_foreign_records_rejected`；宿主原名保留薄注册。
- `acp::mount_recovery_tests::acp_mount_recovery_persistence_failure_retains_old_revision` → `agent_acp::host_migration_53_recovery_contracts::acp_mount_recovery_persistence_failure_retains_old_revision`；宿主原名保留薄注册。
- `acp::mount_recovery_tests::acp_mount_recovery_mcp_retransmission_and_no_credentials_on_disk` → `agent_acp::host_migration_53_recovery_contracts::acp_mount_recovery_mcp_retransmission_and_no_credentials_on_disk`；宿主原名保留薄注册。
- `acp::mount_recovery_tests::acp_mount_recovery_explicit_load_replaces_and_clears` → `agent_acp::host_migration_53_recovery_contracts::acp_mount_recovery_explicit_load_replaces_and_clears`；宿主原名保留薄注册。
- `acp::mount_recovery_tests::acp_mount_recovery_revoked_and_edited_sources_rejected` → `agent_acp::host_migration_53_recovery_contracts::acp_mount_recovery_revoked_and_edited_sources_rejected`；宿主原名保留薄注册。
- `acp::mount_recovery_tests::acp_mount_recovery_committed_revision_survives_exit_before_publication` → `agent_acp::host_migration_53_recovery_contracts::acp_mount_recovery_committed_revision_survives_exit_before_publication`；宿主原名保留薄注册。

静态断言宏计数 21 → 宿主 3 + 领域 18；这是迁移核对，不是运行证明。

| mount_update_tests.rs | host_migration_53_update_contracts | 238 | 21 | NativeAcpHost 类型注入、原生目录查询 |

完整用例映射：

- `acp::mount_update_tests::acp_mount_update_rollback_replace_clear_and_revoke_real_processes` → `agent_acp::host_migration_53_update_contracts::acp_mount_update_rollback_replace_clear_and_revoke_real_processes`；宿主原名保留薄注册。
- `acp::mount_update_tests::acp_mount_update_same_revision_has_one_winner` → `agent_acp::host_migration_53_update_contracts::acp_mount_update_same_revision_has_one_winner`；宿主原名保留薄注册。
- `acp::mount_update_tests::acp_mount_update_protocol_blocks_prompt_load_and_second_update_during_prepare` → `agent_acp::host_migration_53_update_contracts::acp_mount_update_protocol_blocks_prompt_load_and_second_update_during_prepare`；宿主原名保留薄注册。

静态断言宏计数 28 → 宿主 0 + 领域 28；这是迁移核对，不是运行证明。

| mount_acceptance_tests.rs | host_migration_53_acceptance_contracts | 98 | 16 | 生产 CLI 仓库根路径和 NativeAcpHost 类型注入 |

完整用例映射：

- `acp::mount_acceptance_tests::acp_server_mount_acceptance_production_stdio_tools_update_and_restart` → `agent_acp::host_migration_53_acceptance_contracts::acp_server_mount_acceptance_production_stdio_tools_update_and_restart`；宿主原名保留薄注册。
- `acp::mount_acceptance_tests::acp_server_mount_acceptance_tenant_and_same_directory_session_isolation` → `agent_acp::host_migration_53_acceptance_contracts::acp_server_mount_acceptance_tenant_and_same_directory_session_isolation`；宿主原名保留薄注册。
- `acp::mount_acceptance_tests::isolation_worker` → `agent_acp::host_migration_53_acceptance_contracts::isolation_worker`；宿主原名保留薄注册。

静态断言宏计数 12 → 宿主 0 + 领域 12；这是迁移核对，不是运行证明。

当前宿主本族总计 1464 行；相对初始2331行累计减少 867 行（37.2%），仍未达到70%。

完整迁入的职责包含：恢复各 phase 的修订与技能断言、替换/撤销/篡改/损坏记录拒绝、磁盘持久化失败回滚、凭证不落盘、磁盘提交后 publication 前退出；更新的原子回滚、真实进程退出、并发同修订唯一成功者、协议 busy 阻止 prompt/load/update；生产 CLI 的脚本结果断言与实际 tenant/session 隔离。所有 subprocess --exact 原宿主入口字符串与环境隔离变量保留。

RecoveryPorts 仅在 test-support 下提供三个实际使用的测试端口（invoke_skill/checkpoint/skill_body）；关联 Host 是现有生产 protocol::Host，宿主注入 NativeAcpHost，无 mock，不复制 App/AppState。提交点恢复经同一个 NativeAcpHost::prepare_revision/save_mounts 调用，保持原生校验路径。领域 recovery 的 assert_native_call、fixture、record_path、mcp 夹具及所有编排均迁出；宿主仅保留进程 worker 和技能/检查点函数。

依赖：tempfile/walkdir 为已有库的 optional 依赖，仅 test-support 打开；tokio/time 同样限定该 feature。锁文件仅追加 agent-acp 对应依赖。领域契约仍由宿主真实用例执行，不能称为 agent-acp --lib 自注册测试或编译减负；只是断言与夹具所有权归还。

未来仅运行原过滤：

```sh
python3 scripts/cargo_slot.py cargo test -p vw-agent --lib acp::mount_recovery_tests:: -- --test-threads=1
python3 scripts/cargo_slot.py cargo test -p vw-agent --lib acp::mount_update_tests:: -- --test-threads=1
python3 scripts/cargo_slot.py cargo test -p vw-agent --lib acp::mount_acceptance_tests:: -- --test-threads=1
```

静态预期分别9、3、3项（包含隔离 worker；worker 无隔离变量返回与原先一致，父场景必须实际启动子进程）。acceptance 仍要求同槽位生产 shiju；禁止本轮执行。所有 Cargo、构建及测试均未运行，未声明通过。任务卡本轮未重建，原任务已删除，证据保留。剩余 mounts_tests 的实际 SkillTool/FileReadTool 权限、附件与宿主配置绑定尚需进一步拆分，不能整文件豁免；整轮门槛继续由收口核实。

追加 catalog 完整契约迁移：`acp::catalog_mounts_tests::cc_mcp_t29_read_mounts_checks_exact_identity_and_exposes_only_committed_references` → `agent_acp::host_migration_53_catalog_contracts` 同名函数。宿主 158 → 15 行，只保留真实 home 与授权 read_catalog_mounts 端口及原名测试注册。目录数据库操作、真实项目创建、原生无ACP提交会话、篡改/外租户/旧revision/删除断言完整归领域；断言宏 19 → 0+19，并保留隔离子进程 --exact 原入口。新增 optional vw-agent-apps 是实际 McpCatalogDraft 类型依赖，不是 vw-agent 宿主反向依赖；随 test-support 启用，模块保持 macOS 条件。

本轮最后统计替代上文中间数字：宿主本族当前 1321 行，初始2331行累计净减 1010 行（43.3%）；本轮从2041行再减 720 行。独立测试文件当前 943 行；领域新增四个契约模块共 823 行，均计测试支持而非生产。原生产文件本轮未改变。

未来 catalog 精确过滤（macOS，预期1项）：`python3 scripts/cargo_slot.py cargo test -p vw-agent --lib acp::catalog_mounts_tests::cc_mcp_t29_read_mounts_checks_exact_identity_and_exposes_only_committed_references -- --exact --test-threads=1`。本轮静态 diff 空白核对与原断言宏/原名注册核对完成；未执行任何编译/行为验证。

## 任务59最新 owner=53 具体恢复项

读取 task-59-recovery-priorities.json（owner=53），明确请求的三个符号及 source-name conflict / explicit-clear / publication expectation 矩阵均由下述 overlay 契约接收。遵从 retain_real_boundary：原生文件读取/真实权限、真实MCP调用和 root backend prompt 编排仍执行实际宿主路径。已恢复编号任务卡用于本轮收尾，提交后删除。

- `acp::mounts_tests::acp_server_skill_overlay_accepts_native_directory` → `agent_acp::host_migration_53_overlay_contracts::acp_server_skill_overlay_accepts_native_directory`；原名薄注册注入 NativeAcpHost。
- `acp::mounts_tests::acp_server_skill_overlay_rejects_conflicts_and_failed_mcp_without_publishing` → `agent_acp::host_migration_53_overlay_contracts::acp_server_skill_overlay_rejects_conflicts_and_failed_mcp_without_publishing`；原名薄注册注入 NativeAcpHost。
- `acp::mounts_tests::acp_server_skill_overlay_permission_change_revokes_read_and_invocation` → `agent_acp::host_migration_53_overlay_contracts::acp_server_skill_overlay_permission_change_revokes_read_and_invocation`；原名薄注册注入 NativeAcpHost。
- `acp::mounts_tests::acp_server_skill_overlay_same_name_sources_are_isolated_and_explicit_empty_clears` → `agent_acp::host_migration_53_overlay_contracts::acp_server_skill_overlay_same_name_sources_are_isolated_and_explicit_empty_clears`；原名薄注册注入 NativeAcpHost。

挂载原文件 364 → 214 行；断言宏 31 → 宿主 18 + 领域 13。Unix 权限用例在两侧保留 cfg(unix)。目录发现通过实际 load_session_skills 注入，领域仍验证 errors、来源正文隔离、清空、撤销和 publish 前失败；不使用替代目录或模拟授权。

逐函数剩余宿主职责：

- acp_server_skill_overlay_invokes_reads_attachment_and_rediscovers_without_full_access：真实 ToolUseContext/SkillTool/FileReadTool、两次目录发现和附件读、跨会话文件拒绝、禁用后的真实执行拒绝。
- acp_server_skill_overlay_and_real_mcp_execute_in_one_prepared_session：真实 SkillTool 与 MCP stdio 的共同执行上下文，调用真实 tools/call 并读附件。
- acp_server_skill_overlay_retains_native_skill_permission_denial：真实 delegated deny 规则到 SkillTool 执行的拒绝路径及错误信息。
- source/request：只供上述三项真实宿主测试使用；领域来源夹具在本库自持，没有 path 加载宿主。

完整模型映射：`acp::backend::models::tests::model_selection_reaches_runtime_and_preserves_other_sessions` → `agent_acp::host_migration_53_model_contracts::model_selection_reaches_runtime_and_preserves_other_sessions`；ids 夹具与模型顺序/禁用/空id/租户/其它会话/重连/恢复 auto 断言整体归领域。宿主保留 config.providers 的真实保存和 Request.model 到原生运行器的1项断言。原文件 83 → 22 行；断言宏 11 → 1 + 10，不复制 Config/App 状态。

MCP 全映射补充：

- `acp::mcp_tests::loaded_session_rebuilds_requested_mcp_scope` → `agent_acp::host_migration_53_mcp_contracts::loaded_session_rebuilds_requested_mcp_scope`，真实 NativeAcpHost 创建/重载、session_tool_count 的1→0断言整体领域持有。
- `acp::mcp_tests::foreign_session_load_rejects_before_starting_mcp_processes` → 同领域模块同名函数，外租户载入前拒绝与真实进程 marker 不存在断言整体迁出。
- fixture_server 在领域单一持有，宿主第一项真实 ToolRuntimeContext catalog 隔离用例也调用此夹具。其它原 MCP 两项薄测试继续注入真实 ConnectionHost。

当前最后物理统计：宿主1321 → 1026 行，本轮净减 295 行；从2331基线累计净减 1305 行（56.0%），仍未达到70%，不宣称整轮门槛关闭。本轮生产实现未变化，迁出均为领域断言/夹具，仍经真实Host驱动，不计独立领域单测或编译提速。

未来限定过滤：

```sh
python3 scripts/cargo_slot.py cargo test -p vw-agent --lib acp::mounts_tests:: -- --test-threads=1
python3 scripts/cargo_slot.py cargo test -p vw-agent --lib acp::backend::models::tests::model_selection_reaches_runtime_and_preserves_other_sessions -- --exact --test-threads=1
python3 scripts/cargo_slot.py cargo test -p vw-agent --lib acp::mcp_tests:: -- --test-threads=1
```

静态预期Unix为7、1、5项，非Unix挂载6项。原名/属性/关键断言计数及 git diff --check 做静态核对；所有编译、Cargo、单元/实机测试均未运行。lib.rs 最小 test-support 注册，沿用既有 optional tempfile 和 tokio/time；无新依赖、反向引用或跨目录宿主源码借用。

静态终检修正：source/request 夹具在领域保留一份，宿主保留供三项真实执行验收使用的一份，消除重复定义；最后宿主总行数 1046，本轮净减 275，累计净减 1285（55.1%）。上述1026是夹具恢复前中间数字，以此为准。
