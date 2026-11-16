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

## 收口恢复：生命周期完整断言继续下沉（覆盖前述保留说明）

读取 final.md 和 remaining-host.json 任务10条目后，拆开原 session_ops_tests 的实际宿主装配和领域断言。新增 gateway-session::host_lifecycle_acceptance，使用已有 SessionHost 及 H::State；八个完整行为函数迁入领域库。原宿主同名测试仅构造实际 AppState，调用对应领域函数并注入实际 Host。没有复制 AppState、伪造存储、重写策略或删除失败断言。

领域 test-support feature 只有实际 vw-agent dev-dependency 选择，沿用仓库其它领域库的已有模式；tempfile 和 vw-api-todo 是可选的具体夹具依赖，默认生产不启用。没有新增 Host 方法、没有反向依赖 vw-agent、没有 path/include 加载宿主源代码。领域函数调用生产 session_ops::<H>，原宿主函数调用本来就是相同入口的 typed forwarding。

共享 Once 环境和 SESSION_OPS_TEST_LOCK 一并迁至领域测试支持模块，由原 turn_fork_tests/fork_acp_tests 通过父模块导入同一个对象；未创建第二把锁或改变锁顺序。宿主保留 query_for/create_session_in 是 task11 子模块和 task10 fork_acp 的真实夹具调用，不复制 AppState 到库。第一个用例的 global_async_env_lock 仍在宿主薄测试获取，其余夹具隔离行为保持原状。

完整映射（原宿主 session_ops::session_ops_tests 下注册名不变；下列目标均为 gateway-session::host_lifecycle_acceptance 下同名函数，不声称是可独立运行的领域单测）：
- `gateway_first_response_create_without_title_reserves_an_unpublished_session` → `gateway-session::host_lifecycle_acceptance::gateway_first_response_create_without_title_reserves_an_unpublished_session::<实际Host>(实际AppState)`。
- `ui_session_list_respects_updated_boundary_and_zero_limit` → `gateway-session::host_lifecycle_acceptance::ui_session_list_respects_updated_boundary_and_zero_limit::<实际Host>(实际AppState)`。
- `ui_session_status_preserves_existing_non_idle_state` → `gateway-session::host_lifecycle_acceptance::ui_session_status_preserves_existing_non_idle_state::<实际Host>(实际AppState)`。
- `ui_session_patch_ignores_empty_fields_and_uses_header_scope` → `gateway-session::host_lifecycle_acceptance::ui_session_patch_ignores_empty_fields_and_uses_header_scope::<实际Host>(实际AppState)`。
- `session_todo_put_replaces_and_clears_saved_items` → `gateway-session::host_lifecycle_acceptance::session_todo_put_replaces_and_clears_saved_items::<实际Host>(实际AppState)`。
- `scoped_mutations_do_not_modify_sessions_from_another_instance` → `gateway-session::host_lifecycle_acceptance::scoped_mutations_do_not_modify_sessions_from_another_instance::<实际Host>(实际AppState)`。
- `session_ops_cover_lifecycle_filters_and_instance_boundaries` → `gateway-session::host_lifecycle_acceptance::session_ops_cover_lifecycle_filters_and_instance_boundaries::<实际Host>(实际AppState)`。
- `session_ops_cover_diff_summary_reset_and_title_errors` → `gateway-session::host_lifecycle_acceptance::session_ops_cover_diff_summary_reset_and_title_errors::<实际Host>(实际AppState)`。

本轮该宿主文件 694→138 行，新增领域测试支持 611 行，额外净减 556 行；原14文件基线4200行，累计净减 1275 行。28条原纯领域单测注册不变，新增8条完整场景的断言/执行函数迁移，宿主38条注册数量不变（其中8条成为薄注入入口）。生产策略本轮迁移0行，新增feature支持不能算作生产策略收益。

静态比对八个场景的断言宏数量及顺序一致（50处），类型路径改为真实所属库、函数调用显式传入 H；所有安全拒绝/跨项目/错误/消息边界断言保留。未执行任何 Cargo、构建或测试。未来通过原完整宿主名逐项精确运行这8条；未来check同时覆盖 gateway-session 的 test-support feature。旧的“整个生命周期用例必须保留宿主”说明由本节替代。其余文件逐符号与全测试映射仍见上文；当前累计减量仍不足70%，不能据此宣称整轮达标。

## 第三轮收口恢复：profile 与 ACP source 完整主体迁移

读取最新 final.md（51915行）后，按 profile serialization/default/backend projection 审查点，将整个 profile 场景主体、受控外部peer、配置/回执夹具、恢复子进程、持久化断言及撤权场景归入 gateway-session::profile_acceptance。拆成根模块、peer.rs、onboarding.rs；宿主只有真实ProfileAcceptanceHost实现及15条同名注册。新trait仅在test-support下，继承已有SessionHost/AcpHost，显式注入真实恢复配置、锁、请求配置、路由、缓存失效、chat与persist_turn；Admission使用原AdmissionHost关联类型，领域直接执行真实session_engine::admission::prepare。没有AppState复制或测试算法替身。

ACP source两个场景主体归入acp_source_acceptance，由宿主传入原三个typed路由组成的Router。真实回环HTTP和持久化逻辑仍运行，Host::get/prepare_fork及AcpHost的load_binding/save_binding/agent直接引用原生产端口，fork首回合persist_turn调用原宿主流持久化入口。没有用模拟端口代替租户或权限验收。

前述“保留整个profile/acp_source宿主文件”的理由在本节废止。注册数量、原ignore文案和硬编码子进程精确测试名保持；测试支持函数不冒充独立领域#[test]注册，不宣称编译负担/耗时收益。原完整注册名到目标逐项如下（宿主路径不变，目标函数名称相同）：
- `agent_profile_acceptance_runs_selected_profile_end_to_end` → `gateway-session::profile_acceptance::agent_profile_acceptance_runs_selected_profile_end_to_end::<实际Host>`。
- `agent_profile_acceptance_rejects_revoked_capabilities` → `gateway-session::profile_acceptance::agent_profile_acceptance_rejects_revoked_capabilities::<实际Host>`。
- `agent_profile_acceptance_catalog_uses_request_tenant` → `gateway-session::profile_acceptance::agent_profile_acceptance_catalog_uses_request_tenant::<实际Host>`。
- `agent_config_cleanup_global_profile_roundtrips_to_new_session` → `gateway-session::profile_acceptance::agent_config_cleanup_global_profile_roundtrips_to_new_session::<实际Host>`。
- `acp_onboarding_autohand_builtin_profile_persists_and_runs` → `gateway-session::profile_acceptance::acp_onboarding_autohand_builtin_profile_persists_and_runs::<实际Host>`。
- `acp_onboarding_deepagents_builtin_profile_persists_and_runs` → `gateway-session::profile_acceptance::acp_onboarding_deepagents_builtin_profile_persists_and_runs::<实际Host>`。
- `acp_onboarding_dimcode_builtin_profile_persists_and_runs` → `gateway-session::profile_acceptance::acp_onboarding_dimcode_builtin_profile_persists_and_runs::<实际Host>`。
- `acp_onboarding_dirac_builtin_profile_persists_and_runs` → `gateway-session::profile_acceptance::acp_onboarding_dirac_builtin_profile_persists_and_runs::<实际Host>`。
- `acp_onboarding_glm_acp_agent_builtin_profile_persists_and_runs` → `gateway-session::profile_acceptance::acp_onboarding_glm_acp_agent_builtin_profile_persists_and_runs::<实际Host>`。
- `acp_onboarding_grok_builtin_profile_persists_and_runs` → `gateway-session::profile_acceptance::acp_onboarding_grok_builtin_profile_persists_and_runs::<实际Host>`。
- `acp_onboarding_nova_builtin_profile_persists_and_runs` → `gateway-session::profile_acceptance::acp_onboarding_nova_builtin_profile_persists_and_runs::<实际Host>`。
- `acp_onboarding_nova_real_builtin_profile_persists_and_runs` → `gateway-session::profile_acceptance::acp_onboarding_nova_real_builtin_profile_persists_and_runs::<实际Host>`。
- `acp_onboarding_sigit_builtin_profile_persists_and_runs` → `gateway-session::profile_acceptance::acp_onboarding_sigit_builtin_profile_persists_and_runs::<实际Host>`。
- `acp_onboarding_sigit_real_builtin_profile_persists_and_runs` → `gateway-session::profile_acceptance::acp_onboarding_sigit_real_builtin_profile_persists_and_runs::<实际Host>`。
- `acp_onboarding_amp_acp_builtin_profile_persists_and_runs` → `gateway-session::profile_acceptance::acp_onboarding_amp_acp_builtin_profile_persists_and_runs::<实际Host>`。
- `acp_model_source_ui_http_persists_and_rejects_without_local_fallback` → `gateway-session::acp_source_acceptance::acp_model_source_ui_http_persists_and_rejects_without_local_fallback::<实际Host>`。
- `acp_public_permission_options_reject_direct_put_and_chat_over_http` → `gateway-session::acp_source_acceptance::acp_public_permission_options_reject_direct_put_and_chat_over_http::<实际Host>`。

`agent_profile_acceptance_tests.rs` 1082→67行，保留原真实Host注入/注册；完整执行和断言已迁移。

`acp_source_tests.rs` 253→20行，保留原真实Host注入/注册；完整执行和断言已迁移。

静态断言宏数量：profile 61→61，ACP source 17→17；拆分未删除断言。contracts.json相对新manifest改为../acp-agent-registry，Nova peer改为../../vw-acp/tests/fixtures，文件均存在。新增可选测试依赖只在现有test-support启用；Cargo.lock仅补实际库依赖。全部Cargo、编译、构建、测试和实机检查未运行。未来继续使用原17个完整宿主注册名精确过滤，ignored真实运行包场景需原授权和环境。剩余fork/archived场景及生产薄转发仍按前述逐符号清单保留，未作为本轮迁移收益。

第三轮继续迁出 archived_delete_tests 的四个完整场景及 create/archive/read/save_ui/has_ui/delete 夹具至 archived_acceptance，原 persistence 下四条注册仅注入实际租户Router工厂和实际SQLite路径。create/reserve/update_any使用同一个真实SessionHost，UI读取写入使用实际session-store/agent-database，存储枚举使用agent-storage。注入真实Router保留HTTP鉴权/租户拒绝，触发器仍在同一个实际数据库注入删除失败，不模拟事务或history锁。tokio/net/time/rt-multi-thread仅由test-support启用；rusqlite保留Windows bundled差异。

四条映射：
- `persistence::archived_chat_delete_strict_enumeration_rejects_a_file_prefix` → `gateway-session::archived_acceptance::archived_chat_delete_strict_enumeration_rejects_a_file_prefix::<实际Host>(真实Router工厂,实际数据库路径)`。
- `persistence::archived_chat_delete_http_removes_multilevel_tree_and_preserves_other_scopes` → `gateway-session::archived_acceptance::archived_chat_delete_http_removes_multilevel_tree_and_preserves_other_scopes::<实际Host>(真实Router工厂,实际数据库路径)`。
- `persistence::archived_chat_delete_http_rejects_live_root_and_restore_while_waiting_for_history` → `gateway-session::archived_acceptance::archived_chat_delete_http_rejects_live_root_and_restore_while_waiting_for_history::<实际Host>(真实Router工厂,实际数据库路径)`。
- `persistence::archived_chat_delete_storage_failure_keeps_root_and_allows_retry` → `gateway-session::archived_acceptance::archived_chat_delete_storage_failure_keeps_root_and_allows_retry::<实际Host>(真实Router工厂,实际数据库路径)`。

archived_delete_tests.rs 254→31行；静态断言宏数量 26→26。该文件旧整文件保留理由被本节替代。未运行测试、编译或Cargo。

相邻静态修复：提交前发现并行已提交的gateway-session清单把依赖插入serde_json和target表头中间（serde_jsonparking_lot / cfllm-runtime片段），本轮恢复完整表头及原依赖，保留已提交的llm-runtime/parking_lot测试依赖；没有执行Cargo解析。

第三轮提交后静态快照（splitlines，14来源原基线4200）：当前1454行，累计净减2746行。逐文件当前数：acp_options=44, acp_source_tests=20, agent_profile_acceptance_tests=67, archived_delete=7, archived_delete_tests=31, fork_acceptance_tests=506, fork_acp_tests=288, message_ops=72, message_ops_tests=10, raw_commit=2, session_ops=174, session_ops_tests=138, shared=26, shared_tests=69。未达到70%时仍按保留项审查，不宣称整轮达标。

明确继续可迁项：fork_acceptance_tests.rs的Branch/Prefix及assert_continuation/native_model_history/seed_checkpoints等仍在原源，其读端点、模型运输和worktree恢复已是具体组合，可再通过现有SessionHost/StreamHost与Router注入拆分；fork_acp_tests.rs仍由任务11的turn_fork_tests父模块提供fork/send/create_session_in上下文，目前父夹具正在并行下沉fork_acceptance，继续迁移必须对齐其实际新签名，不能复制旧父夹具或伪造接线。本轮已闭合profile/acp_source/archive三族，未把这两项写成已完成迁移。
