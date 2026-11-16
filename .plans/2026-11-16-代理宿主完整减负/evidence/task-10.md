# 任务10：session 文件组1迁移证据

代码迁移及静态接线完成；全部编译/行为检查未运行。未达到70%净减门槛，必须由任务59核对下列保留理由与继续可迁项；不宣称本组减量达标。

## 口径与数量

原14文件 4200 行，现 3481 行，宿主净减 719 行（17.12%）。按 splitlines；测试文件含夹具/注释；生产文件内 cfg(test) 注册单列，不能把 forwarding 算作真实生产策略。原测试函数 66，迁出 28，保留 38；没有删除场景。

分项：宿主生产文件 328→325 行（含转发、文档和测试注册，非纯策略）；宿主测试/夹具文件 3872→3156 行。实际新迁生产策略为0行；领域原生产文件 session_ops.rs 725→732 行（含4行测试注册），新增测试/端口文件共779行。

## 逐文件职责及行数

| 来源 | 修改前 | 修改后 | 保留原因/继续可迁项 |
|---|---:|---:|---|
| `acp_options.rs` | 44 | 44 | AcpHost 的真实 ACP 进程调用、binding 持久化、Agent catalog 装配及 HTTP DTO 转换；生产策略此前已在 gateway-session/acp_options.rs，本轮不重复计收益。 |
| `acp_source_tests.rs` | 253 | 253 | 真实回环 HTTP、RecoveryConfig、ACP 配置落盘/选项拒绝以及 chat 流权限入口，依赖实际 ACP Host 装配；替换为领域假端口会丢失该边界。 |
| `agent_profile_acceptance_tests.rs` | 1082 | 1082 | 配置 handlers→stream→实际工具/模型/ACP 传输的完整端到端验收、宿主恢复配置和子进程隔离；外部 include_str 的 contracts.json 属数据夹具，仍留原处。可继续拆分 profile 值比较辅助断言，但当前完整函数包含真实配置/流调用，不拆断场景。 |
| `archived_delete.rs` | 7 | 7 | 唯一 delete 是 State<AppState> 的 typed route assembly；删除算法已经在领域库。 |
| `archived_delete_tests.rs` | 254 | 254 | 真实 HTTP router 的租户/项目隔离、会话树和 UI 双持久化、history 锁与恢复竞态。strict_enumeration 用例可继续整体下沉 agent-storage，需统一其全局存储初始化，不伪称本轮迁出。 |
| `fork_acceptance_tests.rs` | 506 | 506 | 跨进程 HOME/数据库重启、真实模型传输、worktree 与桌面读端点组合验收；需保留真实宿主生命周期。可继续抽取记录值断言，不能把独立辅助函数计成完整场景迁移。 |
| `fork_acp_tests.rs` | 288 | 288 | 通过父 turn_fork_tests 挂载，使用实际 fork、ACP 第一提示及跨进程重启。父夹具归任务11，完整场景保留实际 Host；不是领域文件的孤立副本。 |
| `message_ops.rs` | 71 | 72 | MessageHost 实现和具体处理器转发保留以适配宿主 InstanceQuery；真实领域消息处理已在 message_ops 中，不新增迁出生产收益。 |
| `message_ops_tests.rs` | 451 | 10 | 只保留实际宿主四个 handler 签名挂载契约；存储/错误/不匹配完整函数已迁走。 |
| `raw_commit.rs` | 2 | 2 | acknowledge/claim/register 为实际 UI/stream 共享 registry 薄重导出，仍有调用者。 |
| `session_ops.rs` | 176 | 174 | 真实 SessionHost 端口、PreparedFork、State<AppState> 和 typed forwarding。CRUD/fork/todo 策略已在领域库；本轮只迁 summary 测试及移除测试专用 import。 |
| `session_ops_tests.rs` | 765 | 694 | 生命周期、状态、todo、跨项目变更拒绝、fork/reset/title 场景直接使用实际 AppState/运行时端口；turn_fork_tests 子模块由任务11拥有。本轮两个 summary 域用例迁出。后续可把 list/patch/todo 部分接入真实 session runtime fixture，但不能复制 AppState 到库。 |
| `shared.rs` | 28 | 26 | with_tenant_instance 是真实租户/项目检查与宿主 bootstrap；resolve_scope_from_query 转发实际 Host。其余重导出用于既有调用。 |
| `shared_tests.rs` | 273 | 69 | 只留两条实际 Host scope 解析和一条真实项目上下文回调；17条 DTO/错误/目录判断域用例全部迁移。 |

领域新增测试（含局部夹具）：`host_migration_10_tests/messages.rs` 463行, `host_migration_10_tests/shared.rs` 206行, `host_migration_10_tests/summary.rs` 68行；根测试端口模块 42行。

领域生产改动只有私有 summary 刷新函数以 AsyncFn 接收当前真实 H::refresh_summary；原有过滤、错误忽略、更新条件及顺序不变。该函数保持私有，测试在 session_ops 内自注册，没有为测试暴露 API 或新增 Host trait。原宿主 refresh 测试调用已经不存在的同名 helper，迁入真实 helper 所属模块后移除失效调用；失败来源改为明确的刷新错误端口，原三个 summary 保持断言保留。

## 全用例映射

每行列出原挂载下的完整用例后缀；迁移目标由 lib.rs 自注册，summary 由 session_ops.rs 内自注册。保留用例保持原路径与挂载，不把 mock 端口声称为实际宿主验收。

### acp_source_tests

- `acp_model_source_ui_http_persists_and_rejects_without_local_fallback` → 原宿主模块保留。真实回环 HTTP、RecoveryConfig、ACP 配置落盘/选项拒绝以及 chat 流权限入口，依赖实际 ACP Host 装配；替换为领域假端口会丢失该边界。
- `acp_public_permission_options_reject_direct_put_and_chat_over_http` → 原宿主模块保留。真实回环 HTTP、RecoveryConfig、ACP 配置落盘/选项拒绝以及 chat 流权限入口，依赖实际 ACP Host 装配；替换为领域假端口会丢失该边界。
### agent_profile_acceptance_tests

- `agent_profile_acceptance_runs_selected_profile_end_to_end` → 原宿主模块保留。配置 handlers→stream→实际工具/模型/ACP 传输的完整端到端验收、宿主恢复配置和子进程隔离；外部 include_str 的 contracts.json 属数据夹具，仍留原处。可继续拆分 profile 值比较辅助断言，但当前完整函数包含真实配置/流调用，不拆断场景。
- `agent_profile_acceptance_rejects_revoked_capabilities` → 原宿主模块保留。配置 handlers→stream→实际工具/模型/ACP 传输的完整端到端验收、宿主恢复配置和子进程隔离；外部 include_str 的 contracts.json 属数据夹具，仍留原处。可继续拆分 profile 值比较辅助断言，但当前完整函数包含真实配置/流调用，不拆断场景。
- `agent_profile_acceptance_catalog_uses_request_tenant` → 原宿主模块保留。配置 handlers→stream→实际工具/模型/ACP 传输的完整端到端验收、宿主恢复配置和子进程隔离；外部 include_str 的 contracts.json 属数据夹具，仍留原处。可继续拆分 profile 值比较辅助断言，但当前完整函数包含真实配置/流调用，不拆断场景。
- `agent_config_cleanup_global_profile_roundtrips_to_new_session` → 原宿主模块保留。配置 handlers→stream→实际工具/模型/ACP 传输的完整端到端验收、宿主恢复配置和子进程隔离；外部 include_str 的 contracts.json 属数据夹具，仍留原处。可继续拆分 profile 值比较辅助断言，但当前完整函数包含真实配置/流调用，不拆断场景。
- `acp_onboarding_autohand_builtin_profile_persists_and_runs` → 原宿主模块保留。配置 handlers→stream→实际工具/模型/ACP 传输的完整端到端验收、宿主恢复配置和子进程隔离；外部 include_str 的 contracts.json 属数据夹具，仍留原处。可继续拆分 profile 值比较辅助断言，但当前完整函数包含真实配置/流调用，不拆断场景。
- `acp_onboarding_deepagents_builtin_profile_persists_and_runs` → 原宿主模块保留。配置 handlers→stream→实际工具/模型/ACP 传输的完整端到端验收、宿主恢复配置和子进程隔离；外部 include_str 的 contracts.json 属数据夹具，仍留原处。可继续拆分 profile 值比较辅助断言，但当前完整函数包含真实配置/流调用，不拆断场景。
- `acp_onboarding_dimcode_builtin_profile_persists_and_runs` → 原宿主模块保留。配置 handlers→stream→实际工具/模型/ACP 传输的完整端到端验收、宿主恢复配置和子进程隔离；外部 include_str 的 contracts.json 属数据夹具，仍留原处。可继续拆分 profile 值比较辅助断言，但当前完整函数包含真实配置/流调用，不拆断场景。
- `acp_onboarding_dirac_builtin_profile_persists_and_runs` → 原宿主模块保留。配置 handlers→stream→实际工具/模型/ACP 传输的完整端到端验收、宿主恢复配置和子进程隔离；外部 include_str 的 contracts.json 属数据夹具，仍留原处。可继续拆分 profile 值比较辅助断言，但当前完整函数包含真实配置/流调用，不拆断场景。
- `acp_onboarding_glm_acp_agent_builtin_profile_persists_and_runs` → 原宿主模块保留。配置 handlers→stream→实际工具/模型/ACP 传输的完整端到端验收、宿主恢复配置和子进程隔离；外部 include_str 的 contracts.json 属数据夹具，仍留原处。可继续拆分 profile 值比较辅助断言，但当前完整函数包含真实配置/流调用，不拆断场景。
- `acp_onboarding_grok_builtin_profile_persists_and_runs` → 原宿主模块保留。配置 handlers→stream→实际工具/模型/ACP 传输的完整端到端验收、宿主恢复配置和子进程隔离；外部 include_str 的 contracts.json 属数据夹具，仍留原处。可继续拆分 profile 值比较辅助断言，但当前完整函数包含真实配置/流调用，不拆断场景。
- `acp_onboarding_nova_builtin_profile_persists_and_runs` → 原宿主模块保留。配置 handlers→stream→实际工具/模型/ACP 传输的完整端到端验收、宿主恢复配置和子进程隔离；外部 include_str 的 contracts.json 属数据夹具，仍留原处。可继续拆分 profile 值比较辅助断言，但当前完整函数包含真实配置/流调用，不拆断场景。
- `acp_onboarding_nova_real_builtin_profile_persists_and_runs` → 原宿主模块保留。配置 handlers→stream→实际工具/模型/ACP 传输的完整端到端验收、宿主恢复配置和子进程隔离；外部 include_str 的 contracts.json 属数据夹具，仍留原处。可继续拆分 profile 值比较辅助断言，但当前完整函数包含真实配置/流调用，不拆断场景。
- `acp_onboarding_sigit_builtin_profile_persists_and_runs` → 原宿主模块保留。配置 handlers→stream→实际工具/模型/ACP 传输的完整端到端验收、宿主恢复配置和子进程隔离；外部 include_str 的 contracts.json 属数据夹具，仍留原处。可继续拆分 profile 值比较辅助断言，但当前完整函数包含真实配置/流调用，不拆断场景。
- `acp_onboarding_sigit_real_builtin_profile_persists_and_runs` → 原宿主模块保留。配置 handlers→stream→实际工具/模型/ACP 传输的完整端到端验收、宿主恢复配置和子进程隔离；外部 include_str 的 contracts.json 属数据夹具，仍留原处。可继续拆分 profile 值比较辅助断言，但当前完整函数包含真实配置/流调用，不拆断场景。
- `acp_onboarding_amp_acp_builtin_profile_persists_and_runs` → 原宿主模块保留。配置 handlers→stream→实际工具/模型/ACP 传输的完整端到端验收、宿主恢复配置和子进程隔离；外部 include_str 的 contracts.json 属数据夹具，仍留原处。可继续拆分 profile 值比较辅助断言，但当前完整函数包含真实配置/流调用，不拆断场景。
### archived_delete_tests

- `archived_chat_delete_strict_enumeration_rejects_a_file_prefix` → 原宿主模块保留。真实 HTTP router 的租户/项目隔离、会话树和 UI 双持久化、history 锁与恢复竞态。strict_enumeration 用例可继续整体下沉 agent-storage，需统一其全局存储初始化，不伪称本轮迁出。
- `archived_chat_delete_http_removes_multilevel_tree_and_preserves_other_scopes` → 原宿主模块保留。真实 HTTP router 的租户/项目隔离、会话树和 UI 双持久化、history 锁与恢复竞态。strict_enumeration 用例可继续整体下沉 agent-storage，需统一其全局存储初始化，不伪称本轮迁出。
- `archived_chat_delete_http_rejects_live_root_and_restore_while_waiting_for_history` → 原宿主模块保留。真实 HTTP router 的租户/项目隔离、会话树和 UI 双持久化、history 锁与恢复竞态。strict_enumeration 用例可继续整体下沉 agent-storage，需统一其全局存储初始化，不伪称本轮迁出。
- `archived_chat_delete_storage_failure_keeps_root_and_allows_retry` → 原宿主模块保留。真实 HTTP router 的租户/项目隔离、会话树和 UI 双持久化、history 锁与恢复竞态。strict_enumeration 用例可继续整体下沉 agent-storage，需统一其全局存储初始化，不伪称本轮迁出。
### fork_acceptance_tests

- `fork_acceptance_negative_detects_discarded_model_history` → 原宿主模块保留。跨进程 HOME/数据库重启、真实模型传输、worktree 与桌面读端点组合验收；需保留真实宿主生命周期。可继续抽取记录值断言，不能把独立辅助函数计成完整场景迁移。
- `fork_acceptance_restarts_and_resumes_both_targets` → 原宿主模块保留。跨进程 HOME/数据库重启、真实模型传输、worktree 与桌面读端点组合验收；需保留真实宿主生命周期。可继续抽取记录值断言，不能把独立辅助函数计成完整场景迁移。
### fork_acp_tests

- `fork_acp_negative_missing_history_is_detected` → 原宿主模块保留。通过父 turn_fork_tests 挂载，使用实际 fork、ACP 第一提示及跨进程重启。父夹具归任务11，完整场景保留实际 Host；不是领域文件的孤立副本。
- `fork_acp_first_prompt_and_restart_preserve_history` → 原宿主模块保留。通过父 turn_fork_tests 挂载，使用实际 fork、ACP 第一提示及跨进程重启。父夹具归任务11，完整场景保留实际 Host；不是领域文件的孤立副本。
### message_ops_tests

- `message_handlers_are_available` → 原宿主模块保留。只保留实际宿主四个 handler 签名挂载契约；存储/错误/不匹配完整函数已迁走。
- `session_message_part_patch_rejects_body_path_mismatch_before_storage_access` → `gateway-session::host_migration_10_tests::messages::session_message_part_patch_rejects_body_path_mismatch_before_storage_access`
- `session_message_part_patch_rejects_message_mismatch_when_other_ids_match` → `gateway-session::host_migration_10_tests::messages::session_message_part_patch_rejects_message_mismatch_when_other_ids_match`
- `session_message_part_patch_rejects_session_mismatch_when_other_ids_match` → `gateway-session::host_migration_10_tests::messages::session_message_part_patch_rejects_session_mismatch_when_other_ids_match`
- `session_message_list_honors_zero_limit_and_unbounded_newest_first` → `gateway-session::host_migration_10_tests::messages::session_message_list_honors_zero_limit_and_unbounded_newest_first`
- `session_message_list_rejects_incomplete_history` → `gateway-session::host_migration_10_tests::messages::session_message_list_rejects_incomplete_history`
- `session_message_list_returns_empty_for_unknown_session` → `gateway-session::host_migration_10_tests::messages::session_message_list_returns_empty_for_unknown_session`
- `session_message_handlers_round_trip_messages_and_parts` → `gateway-session::host_migration_10_tests::messages::session_message_handlers_round_trip_messages_and_parts`
- `session_message_get_maps_missing_message_to_bad_request` → `gateway-session::host_migration_10_tests::messages::session_message_get_maps_missing_message_to_bad_request`
- `session_message_part_delete_is_idempotent_for_missing_part` → `gateway-session::host_migration_10_tests::messages::session_message_part_delete_is_idempotent_for_missing_part`
### session_ops_tests

- `session_handlers_are_available` → 原宿主模块保留。生命周期、状态、todo、跨项目变更拒绝、fork/reset/title 场景直接使用实际 AppState/运行时端口；turn_fork_tests 子模块由任务11拥有。本轮两个 summary 域用例迁出。后续可把 list/patch/todo 部分接入真实 session runtime fixture，但不能复制 AppState 到库。
- `task_session_summary_needs_refresh_only_for_empty_task_board_summaries` → `gateway-session::session_ops::host_migration_10_tests::task_session_summary_needs_refresh_only_for_empty_task_board_summaries`
- `refresh_task_session_summaries_preserves_summary_when_refresh_fails` → `gateway-session::session_ops::host_migration_10_tests::refresh_task_session_summaries_preserves_summary_when_refresh_fails`
- `gateway_first_response_create_without_title_reserves_an_unpublished_session` → 原宿主模块保留。生命周期、状态、todo、跨项目变更拒绝、fork/reset/title 场景直接使用实际 AppState/运行时端口；turn_fork_tests 子模块由任务11拥有。本轮两个 summary 域用例迁出。后续可把 list/patch/todo 部分接入真实 session runtime fixture，但不能复制 AppState 到库。
- `ui_session_list_respects_updated_boundary_and_zero_limit` → 原宿主模块保留。生命周期、状态、todo、跨项目变更拒绝、fork/reset/title 场景直接使用实际 AppState/运行时端口；turn_fork_tests 子模块由任务11拥有。本轮两个 summary 域用例迁出。后续可把 list/patch/todo 部分接入真实 session runtime fixture，但不能复制 AppState 到库。
- `ui_session_status_preserves_existing_non_idle_state` → 原宿主模块保留。生命周期、状态、todo、跨项目变更拒绝、fork/reset/title 场景直接使用实际 AppState/运行时端口；turn_fork_tests 子模块由任务11拥有。本轮两个 summary 域用例迁出。后续可把 list/patch/todo 部分接入真实 session runtime fixture，但不能复制 AppState 到库。
- `ui_session_patch_ignores_empty_fields_and_uses_header_scope` → 原宿主模块保留。生命周期、状态、todo、跨项目变更拒绝、fork/reset/title 场景直接使用实际 AppState/运行时端口；turn_fork_tests 子模块由任务11拥有。本轮两个 summary 域用例迁出。后续可把 list/patch/todo 部分接入真实 session runtime fixture，但不能复制 AppState 到库。
- `session_todo_put_replaces_and_clears_saved_items` → 原宿主模块保留。生命周期、状态、todo、跨项目变更拒绝、fork/reset/title 场景直接使用实际 AppState/运行时端口；turn_fork_tests 子模块由任务11拥有。本轮两个 summary 域用例迁出。后续可把 list/patch/todo 部分接入真实 session runtime fixture，但不能复制 AppState 到库。
- `scoped_mutations_do_not_modify_sessions_from_another_instance` → 原宿主模块保留。生命周期、状态、todo、跨项目变更拒绝、fork/reset/title 场景直接使用实际 AppState/运行时端口；turn_fork_tests 子模块由任务11拥有。本轮两个 summary 域用例迁出。后续可把 list/patch/todo 部分接入真实 session runtime fixture，但不能复制 AppState 到库。
- `session_ops_cover_lifecycle_filters_and_instance_boundaries` → 原宿主模块保留。生命周期、状态、todo、跨项目变更拒绝、fork/reset/title 场景直接使用实际 AppState/运行时端口；turn_fork_tests 子模块由任务11拥有。本轮两个 summary 域用例迁出。后续可把 list/patch/todo 部分接入真实 session runtime fixture，但不能复制 AppState 到库。
- `session_ops_cover_diff_summary_reset_and_title_errors` → 原宿主模块保留。生命周期、状态、todo、跨项目变更拒绝、fork/reset/title 场景直接使用实际 AppState/运行时端口；turn_fork_tests 子模块由任务11拥有。本轮两个 summary 域用例迁出。后续可把 list/patch/todo 部分接入真实 session runtime fixture，但不能复制 AppState 到库。
### shared_tests

- `session_api_error_maps_storage_not_found_to_not_found` → `gateway-session::host_migration_10_tests::shared::session_api_error_maps_storage_not_found_to_not_found`
- `session_api_error_maps_other_errors_to_bad_request` → `gateway-session::host_migration_10_tests::shared::session_api_error_maps_other_errors_to_bad_request`
- `session_api_error_maps_non_not_found_storage_errors_to_bad_request` → `gateway-session::host_migration_10_tests::shared::session_api_error_maps_non_not_found_storage_errors_to_bad_request`
- `has_explicit_directory_checks_query_before_headers` → `gateway-session::host_migration_10_tests::shared::has_explicit_directory_checks_query_before_headers`
- `has_explicit_directory_accepts_directory_header` → `gateway-session::host_migration_10_tests::shared::has_explicit_directory_accepts_directory_header`
- `has_explicit_directory_falls_back_to_header_when_query_is_blank` → `gateway-session::host_migration_10_tests::shared::has_explicit_directory_falls_back_to_header_when_query_is_blank`
- `has_explicit_directory_rejects_blank_query_and_header` → `gateway-session::host_migration_10_tests::shared::has_explicit_directory_rejects_blank_query_and_header`
- `has_explicit_directory_rejects_absent_query_and_header` → `gateway-session::host_migration_10_tests::shared::has_explicit_directory_rejects_absent_query_and_header`
- `has_explicit_directory_ignores_invalid_header_value` → `gateway-session::host_migration_10_tests::shared::has_explicit_directory_ignores_invalid_header_value`
- `ui_session_list_query_deserializes_defaults` → `gateway-session::host_migration_10_tests::shared::ui_session_list_query_deserializes_defaults`
- `ui_session_list_query_deserializes_empty_filters` → `gateway-session::host_migration_10_tests::shared::ui_session_list_query_deserializes_empty_filters`
- `ui_session_list_query_rejects_non_numeric_limit` → `gateway-session::host_migration_10_tests::shared::ui_session_list_query_rejects_non_numeric_limit`
- `ui_session_list_query_deserializes_all_filters` → `gateway-session::host_migration_10_tests::shared::ui_session_list_query_deserializes_all_filters`
- `ui_session_create_body_deserializes_flattened_session_and_permission` → `gateway-session::host_migration_10_tests::shared::ui_session_create_body_deserializes_flattened_session_and_permission`
- `ui_session_create_body_allows_permission_to_be_absent` → `gateway-session::host_migration_10_tests::shared::ui_session_create_body_allows_permission_to_be_absent`
- `ui_session_create_body_deserializes_empty_request` → `gateway-session::host_migration_10_tests::shared::ui_session_create_body_deserializes_empty_request`
- `ui_session_create_body_rejects_unknown_permission_action` → `gateway-session::host_migration_10_tests::shared::ui_session_create_body_rejects_unknown_permission_action`
- `resolve_scope_from_query_returns_scope_for_directory_query` → 原宿主模块保留。只留两条实际 Host scope 解析和一条真实项目上下文回调；17条 DTO/错误/目录判断域用例全部迁移。
- `resolve_scope_from_query_returns_scope_for_directory_header` → 原宿主模块保留。只留两条实际 Host scope 解析和一条真实项目上下文回调；17条 DTO/错误/目录判断域用例全部迁移。
- `with_tenant_instance_establishes_directory_context_and_returns_value` → 原宿主模块保留。只留两条实际 Host scope 解析和一条真实项目上下文回调；17条 DTO/错误/目录判断域用例全部迁移。

## 生产符号逐项与依赖方向

真实生产路径仍是宿主 router → 具体 typed handler → gateway-session::{acp_options,message_ops,session_ops,archived_routes,shared} → 明确 Host 端口或 session-engine；没有新增反向 vw-agent 依赖。下表列出每个原定义（含端口方法），均保留具体装配，非本轮生产迁移收益。

- `acp_options.rs`: `annotate_model_source`, `read_options`, `select_option`, `select_model_source`, `load_binding`, `save_binding`, `fork_started`, `agent`, `get`, `set`, `set_source`, `source_capabilities`。AcpHost 的真实 ACP 进程调用、binding 持久化、Agent catalog 装配及 HTTP DTO 转换；生产策略此前已在 gateway-session/acp_options.rs，本轮不重复计收益。
- `archived_delete.rs`: `delete`。唯一 delete 是 State<AppState> 的 typed route assembly；删除算法已经在领域库。
- `message_ops.rs`: `current_tenant_scope`, `resolve_directory`, `with_tenant_instance`, `session_message_list`, `session_message_get`, `session_message_part_delete`, `session_message_part_patch`。MessageHost 实现和具体处理器转发保留以适配宿主 InstanceQuery；真实领域消息处理已在 message_ops 中，不新增迁出生产收益。
- `raw_commit.rs`: （薄重导出，无函数）。acknowledge/claim/register 为实际 UI/stream 共享 registry 薄重导出，仍有调用者。
- `session_ops.rs`: `source_directory`, `persist`, `with_tenant_scope`, `status_list`, `is_successful_finish_reason`, `first_response_source`, `resolve_scope`, `has_committed_first_response`, `deletion_candidates`, `ensure_ready`, `create_session`, `reserve_session`, `load_session`, `get`, `get_any`, `children`, `remove`, `remove_any`, `list_all`, `list_for_project`, `remove_archived_locked`, `update`, `update_any`, `fork`, `refresh_summary`, `todo_get`, `todo_update`, `cleanup_after_message`, `diff`, `process_session`, `generate_title`, `resolve_full_access`, `invalidate_engine`, `fork_engine`, `prepare_fork`, `create_checkout`, `remove_checkout`, `metadata_lock`, `ui_session_list`, `ui_session_status`, `ui_session_get`, `session_full_access_put`, `ui_session_create`, `ui_session_patch`, `ui_session_delete`, `session_children`, `session_todo_get`, `session_todo_put`, `session_fork`, `session_reset`, `session_diff`, `session_summarize`, `session_title_generate`。真实 SessionHost 端口、PreparedFork、State<AppState> 和 typed forwarding。CRUD/fork/todo 策略已在领域库；本轮只迁 summary 测试及移除测试专用 import。
- `shared.rs`: `has_explicit_directory`, `resolve_scope_from_query`, `with_tenant_instance`。with_tenant_instance 是真实租户/项目检查与宿主 bootstrap；resolve_scope_from_query 转发实际 Host。其余重导出用于既有调用。

## 注册、夹具与未来验证

message_ops 的原挂载仅承接签名契约；全部9条消息行为用例在库自注册。shared_tests 保留3条实际端口用例，17条纯领域测试不再宿主挂载；summary两条通过领域私有模块注册。新增测试依赖 agent-storage/agent-gateway/tempfile/uuid 均为已有 workspace 依赖，Cargo.lock 仅 gateway-session 项增加引用。MessageTestHost 只替换进程 bootstrap，使用实际 agent-project::instance::provide、agent-storage 与 session-engine 消息算法；真实宿主 bootstrap 由保留 scope/acceptance 测试承接。沿用消息测试原 Once 环境初始化与互斥锁，唯一 session ID 和清理；不复制 AppState。

未来：`python3 scripts/cargo_slot.py cargo test -p gateway-session --lib host_migration_10_tests -- --test-threads=1`；静态预期 28 条，须取得 ran>0、failed=0。未运行 Cargo metadata/list/fmt/check/test、测试执行文件、实机或构建；无退出码/通过数/构建耗时收益证据。

宿主未来对上列保留函数逐项使用原完整路径 `gateway::api::handlers::session::<原模块>::<函数>` 精确过滤；nested fork_acp 的原挂载是 session_ops::session_ops_tests::turn_fork_tests::fork_acp_tests（以未来 list 核实）。迁走用例的旧路径须用真实宿主 list 证明消失，零测试不算通过。

静态检查：测试函数名逐项对应、原断言保留、无 path/include 装载宿主源码、无 vw-agent 依赖；git diff --check 仅检查本任务补丁。按 scope 文件所有权没有修改任务11的 stream/ui/turn_fork 源文件。
