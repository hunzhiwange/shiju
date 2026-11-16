# 任务44：video_edit 职责迁移证据

代码完成，编译/行为未验证。遵守用户禁令，未执行任何 Cargo（含 fmt/check/list）、构建、单元、实机测试或测试程序。仅源码、依赖、挂载、Git 差异静态核对。

原文件族 3818 行 → 2824 行，净减 994 行（26.0%）。迁移 44 个完整测试，宿主保留 38 个完整边界测试；不存在删除或重复挂载的测试名。

## 生产方向与接线

本轮生产算法迁出收益为 0：原业务已在 workflow-video-edit。不得将本轮测试迁移或既有下沉计算为生产成果。宿主 command/prepare/cloud/planning/entry 的真实调用仍进入 workflow_video_edit 对应领域入口；域库依赖 workflow-core、tool-context、agent-database 等端口，不依赖 vw-agent，不 path/include 宿主源码。新增 lib.rs 自注册 host_migration_44_tests；模板测试直接使用 workflow_core::templates::VideoEditTemplateSeed 与 workflow-template-contract 正式资源，声明 dev 依赖并同步锁文件单项。

领域夹具 TestContext 仅为本地 workspace 的显式 VideoContext 实现，真实 SQLite 和文件仍被使用；不会执行宿主命令或替代宿主鉴权验收。原生安全、审批、租户、问答、网关事件断言全部保留。

## 每个来源文件去向与保留职责

| 来源 | 原行数 | 现宿主行数 | 领域接收与保留原因 |
|---|---:|---:|---|
| `cloud_assets_tests.rs` | 60 | 60 | 原生 CLI 分页任务核对与引用文件读取边界。 |
| `e2e_tests.rs` | 479 | 479 | 真实网关流、会话持久化、租户、模板加载、取消与凭证隔离，不可用领域假端口替代。 |
| `entry_contract.rs` | 2 | 2 | 默认模板与 validate 的兼容重导出，供宿主模板契约入口。 |
| `types.rs` | 2 | 2 | 仅既有领域模块薄重导出，维持真实调用者兼容路径。 |
| `cloud_delivery_tests.rs` | 213 | 213 | 原生 CLI 的项目写入顺序、媒体探测、票据和交付回读协议。 |
| `budget.rs` | 2 | 2 | 仅既有领域模块薄重导出，维持真实调用者兼容路径。 |
| `segments.rs` | 2 | 2 | 仅既有领域模块薄重导出，维持真实调用者兼容路径。 |
| `transcript_tests.rs` | 262 | 65 | 迁入 `host_migration_44_tests/transcript_tests.rs`；仅既有领域模块薄重导出，维持真实调用者兼容路径。 |
| `assets_tests.rs` | 452 | 45 | 迁入 `host_migration_44_tests/assets_tests.rs`；仅保留原生 cloud_assets_tests 使用的真实账本/素材请求 fixture，删除未使用的 HTTP Endpoint/Remote 夹具。 |
| `command_tests.rs` | 130 | 73 | 迁入 `host_migration_44_tests/command_tests.rs`；保留三个原生策略契约：symlink/full-access、额外授权根和无项目、委派父策略约束。 |
| `command.rs` | 1 | 1 | 仅既有领域模块薄重导出，维持真实调用者兼容路径。 |
| `cloud.rs` | 26 | 26 | visible_shiju_root 从当前已授权 Skill 集合选择唯一来源、校验 SKILL.md 路径；discover 注入原生上下文。 |
| `manifest.rs` | 2 | 2 | 仅既有领域模块薄重导出，维持真实调用者兼容路径。 |
| `entry_tests.rs` | 72 | 0 | 迁入 `host_migration_44_tests/entry_tests.rs`；宿主旧挂载与文件已删除。 |
| `transcript.rs` | 1 | 1 | 仅既有领域模块薄重导出，维持真实调用者兼容路径。 |
| `cloud_failure.rs` | 1 | 1 | 仅既有领域模块薄重导出，维持真实调用者兼容路径。 |
| `preview.rs` | 2 | 2 | 仅既有领域模块薄重导出，维持真实调用者兼容路径。 |
| `cloud_tests.rs` | 384 | 384 | 原生 Skill 授权来源、允许工具、路径/符号链接、审批拒绝、hook 审计与实际子进程调用；fixture 供原生适配器族使用。 |
| `merge.rs` | 2 | 2 | 仅既有领域模块薄重导出，维持真实调用者兼容路径。 |
| `apply_journal.rs` | 2 | 2 | 仅既有领域模块薄重导出，维持真实调用者兼容路径。 |
| `prepare.rs` | 61 | 61 | ToolRuntimeContext 实现 VideoContext；路径策略、租户、工具执行、真实 question UI 与取消适配。 |
| `mod.rs` | 71 | 69 | 迁入 `host_migration_44_tests/mod.rs`；领域公开端口重导出与真实宿主模块/测试注册；删除 entry_tests 旧挂载。 |
| `planning_tests.rs` | 184 | 219 | 迁入 `host_migration_44_tests/planning_tests.rs`；保留真实 WorkflowRuntime/Provider 图执行、共享模型预算、失败与上下文上限三个测试；补齐任务1迁走 merge_tests 后断开的 source/effect 数据夹具。 |
| `apply.rs` | 2 | 2 | 仅既有领域模块薄重导出，维持真实调用者兼容路径。 |
| `entry.rs` | 66 | 66 | NativeVideoEntryHost::database/services 读取宿主目录、当前工具上下文、技能和 SessionLlmProvider；build 组装真实 WorkflowRuntime。 |
| `assets.rs` | 2 | 2 | 仅既有领域模块薄重导出，维持真实调用者兼容路径。 |
| `e2e_test_cloud.rs` | 371 | 371 | 仅供真实网关 e2e 使用的外部云端 HTTP 夹具，包含工具调用和事件协议。 |
| `cloud_prepare_tests.rs` | 116 | 116 | 原生官方脚本 schema、真实命令执行、上传票据隐私与文件上传边界。 |
| `planning.rs` | 11 | 11 | 将具体 WorkflowRuntime 与 run_workflow 注入既有领域规划入口，保留真实调用者的宿主组合。 |
| `entry_test_transport.rs` | 257 | 257 | 按宿主 session/tenant 安装与释放外部模型/云端测试传输，仅 cfg(test)。 |
| `manifest_tests.rs` | 134 | 32 | 迁入 `host_migration_44_tests/manifest_tests.rs`；仅保留原生资产/交付夹具使用的 candidate 数据构造，无领域测试挂载。 |
| `prepare_tests.rs` | 446 | 254 | 迁入 `host_migration_44_tests/prepare_tests.rs`；保留真实问答选择/取消、原生哈希安全与取消、当前租户隔离三个测试及它们使用的 Cloud/Remote/fixture。 |

净减不足70%：以上逐文件列明原因。继续可迁项是 cloud_prepare/cloud_assets/cloud_delivery 中脱离原生命令执行的响应规约断言，以及 e2e 中脱离真实会话流的领域状态断言；需逐场景拆分，不能整体搬入假宿主。当前保留完整真实边界，未用删除断言凑比例；收口须独立验收减量缺口。

## 全测试逐项映射

| 原测试 | 唯一现挂载文件 |
|---|---|
| `cloud_assets_tests.rs::video_edit_asset_jobs_native_adapter_reconciles_pages_and_reads_reference` | `crates/vw-agent/src/workflow/video_edit/cloud_assets_tests.rs` |
| `e2e_tests.rs::video_edit_e2e_long_transcripts_reach_done_and_reload` | `crates/vw-agent/src/workflow/video_edit/e2e_tests.rs` |
| `e2e_tests.rs::video_edit_e2e_quota_invalid_segment_and_fake_preview_never_done` | `crates/vw-agent/src/workflow/video_edit/e2e_tests.rs` |
| `e2e_tests.rs::video_edit_e2e_accepted_timeout_reconciles_without_duplicate_post` | `crates/vw-agent/src/workflow/video_edit/e2e_tests.rs` |
| `e2e_tests.rs::video_edit_e2e_independent_requests_have_distinct_run_budgets` | `crates/vw-agent/src/workflow/video_edit/e2e_tests.rs` |
| `e2e_tests.rs::video_edit_e2e_native_permission_denial_prevents_external_io` | `crates/vw-agent/src/workflow/video_edit/e2e_tests.rs` |
| `e2e_tests.rs::video_edit_e2e_final_save_failure_never_done` | `crates/vw-agent/src/workflow/video_edit/e2e_tests.rs` |
| `e2e_tests.rs::video_edit_e2e_cancelled_restart_reconciles_and_never_resubmits` | `crates/vw-agent/src/workflow/video_edit/e2e_tests.rs` |
| `e2e_tests.rs::video_edit_e2e_template_edit_changes_next_native_model_request` | `crates/vw-agent/src/workflow/video_edit/e2e_tests.rs` |
| `cloud_delivery_tests.rs::video_edit_delivery_normalizes_live_media_probe_projection_without_losing_audio` | `crates/vw-agent/src/workflow/video_edit/cloud_delivery_tests.rs` |
| `cloud_delivery_tests.rs::video_edit_delivery_native_adapter_applies_three_types_and_reads_real_contract_shapes` | `crates/vw-agent/src/workflow/video_edit/cloud_delivery_tests.rs` |
| `cloud_delivery_tests.rs::video_edit_delivery_native_adapter_rejects_partial_caption_failure_and_pending_draft` | `crates/vw-agent/src/workflow/video_edit/cloud_delivery_tests.rs` |
| `cloud_delivery_tests.rs::video_edit_delivery_native_adapter_records_verified_empty_cleanup_separately` | `crates/vw-agent/src/workflow/video_edit/cloud_delivery_tests.rs` |
| `cloud_delivery_tests.rs::video_edit_delivery_native_finalize_accepts_observed_complete_import_receipt` | `crates/vw-agent/src/workflow/video_edit/cloud_delivery_tests.rs` |
| `transcript_tests.rs::video_edit_transcript_preserves_facts_silence_and_fractional_fps` | `crates/libs/workflow-video-edit/src/host_migration_44_tests/transcript_tests.rs` |
| `transcript_tests.rs::video_edit_transcript_rejects_invalid_times_and_duplicate_indices` | `crates/libs/workflow-video-edit/src/host_migration_44_tests/transcript_tests.rs` |
| `transcript_tests.rs::video_edit_transcript_rejects_silent_failed_empty_and_unconfirmed_punctuation` | `crates/libs/workflow-video-edit/src/host_migration_44_tests/transcript_tests.rs` |
| `transcript_tests.rs::video_edit_transcript_concurrent_resume_reuses_one_job_and_persisted_result` | `crates/libs/workflow-video-edit/src/host_migration_44_tests/transcript_tests.rs` |
| `transcript_tests.rs::video_edit_transcript_errors_never_resubmit_or_produce_fake_speech` | `crates/libs/workflow-video-edit/src/host_migration_44_tests/transcript_tests.rs` |
| `transcript_tests.rs::video_edit_transcript_rejects_invalid_media_and_segment_order` | `crates/libs/workflow-video-edit/src/host_migration_44_tests/transcript_tests.rs` |
| `transcript_tests.rs::video_edit_transcript_native_adapter_keeps_audio_ticket_private_and_queries_exact_task` | `crates/vw-agent/src/workflow/video_edit/transcript_tests.rs` |
| `assets_tests.rs::video_edit_asset_jobs_unknown_is_not_ready` | `crates/libs/workflow-video-edit/src/host_migration_44_tests/assets_tests.rs` |
| `assets_tests.rs::video_edit_asset_jobs_hold_five_until_reverse_completion` | `crates/libs/workflow-video-edit/src/host_migration_44_tests/assets_tests.rs` |
| `assets_tests.rs::video_edit_asset_jobs_restart_reconciles_lost_receipt_without_post` | `crates/libs/workflow-video-edit/src/host_migration_44_tests/assets_tests.rs` |
| `assets_tests.rs::video_edit_asset_jobs_refusal_retries_share_peak_and_bound_attempts` | `crates/libs/workflow-video-edit/src/host_migration_44_tests/assets_tests.rs` |
| `assets_tests.rs::video_edit_asset_jobs_cancel_retains_remote_slots_and_stops_dispatch` | `crates/libs/workflow-video-edit/src/host_migration_44_tests/assets_tests.rs` |
| `assets_tests.rs::video_edit_asset_jobs_failure_or_unreadable_result_blocks_registry` | `crates/libs/workflow-video-edit/src/host_migration_44_tests/assets_tests.rs` |
| `assets_tests.rs::video_edit_asset_jobs_share_model_budget_and_resume_unknown` | `crates/libs/workflow-video-edit/src/host_migration_44_tests/assets_tests.rs` |
| `assets_tests.rs::video_edit_asset_jobs_unknown_missing_from_remote_is_not_resubmitted` | `crates/libs/workflow-video-edit/src/host_migration_44_tests/assets_tests.rs` |
| `assets_tests.rs::video_edit_asset_jobs_failed_attempt_retries_only_after_terminal_and_keeps_id` | `crates/libs/workflow-video-edit/src/host_migration_44_tests/assets_tests.rs` |
| `assets_tests.rs::video_edit_asset_jobs_deduplicate_content_parameters_and_scope_without_guessing_recipe` | `crates/libs/workflow-video-edit/src/host_migration_44_tests/assets_tests.rs` |
| `assets_tests.rs::video_edit_asset_jobs_restart_does_not_resume_cancelled_dispatch` | `crates/libs/workflow-video-edit/src/host_migration_44_tests/assets_tests.rs` |
| `command_tests.rs::video_edit_command_parses_quoted_chinese_and_windows_paths` | `crates/libs/workflow-video-edit/src/host_migration_44_tests/command_tests.rs` |
| `command_tests.rs::video_edit_command_validates_real_paths_without_binding` | `crates/libs/workflow-video-edit/src/host_migration_44_tests/command_tests.rs` |
| `command_tests.rs::video_edit_command_rejects_invalid_files_and_ambiguous_input` | `crates/libs/workflow-video-edit/src/host_migration_44_tests/command_tests.rs` |
| `command_tests.rs::video_edit_command_symlink_obeys_native_security_and_full_access` | `crates/vw-agent/src/workflow/video_edit/command_tests.rs` |
| `command_tests.rs::video_edit_command_shell_characters_remain_literal` | `crates/libs/workflow-video-edit/src/host_migration_44_tests/command_tests.rs` |
| `command_tests.rs::video_edit_command_external_allowed_root_and_no_project` | `crates/vw-agent/src/workflow/video_edit/command_tests.rs` |
| `command_tests.rs::video_edit_command_delegated_parent_remains_restricted_under_full_access` | `crates/vw-agent/src/workflow/video_edit/command_tests.rs` |
| `entry_tests.rs::video_edit_entry_rejects_uncontrolled_graph_before_storage` | `crates/libs/workflow-video-edit/src/host_migration_44_tests/entry_tests.rs` |
| `entry_tests.rs::video_edit_entry_distributes_production_graph` | `crates/libs/workflow-video-edit/src/host_migration_44_tests/entry_tests.rs` |
| `entry_tests.rs::video_edit_entry_preserves_menu_and_unrelated_commands` | `crates/libs/workflow-video-edit/src/host_migration_44_tests/entry_tests.rs` |
| `entry_tests.rs::video_edit_entry_rejects_capability_order_and_fake_ready_edits` | `crates/libs/workflow-video-edit/src/host_migration_44_tests/entry_tests.rs` |
| `entry_tests.rs::video_edit_entry_accepts_business_prompt_model_and_lower_budget` | `crates/libs/workflow-video-edit/src/host_migration_44_tests/entry_tests.rs` |
| `cloud_tests.rs::video_edit_cloud_transport_hidden_skill_is_rejected` | `crates/vw-agent/src/workflow/video_edit/cloud_tests.rs` |
| `cloud_tests.rs::video_edit_cloud_transport_discovers_current_instance_and_full_private_receipt` | `crates/vw-agent/src/workflow/video_edit/cloud_tests.rs` |
| `cloud_tests.rs::video_edit_cloud_transport_version_source_and_authorization_fail_closed` | `crates/vw-agent/src/workflow/video_edit/cloud_tests.rs` |
| `cloud_tests.rs::video_edit_cloud_transport_missing_credentials_never_logs_in_or_falls_back` | `crates/vw-agent/src/workflow/video_edit/cloud_tests.rs` |
| `cloud_tests.rs::video_edit_cloud_transport_native_tool_and_path_denials_prevent_process` | `crates/vw-agent/src/workflow/video_edit/cloud_tests.rs` |
| `cloud_tests.rs::video_edit_cloud_transport_cancel_before_discovery_starts_no_process` | `crates/vw-agent/src/workflow/video_edit/cloud_tests.rs` |
| `cloud_tests.rs::video_edit_cloud_transport_errors_and_unknown_states_are_not_success` | `crates/vw-agent/src/workflow/video_edit/cloud_tests.rs` |
| `cloud_tests.rs::video_edit_cloud_transport_duplicate_visible_instances_are_ambiguous` | `crates/vw-agent/src/workflow/video_edit/cloud_tests.rs` |
| `cloud_tests.rs::video_edit_cloud_transport_no_project_uses_native_workspace` | `crates/vw-agent/src/workflow/video_edit/cloud_tests.rs` |
| `cloud_tests.rs::video_edit_cloud_transport_inflight_cancel_stops_process_before_private_cleanup` | `crates/vw-agent/src/workflow/video_edit/cloud_tests.rs` |
| `cloud_tests.rs::video_edit_cloud_transport_native_approval_denial_prevents_process` | `crates/vw-agent/src/workflow/video_edit/cloud_tests.rs` |
| `cloud_tests.rs::video_edit_cloud_transport_native_hooks_never_receive_private_payloads` | `crates/vw-agent/src/workflow/video_edit/cloud_tests.rs` |
| `planning_tests.rs::video_edit_plan_json_rejects_boolean_time_and_unstructured_assets` | `crates/libs/workflow-video-edit/src/host_migration_44_tests/planning_tests.rs` |
| `planning_tests.rs::video_edit_plan_real_iteration_six_segments_bounded_requests_and_shared_budget` | `crates/vw-agent/src/workflow/video_edit/planning_tests.rs` |
| `planning_tests.rs::video_edit_plan_failed_segment_never_returns_partial_plan` | `crates/vw-agent/src/workflow/video_edit/planning_tests.rs` |
| `planning_tests.rs::video_edit_plan_context_overflow_is_explicit_before_model_request` | `crates/vw-agent/src/workflow/video_edit/planning_tests.rs` |
| `cloud_prepare_tests.rs::video_edit_prepare_native_receipts_keep_signed_tickets_in_memory` | `crates/vw-agent/src/workflow/video_edit/cloud_prepare_tests.rs` |
| `cloud_prepare_tests.rs::video_edit_prepare_cloud_identity_is_independent_of_local_tenant_and_rejects_mismatch` | `crates/vw-agent/src/workflow/video_edit/cloud_prepare_tests.rs` |
| `manifest_tests.rs::video_edit_prepare_ledger_claims_without_project_binding` | `crates/libs/workflow-video-edit/src/host_migration_44_tests/manifest_tests.rs` |
| `manifest_tests.rs::video_edit_prepare_ledger_fences_stale_writers_and_tenants` | `crates/libs/workflow-video-edit/src/host_migration_44_tests/manifest_tests.rs` |
| `manifest_tests.rs::video_edit_prepare_ledger_postgres_contract` | `crates/libs/workflow-video-edit/src/host_migration_44_tests/manifest_tests.rs` |
| `manifest_tests.rs::video_edit_prepare_ledger_competing_connections_claim_once_and_restart` | `crates/libs/workflow-video-edit/src/host_migration_44_tests/manifest_tests.rs` |
| `manifest_tests.rs::video_edit_e2e_cancel_stale_snapshot_preserves_latest_receipts_and_fences_owner` | `crates/libs/workflow-video-edit/src/host_migration_44_tests/manifest_tests.rs` |
| `prepare_tests.rs::video_edit_prepare_unbound_readonly_source_reuses_durable_run` | `crates/libs/workflow-video-edit/src/host_migration_44_tests/prepare_tests.rs` |
| `prepare_tests.rs::video_edit_prepare_unknown_writes_reconcile_without_duplicate_side_effects` | `crates/libs/workflow-video-edit/src/host_migration_44_tests/prepare_tests.rs` |
| `prepare_tests.rs::video_edit_prepare_lost_session_response_stays_pending_without_resubmission` | `crates/libs/workflow-video-edit/src/host_migration_44_tests/prepare_tests.rs` |
| `prepare_tests.rs::video_edit_prepare_source_content_and_snapshots_invalidate_cache` | `crates/libs/workflow-video-edit/src/host_migration_44_tests/prepare_tests.rs` |
| `prepare_tests.rs::video_edit_prepare_replacement_and_nonvideo_never_return_success` | `crates/libs/workflow-video-edit/src/host_migration_44_tests/prepare_tests.rs` |
| `prepare_tests.rs::video_edit_prepare_two_projects_need_no_binding_and_have_independent_sources` | `crates/libs/workflow-video-edit/src/host_migration_44_tests/prepare_tests.rs` |
| `prepare_tests.rs::video_edit_prepare_no_space_and_unapproved_choice_have_no_side_effects` | `crates/libs/workflow-video-edit/src/host_migration_44_tests/prepare_tests.rs` |
| `prepare_tests.rs::video_edit_prepare_multiple_spaces_use_real_question_and_cancel` | `crates/vw-agent/src/workflow/video_edit/prepare_tests.rs` |
| `prepare_tests.rs::video_edit_prepare_streaming_hash_obeys_security_and_cancellation` | `crates/vw-agent/src/workflow/video_edit/prepare_tests.rs` |
| `prepare_tests.rs::video_edit_prepare_cross_tenant_runs_and_native_mismatch_are_isolated` | `crates/vw-agent/src/workflow/video_edit/prepare_tests.rs` |
| `prepare_tests.rs::video_edit_prepare_competing_requests_create_and_upload_only_once` | `crates/libs/workflow-video-edit/src/host_migration_44_tests/prepare_tests.rs` |
| `prepare_tests.rs::video_edit_prepare_incomplete_put_never_becomes_success_or_reuploads` | `crates/libs/workflow-video-edit/src/host_migration_44_tests/prepare_tests.rs` |
| `prepare_tests.rs::video_edit_prepare_source_replacement_with_same_bytes_changes_identity` | `crates/libs/workflow-video-edit/src/host_migration_44_tests/prepare_tests.rs` |
| `prepare_tests.rs::video_edit_prepare_proven_non_submission_retries_same_ledger_without_duplicate_project` | `crates/libs/workflow-video-edit/src/host_migration_44_tests/prepare_tests.rs` |
| `prepare_tests.rs::video_edit_prepare_unknown_without_matching_project_never_reposts` | `crates/libs/workflow-video-edit/src/host_migration_44_tests/prepare_tests.rs` |

## 生产/测试行数口径

生产文件为不含 tests/test_cloud/test_transport 的文件；实际生产算法与宿主端口逐符号职责如上表。其余为 cfg(test) 测试及夹具，不计生产收益。

- 迁前宿主：生产/转发/文档 258 行，测试/夹具 3560 行。
- 迁后宿主：生产/转发/文档 256 行，测试/夹具 2568 行。
- 新增领域测试：生产/转发/文档 0 行，测试/夹具 1337 行。

## 未来验证（本次未运行）

领域：`python3 scripts/cargo_slot.py cargo test -p workflow-video-edit --lib host_migration_44_tests -- --test-threads=1`，预期 44 条（其中 Unix 特定用例按平台条件，Postgres 用例依数据库 fixture 前置条件）。

宿主：对上表仍在 vw-agent 的每个测试，以 `python3 scripts/cargo_slot.py cargo test -p vw-agent --lib workflow::video_edit::<模块>::<测试名> -- --exact --test-threads=1` 定向执行，逐条预期 1。迁出测试须通过未来真实宿主 --list 证明消失；不以零测试表示通过。

编译、断言、运行耗时与构建收益全部未知。静态已核对原始测试名守恒、领域自注册、原 entry_tests 挂载删除、直接依赖方向和差异空白。

## 深度收口追加（取代前述最终行数与保留结论）

累计宿主 3818 → 1663 行，净减 2155 行（56.4%）。领域自注册45测，真实宿主37测；82个原始测试名称静态守恒。未运行 Cargo、构建、测试或任何测试程序。

本轮新增真实消费者使用的 `test-support` feature，仅 vw-agent dev-dependencies 启用；默认生产构建不加载夹具。依赖方向仍为宿主→领域，不复制 App、工具权限或 session 全局状态。feature 不改变生产授权。没有新增生产算法迁出收益；本轮全部新增领域行均为受 feature/cfg(test) 控制的契约与夹具，不宣称编译提速。

- e2e_test_cloud.rs 的 Scenario/Fault/FinishOrder/Cloud/Model 完整归还 test_support/scenario.rs；Cloud.context 接收已有 VideoContextHandle，宿主 e2e 仍注入真实 ToolRuntimeContext。三份 synthetic JSON 由领域库自身持有。
- entry_test_transport 的 Cloud/Model/PROJECTS 移至 test_support/receipts.rs；宿主仅保留 SESSIONS/EXTERNALS、TransportGuard、install/install_external/get，用原 session 查找并注入真实上下文。原共享 PROJECTS 行为保持。
- prepare_tests 的 Remote/Cloud 完整下沉 test_support/prepare_fixture.rs，Cloud<C: VideoContext> 同时服务领域和宿主三个边界测试，无算法重写或假授权。
- cloud_assets、cloud_prepare、transcript 原生测试仅保留 fixture/discover 和端口注入；原响应、任务、票据、文件隐私断言与外部脚本移至 contracts.rs 的同名函数，每个函数有真实薄测试调用。
- cloud_delivery 的 OCC、native_scenario 领域主体、finalize 回读断言归还 contracts::{delivery_scenario,finalize_receipt}；真实发现/路径与三个交付结果测试仍宿主注册。纯 probe 规范化测试迁入 host_migration_44_tests/probe_tests.rs。
- manifest/assets 的无测试 fixture 完整迁至 test_support/{manifest_fixture,assets_fixture}，旧文件及旧 mod 挂载均已删除；contracts 直接调用领域 fixture。

### 当前全文件职责与行数

| 宿主文件 | 行数 | 当前职责 |
|---|---:|---|
| `apply.rs` | 2 | 既有宿主端口或兼容重导出，生产职责参见前表。 |
| `apply_journal.rs` | 2 | 既有宿主端口或兼容重导出，生产职责参见前表。 |
| `assets.rs` | 2 | 既有宿主端口或兼容重导出，生产职责参见前表。 |
| `budget.rs` | 2 | 既有宿主端口或兼容重导出，生产职责参见前表。 |
| `cloud.rs` | 26 | 既有宿主端口或兼容重导出，生产职责参见前表。 |
| `cloud_assets_tests.rs` | 15 | 原生发现注入，调用领域素材契约。 |
| `cloud_delivery_tests.rs` | 51 | 原生发现和交付结果边界，调用领域真实交付契约。 |
| `cloud_failure.rs` | 1 | 既有宿主端口或兼容重导出，生产职责参见前表。 |
| `cloud_prepare_tests.rs` | 27 | 原生发现与上下文注入，调用领域上传/身份契约。 |
| `cloud_tests.rs` | 384 | 真实原生 CLI/工具审批/hook/路径与取消测试，原生工厂 fixture。 |
| `command.rs` | 1 | 既有宿主端口或兼容重导出，生产职责参见前表。 |
| `command_tests.rs` | 73 | 真实原生安全策略三个边界。 |
| `e2e_tests.rs` | 479 | 真实网关流、会话租户持久化、事件顺序和取消验收，使用领域外部夹具。 |
| `entry.rs` | 66 | 既有宿主端口或兼容重导出，生产职责参见前表。 |
| `entry_contract.rs` | 2 | 既有宿主端口或兼容重导出，生产职责参见前表。 |
| `entry_test_transport.rs` | 37 | 真实 session 注册、生命周期 guard、上下文工厂。 |
| `manifest.rs` | 2 | 既有宿主端口或兼容重导出，生产职责参见前表。 |
| `merge.rs` | 2 | 既有宿主端口或兼容重导出，生产职责参见前表。 |
| `mod.rs` | 62 | 既有宿主端口或兼容重导出，生产职责参见前表。 |
| `planning.rs` | 11 | 既有宿主端口或兼容重导出，生产职责参见前表。 |
| `planning_tests.rs` | 219 | 真实 WorkflowRuntime 图调度和共享模型预算边界。 |
| `prepare.rs` | 61 | 既有宿主端口或兼容重导出，生产职责参见前表。 |
| `prepare_tests.rs` | 115 | 真实问答/取消、原生路径和租户隔离三个测试与薄实例化。 |
| `preview.rs` | 2 | 既有宿主端口或兼容重导出，生产职责参见前表。 |
| `segments.rs` | 2 | 既有宿主端口或兼容重导出，生产职责参见前表。 |
| `transcript.rs` | 1 | 既有宿主端口或兼容重导出，生产职责参见前表。 |
| `transcript_tests.rs` | 14 | 原生发现注入，调用领域转写任务/隐私契约。 |
| `types.rs` | 2 | 既有宿主端口或兼容重导出，生产职责参见前表。 |

### 更新后的全量测试挂载

| 原测试名称 | 当前唯一挂载 |
|---|---|
| `video_edit_asset_jobs_native_adapter_reconciles_pages_and_reads_reference` | `crates/vw-agent/src/workflow/video_edit/cloud_assets_tests.rs` |
| `video_edit_e2e_long_transcripts_reach_done_and_reload` | `crates/vw-agent/src/workflow/video_edit/e2e_tests.rs` |
| `video_edit_e2e_quota_invalid_segment_and_fake_preview_never_done` | `crates/vw-agent/src/workflow/video_edit/e2e_tests.rs` |
| `video_edit_e2e_accepted_timeout_reconciles_without_duplicate_post` | `crates/vw-agent/src/workflow/video_edit/e2e_tests.rs` |
| `video_edit_e2e_independent_requests_have_distinct_run_budgets` | `crates/vw-agent/src/workflow/video_edit/e2e_tests.rs` |
| `video_edit_e2e_native_permission_denial_prevents_external_io` | `crates/vw-agent/src/workflow/video_edit/e2e_tests.rs` |
| `video_edit_e2e_final_save_failure_never_done` | `crates/vw-agent/src/workflow/video_edit/e2e_tests.rs` |
| `video_edit_e2e_cancelled_restart_reconciles_and_never_resubmits` | `crates/vw-agent/src/workflow/video_edit/e2e_tests.rs` |
| `video_edit_e2e_template_edit_changes_next_native_model_request` | `crates/vw-agent/src/workflow/video_edit/e2e_tests.rs` |
| `video_edit_delivery_normalizes_live_media_probe_projection_without_losing_audio` | `crates/libs/workflow-video-edit/src/host_migration_44_tests/probe_tests.rs` |
| `video_edit_delivery_native_adapter_applies_three_types_and_reads_real_contract_shapes` | `crates/vw-agent/src/workflow/video_edit/cloud_delivery_tests.rs` |
| `video_edit_delivery_native_adapter_rejects_partial_caption_failure_and_pending_draft` | `crates/vw-agent/src/workflow/video_edit/cloud_delivery_tests.rs` |
| `video_edit_delivery_native_adapter_records_verified_empty_cleanup_separately` | `crates/vw-agent/src/workflow/video_edit/cloud_delivery_tests.rs` |
| `video_edit_delivery_native_finalize_accepts_observed_complete_import_receipt` | `crates/vw-agent/src/workflow/video_edit/cloud_delivery_tests.rs` |
| `video_edit_transcript_preserves_facts_silence_and_fractional_fps` | `crates/libs/workflow-video-edit/src/host_migration_44_tests/transcript_tests.rs` |
| `video_edit_transcript_rejects_invalid_times_and_duplicate_indices` | `crates/libs/workflow-video-edit/src/host_migration_44_tests/transcript_tests.rs` |
| `video_edit_transcript_rejects_silent_failed_empty_and_unconfirmed_punctuation` | `crates/libs/workflow-video-edit/src/host_migration_44_tests/transcript_tests.rs` |
| `video_edit_transcript_concurrent_resume_reuses_one_job_and_persisted_result` | `crates/libs/workflow-video-edit/src/host_migration_44_tests/transcript_tests.rs` |
| `video_edit_transcript_errors_never_resubmit_or_produce_fake_speech` | `crates/libs/workflow-video-edit/src/host_migration_44_tests/transcript_tests.rs` |
| `video_edit_transcript_rejects_invalid_media_and_segment_order` | `crates/libs/workflow-video-edit/src/host_migration_44_tests/transcript_tests.rs` |
| `video_edit_transcript_native_adapter_keeps_audio_ticket_private_and_queries_exact_task` | `crates/vw-agent/src/workflow/video_edit/transcript_tests.rs` |
| `video_edit_asset_jobs_unknown_is_not_ready` | `crates/libs/workflow-video-edit/src/host_migration_44_tests/assets_tests.rs` |
| `video_edit_asset_jobs_hold_five_until_reverse_completion` | `crates/libs/workflow-video-edit/src/host_migration_44_tests/assets_tests.rs` |
| `video_edit_asset_jobs_restart_reconciles_lost_receipt_without_post` | `crates/libs/workflow-video-edit/src/host_migration_44_tests/assets_tests.rs` |
| `video_edit_asset_jobs_refusal_retries_share_peak_and_bound_attempts` | `crates/libs/workflow-video-edit/src/host_migration_44_tests/assets_tests.rs` |
| `video_edit_asset_jobs_cancel_retains_remote_slots_and_stops_dispatch` | `crates/libs/workflow-video-edit/src/host_migration_44_tests/assets_tests.rs` |
| `video_edit_asset_jobs_failure_or_unreadable_result_blocks_registry` | `crates/libs/workflow-video-edit/src/host_migration_44_tests/assets_tests.rs` |
| `video_edit_asset_jobs_share_model_budget_and_resume_unknown` | `crates/libs/workflow-video-edit/src/host_migration_44_tests/assets_tests.rs` |
| `video_edit_asset_jobs_unknown_missing_from_remote_is_not_resubmitted` | `crates/libs/workflow-video-edit/src/host_migration_44_tests/assets_tests.rs` |
| `video_edit_asset_jobs_failed_attempt_retries_only_after_terminal_and_keeps_id` | `crates/libs/workflow-video-edit/src/host_migration_44_tests/assets_tests.rs` |
| `video_edit_asset_jobs_deduplicate_content_parameters_and_scope_without_guessing_recipe` | `crates/libs/workflow-video-edit/src/host_migration_44_tests/assets_tests.rs` |
| `video_edit_asset_jobs_restart_does_not_resume_cancelled_dispatch` | `crates/libs/workflow-video-edit/src/host_migration_44_tests/assets_tests.rs` |
| `video_edit_command_parses_quoted_chinese_and_windows_paths` | `crates/libs/workflow-video-edit/src/host_migration_44_tests/command_tests.rs` |
| `video_edit_command_validates_real_paths_without_binding` | `crates/libs/workflow-video-edit/src/host_migration_44_tests/command_tests.rs` |
| `video_edit_command_rejects_invalid_files_and_ambiguous_input` | `crates/libs/workflow-video-edit/src/host_migration_44_tests/command_tests.rs` |
| `video_edit_command_symlink_obeys_native_security_and_full_access` | `crates/vw-agent/src/workflow/video_edit/command_tests.rs` |
| `video_edit_command_shell_characters_remain_literal` | `crates/libs/workflow-video-edit/src/host_migration_44_tests/command_tests.rs` |
| `video_edit_command_external_allowed_root_and_no_project` | `crates/vw-agent/src/workflow/video_edit/command_tests.rs` |
| `video_edit_command_delegated_parent_remains_restricted_under_full_access` | `crates/vw-agent/src/workflow/video_edit/command_tests.rs` |
| `video_edit_entry_rejects_uncontrolled_graph_before_storage` | `crates/libs/workflow-video-edit/src/host_migration_44_tests/entry_tests.rs` |
| `video_edit_entry_distributes_production_graph` | `crates/libs/workflow-video-edit/src/host_migration_44_tests/entry_tests.rs` |
| `video_edit_entry_preserves_menu_and_unrelated_commands` | `crates/libs/workflow-video-edit/src/host_migration_44_tests/entry_tests.rs` |
| `video_edit_entry_rejects_capability_order_and_fake_ready_edits` | `crates/libs/workflow-video-edit/src/host_migration_44_tests/entry_tests.rs` |
| `video_edit_entry_accepts_business_prompt_model_and_lower_budget` | `crates/libs/workflow-video-edit/src/host_migration_44_tests/entry_tests.rs` |
| `video_edit_cloud_transport_hidden_skill_is_rejected` | `crates/vw-agent/src/workflow/video_edit/cloud_tests.rs` |
| `video_edit_cloud_transport_discovers_current_instance_and_full_private_receipt` | `crates/vw-agent/src/workflow/video_edit/cloud_tests.rs` |
| `video_edit_cloud_transport_version_source_and_authorization_fail_closed` | `crates/vw-agent/src/workflow/video_edit/cloud_tests.rs` |
| `video_edit_cloud_transport_missing_credentials_never_logs_in_or_falls_back` | `crates/vw-agent/src/workflow/video_edit/cloud_tests.rs` |
| `video_edit_cloud_transport_native_tool_and_path_denials_prevent_process` | `crates/vw-agent/src/workflow/video_edit/cloud_tests.rs` |
| `video_edit_cloud_transport_cancel_before_discovery_starts_no_process` | `crates/vw-agent/src/workflow/video_edit/cloud_tests.rs` |
| `video_edit_cloud_transport_errors_and_unknown_states_are_not_success` | `crates/vw-agent/src/workflow/video_edit/cloud_tests.rs` |
| `video_edit_cloud_transport_duplicate_visible_instances_are_ambiguous` | `crates/vw-agent/src/workflow/video_edit/cloud_tests.rs` |
| `video_edit_cloud_transport_no_project_uses_native_workspace` | `crates/vw-agent/src/workflow/video_edit/cloud_tests.rs` |
| `video_edit_cloud_transport_inflight_cancel_stops_process_before_private_cleanup` | `crates/vw-agent/src/workflow/video_edit/cloud_tests.rs` |
| `video_edit_cloud_transport_native_approval_denial_prevents_process` | `crates/vw-agent/src/workflow/video_edit/cloud_tests.rs` |
| `video_edit_cloud_transport_native_hooks_never_receive_private_payloads` | `crates/vw-agent/src/workflow/video_edit/cloud_tests.rs` |
| `video_edit_plan_json_rejects_boolean_time_and_unstructured_assets` | `crates/libs/workflow-video-edit/src/host_migration_44_tests/planning_tests.rs` |
| `video_edit_plan_real_iteration_six_segments_bounded_requests_and_shared_budget` | `crates/vw-agent/src/workflow/video_edit/planning_tests.rs` |
| `video_edit_plan_failed_segment_never_returns_partial_plan` | `crates/vw-agent/src/workflow/video_edit/planning_tests.rs` |
| `video_edit_plan_context_overflow_is_explicit_before_model_request` | `crates/vw-agent/src/workflow/video_edit/planning_tests.rs` |
| `video_edit_prepare_native_receipts_keep_signed_tickets_in_memory` | `crates/vw-agent/src/workflow/video_edit/cloud_prepare_tests.rs` |
| `video_edit_prepare_cloud_identity_is_independent_of_local_tenant_and_rejects_mismatch` | `crates/vw-agent/src/workflow/video_edit/cloud_prepare_tests.rs` |
| `video_edit_prepare_ledger_claims_without_project_binding` | `crates/libs/workflow-video-edit/src/host_migration_44_tests/manifest_tests.rs` |
| `video_edit_prepare_ledger_fences_stale_writers_and_tenants` | `crates/libs/workflow-video-edit/src/host_migration_44_tests/manifest_tests.rs` |
| `video_edit_prepare_ledger_postgres_contract` | `crates/libs/workflow-video-edit/src/host_migration_44_tests/manifest_tests.rs` |
| `video_edit_prepare_ledger_competing_connections_claim_once_and_restart` | `crates/libs/workflow-video-edit/src/host_migration_44_tests/manifest_tests.rs` |
| `video_edit_e2e_cancel_stale_snapshot_preserves_latest_receipts_and_fences_owner` | `crates/libs/workflow-video-edit/src/host_migration_44_tests/manifest_tests.rs` |
| `video_edit_prepare_unbound_readonly_source_reuses_durable_run` | `crates/libs/workflow-video-edit/src/host_migration_44_tests/prepare_tests.rs` |
| `video_edit_prepare_unknown_writes_reconcile_without_duplicate_side_effects` | `crates/libs/workflow-video-edit/src/host_migration_44_tests/prepare_tests.rs` |
| `video_edit_prepare_lost_session_response_stays_pending_without_resubmission` | `crates/libs/workflow-video-edit/src/host_migration_44_tests/prepare_tests.rs` |
| `video_edit_prepare_source_content_and_snapshots_invalidate_cache` | `crates/libs/workflow-video-edit/src/host_migration_44_tests/prepare_tests.rs` |
| `video_edit_prepare_replacement_and_nonvideo_never_return_success` | `crates/libs/workflow-video-edit/src/host_migration_44_tests/prepare_tests.rs` |
| `video_edit_prepare_two_projects_need_no_binding_and_have_independent_sources` | `crates/libs/workflow-video-edit/src/host_migration_44_tests/prepare_tests.rs` |
| `video_edit_prepare_no_space_and_unapproved_choice_have_no_side_effects` | `crates/libs/workflow-video-edit/src/host_migration_44_tests/prepare_tests.rs` |
| `video_edit_prepare_multiple_spaces_use_real_question_and_cancel` | `crates/vw-agent/src/workflow/video_edit/prepare_tests.rs` |
| `video_edit_prepare_streaming_hash_obeys_security_and_cancellation` | `crates/vw-agent/src/workflow/video_edit/prepare_tests.rs` |
| `video_edit_prepare_cross_tenant_runs_and_native_mismatch_are_isolated` | `crates/vw-agent/src/workflow/video_edit/prepare_tests.rs` |
| `video_edit_prepare_competing_requests_create_and_upload_only_once` | `crates/libs/workflow-video-edit/src/host_migration_44_tests/prepare_tests.rs` |
| `video_edit_prepare_incomplete_put_never_becomes_success_or_reuploads` | `crates/libs/workflow-video-edit/src/host_migration_44_tests/prepare_tests.rs` |
| `video_edit_prepare_source_replacement_with_same_bytes_changes_identity` | `crates/libs/workflow-video-edit/src/host_migration_44_tests/prepare_tests.rs` |
| `video_edit_prepare_proven_non_submission_retries_same_ledger_without_duplicate_project` | `crates/libs/workflow-video-edit/src/host_migration_44_tests/prepare_tests.rs` |
| `video_edit_prepare_unknown_without_matching_project_never_reposts` | `crates/libs/workflow-video-edit/src/host_migration_44_tests/prepare_tests.rs` |

未来领域命令保持 `python3 scripts/cargo_slot.py cargo test -p workflow-video-edit --lib host_migration_44_tests -- --test-threads=1`，Unix 预期45测；宿主上表各37个完整名称按原精确命令逐条执行。37测包含调用领域契约函数的薄测试，契约函数不是独立测试，不能重复计数。

静态已检查82测试名唯一挂载、旧fixture挂载移除、所有契约函数真实调用、领域资源相对路径及依赖 feature 接线。净减仍不足70%：主要残留 cloud_tests、真实网关 e2e、原生规划调度。继续减少需进一步从真实权限/事件断言中抽取领域规约；整轮是否达35000行由收口重新统计。本证据不宣称整轮门槛通过。
