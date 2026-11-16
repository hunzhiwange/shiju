# 任务33：兼容入口与残留装配

代码迁移及静态核对完成；所有 Cargo、编译、单元测试、实机与行为检查均未运行，不能视作运行验收。

## 范围与减量

同路径 splitlines：宿主 2474 → 1650，净减 824（33.3%）；新领域测试/夹具 951 行。真实生产实现迁出 0 行；删除的转发/测试包装不计生产收益。
净减未达70%，以下逐文件给出保留职责及继续可迁项，交收口任务判定；不声称整轮减量达标。

| 来源 | 前→后行数 | 实际所有者/保留职责 |
|---|---:|---|
| `agent_hosts.rs` | 242→242 | ChildCoordinatorHost/TaskCoordinatorHost/ToolAgentHost/HostChildRun：具体协调器、租户存储、网关流与循环端口；全部方法转调真实宿主，不复制状态。 |
| `agent_tool/mod.rs` | 32→32 | 已是领域库薄 re-export/具体 AgentContextHost 类型别名，保留公开调用兼容性；无新增业务算法。 既有跨目录测试挂载仍需真实宿主；继续可迁项见下表，不作为本轮收益。 |
| `apply_patch/mod.rs` | 6→6 | 已是领域库薄 re-export/具体 AgentContextHost 类型别名，保留公开调用兼容性；无新增业务算法。 既有跨目录测试挂载仍需真实宿主；继续可迁项见下表，不作为本轮收益。 |
| `batch/mod.rs` | 8→8 | 已是领域库薄 re-export/具体 AgentContextHost 类型别名，保留公开调用兼容性；无新增业务算法。 既有跨目录测试挂载仍需真实宿主；继续可迁项见下表，不作为本轮收益。 |
| `browser/mod.rs` | 3→3 | 已是领域库薄 re-export/具体 AgentContextHost 类型别名，保留公开调用兼容性；无新增业务算法。 |
| `browser_open/mod.rs` | 3→3 | 已是领域库薄 re-export/具体 AgentContextHost 类型别名，保留公开调用兼容性；无新增业务算法。 |
| `catalog.rs` | 2→2 | 已是领域库薄 re-export/具体 AgentContextHost 类型别名，保留公开调用兼容性；无新增业务算法。 |
| `catalog_tests.rs` | 174→0 | 整组领域测试迁入 tool-assembly::host_migration_33_tests；旧物理文件与挂载已删除。 |
| `client_tool.rs` | 2→2 | 已是领域库薄 re-export/具体 AgentContextHost 类型别名，保留公开调用兼容性；无新增业务算法。 |
| `client_tool_tests.rs` | 1→1 | 已是领域库薄 re-export/具体 AgentContextHost 类型别名，保留公开调用兼容性；无新增业务算法。 既有跨目录测试挂载仍需真实宿主；继续可迁项见下表，不作为本轮收益。 |
| `codesearch/mod.rs` | 3→3 | 已是领域库薄 re-export/具体 AgentContextHost 类型别名，保留公开调用兼容性；无新增业务算法。 |
| `computer_control.rs` | 7→7 | 已是领域库薄 re-export/具体 AgentContextHost 类型别名，保留公开调用兼容性；无新增业务算法。 既有跨目录测试挂载仍需真实宿主；继续可迁项见下表，不作为本轮收益。 |
| `config/mod.rs` | 2→2 | 已是领域库薄 re-export/具体 AgentContextHost 类型别名，保留公开调用兼容性；无新增业务算法。 |
| `content_search/mod.rs` | 2→2 | 已是领域库薄 re-export/具体 AgentContextHost 类型别名，保留公开调用兼容性；无新增业务算法。 |
| `context/mod.rs` | 81→81 | AgentContextHost 实现及任务局部类型绑定；保留真实审批、hooks、插件 ready 与工作区租约端口。 |
| `context/run_worktree.rs` | 3→3 | 已是领域库薄 re-export/具体 AgentContextHost 类型别名，保留公开调用兼容性；无新增业务算法。 |
| `context/shell_policy.rs` | 32→32 | shell_security_decision 把真实 Shell Permission 检查结果适配领域决策；继续可迁：将适配接回 agent-security。 |
| `context/tests.rs` | 223→52 | builder_methods_expose_injected_runtime_capabilities 核对真实 ApprovalManager、HookRunner、Observer 注入身份。 |
| `cron_add/mod.rs` | 1→1 | 已是领域库薄 re-export/具体 AgentContextHost 类型别名，保留公开调用兼容性；无新增业务算法。 |
| `cron_list/mod.rs` | 1→1 | 已是领域库薄 re-export/具体 AgentContextHost 类型别名，保留公开调用兼容性；无新增业务算法。 |
| `cron_remove/mod.rs` | 1→1 | 已是领域库薄 re-export/具体 AgentContextHost 类型别名，保留公开调用兼容性；无新增业务算法。 |
| `cron_run/mod.rs` | 12→12 | execute_job_now 绑定宿主 scheduler::execute_job_now 并保留取消语义。 |
| `cron_run/tests.rs` | 174→174 | 真实计划执行、SQLite 历史及宿主审批/速率限制集成；继续可迁：独立 scheduler 执行入口后迁 tool-cron。 |
| `cron_runs/mod.rs` | 1→1 | 已是领域库薄 re-export/具体 AgentContextHost 类型别名，保留公开调用兼容性；无新增业务算法。 |
| `cron_update/mod.rs` | 1→1 | 已是领域库薄 re-export/具体 AgentContextHost 类型别名，保留公开调用兼容性；无新增业务算法。 |
| `decision/mod.rs` | 26→20 | 保留 request_control_scope_approval 生产适配与宿主薄审批测试工具；已删除迁出断言专用包装。 |
| `decision/tests.rs` | 481→288 | 保留 shell 端口、真实非 CLI 审批投递和 native defaults/hook 接线断言；继续可迁：agent-loop 审批等待与 defaults 领域测试。 |
| `delegate/mod.rs` | 19→19 | 已是领域库薄 re-export/具体 AgentContextHost 类型别名，保留公开调用兼容性；无新增业务算法。 既有跨目录测试挂载仍需真实宿主；继续可迁项见下表，不作为本轮收益。 |
| `delegated_tools.rs` | 18→18 | build_agentic_tools 把 session-engine::capability_policy 的可信解析结果送 tool-services；继续可迁：能力解析归还独立能力库后移走包装。 |
| `delegated_tools_tests.rs` | 175→175 | 9 条断言覆盖上述解析+筛选真实接线；继续可迁：session-engine 解析与 tool-services 组合整体领域化。 |
| `discovery.rs` | 1→1 | 已是领域库薄 re-export/具体 AgentContextHost 类型别名，保留公开调用兼容性；无新增业务算法。 |
| `enter_worktree/mod.rs` | 6→6 | 已是领域库薄 re-export/具体 AgentContextHost 类型别名，保留公开调用兼容性；无新增业务算法。 既有跨目录测试挂载仍需真实宿主；继续可迁项见下表，不作为本轮收益。 |
| `excel_control.rs` | 4→4 | 已是领域库薄 re-export/具体 AgentContextHost 类型别名，保留公开调用兼容性；无新增业务算法。 |
| `excel_control_tests.rs` | 430→430 | 真实 UID/TLS/WebSocket/OfficeTransport、权限过期、取消与共享 NativeControlRuntime 工厂集成；继续可迁：tool-office 集成夹具和仅保留工厂一条。 |
| `executor/mod.rs` | 2→2 | 已是领域库薄 re-export/具体 AgentContextHost 类型别名，保留公开调用兼容性；无新增业务算法。 |
| `exit_worktree/mod.rs` | 6→6 | 已是领域库薄 re-export/具体 AgentContextHost 类型别名，保留公开调用兼容性；无新增业务算法。 既有跨目录测试挂载仍需真实宿主；继续可迁项见下表，不作为本轮收益。 |
| `external_directory/mod.rs` | 11→6 | assert_external_directory 注入真实 session_skills/skill_directories；保留调用者入口。 |
| `external_directory/tests.rs` | 271→0 | 整组领域测试迁入 tool-assembly::host_migration_33_tests；旧物理文件与挂载已删除。 |
| `file_edit/mod.rs` | 7→3 | 已是领域库薄 re-export/具体 AgentContextHost 类型别名，保留公开调用兼容性；无新增业务算法。 既有跨目录测试挂载仍需真实宿主；继续可迁项见下表，不作为本轮收益。 |

## 全用例映射（函数名和断言保持）

- `crates/vw-agent/src/tools/catalog_tests.rs::dynamic_tools_catalog_replacement_pins_inflight_executor` → `crates/libs/tool-assembly/src/host_migration_33_tests/catalog.rs::dynamic_tools_catalog_replacement_pins_inflight_executor`
- `crates/vw-agent/src/tools/catalog_tests.rs::dynamic_tools_catalog_rejects_duplicates_without_publication` → `crates/libs/tool-assembly/src/host_migration_33_tests/catalog.rs::dynamic_tools_catalog_rejects_duplicates_without_publication`
- `crates/vw-agent/src/tools/catalog_tests.rs::dynamic_tools_catalog_revocation_does_not_restore_loaded_ids` → `crates/libs/tool-assembly/src/host_migration_33_tests/catalog.rs::dynamic_tools_catalog_revocation_does_not_restore_loaded_ids`
- `crates/vw-agent/src/tools/catalog_tests.rs::dynamic_tools_catalog_rejects_reserved_client_names` → `crates/libs/tool-assembly/src/host_migration_33_tests/catalog.rs::dynamic_tools_catalog_rejects_reserved_client_names`
- `crates/vw-agent/src/tools/catalog_tests.rs::dynamic_tools_catalog_same_number_from_another_catalog_is_rejected` → `crates/libs/tool-assembly/src/host_migration_33_tests/catalog.rs::dynamic_tools_catalog_same_number_from_another_catalog_is_rejected`
- `crates/vw-agent/src/tools/catalog_tests.rs::dynamic_tools_catalog_session_and_context_share_snapshot_without_granting_metadata` → `crates/libs/tool-assembly/src/host_migration_33_tests/catalog.rs::dynamic_tools_catalog_session_and_context_share_snapshot_without_granting_metadata`
- `crates/vw-agent/src/tools/context/tests.rs::default_workflow_state_is_unbound` → `crates/libs/tool-assembly/src/host_migration_33_tests/context.rs::default_workflow_state_is_unbound`
- `crates/vw-agent/src/tools/context/tests.rs::context_preserves_session_and_root` → `crates/libs/tool-assembly/src/host_migration_33_tests/context.rs::context_preserves_session_and_root`
- `crates/vw-agent/src/tools/context/tests.rs::workflow_transitions_reset_worktree` → `crates/libs/tool-assembly/src/host_migration_33_tests/context.rs::workflow_transitions_reset_worktree`
- `crates/vw-agent/src/tools/context/tests.rs::task_local_context_is_visible_only_inside_scope` → `crates/libs/tool-assembly/src/host_migration_33_tests/context.rs::task_local_context_is_visible_only_inside_scope`
- `crates/vw-agent/src/tools/context/tests.rs::normalize_path_uses_context_workspace_root` → `crates/libs/tool-assembly/src/host_migration_33_tests/context.rs::normalize_path_uses_context_workspace_root`
- `crates/vw-agent/src/tools/context/tests.rs::default_and_debug_report_runtime_capability_presence` → `crates/libs/tool-assembly/src/host_migration_33_tests/context.rs::default_and_debug_report_runtime_capability_presence`
- `crates/vw-agent/src/tools/context/tests.rs::builder_methods_expose_injected_runtime_capabilities` → `crates/vw-agent/src/tools/context/tests.rs::builder_methods_expose_injected_runtime_capabilities`（宿主边界保留）
- `crates/vw-agent/src/tools/context/tests.rs::trusted_user_skill_selections_survive_tool_call_context_cloning` → `crates/libs/tool-assembly/src/host_migration_33_tests/context.rs::trusted_user_skill_selections_survive_tool_call_context_cloning`
- `crates/vw-agent/src/tools/context/tests.rs::shared_read_state_and_workflow_handles_are_reused` → `crates/libs/tool-assembly/src/host_migration_33_tests/context.rs::shared_read_state_and_workflow_handles_are_reused`
- `crates/vw-agent/src/tools/context/tests.rs::scoped_read_state_helpers_note_get_and_invalidate_paths` → `crates/libs/tool-assembly/src/host_migration_33_tests/context.rs::scoped_read_state_helpers_note_get_and_invalidate_paths`
- `crates/vw-agent/src/tools/cron_run/tests.rs::force_runs_job_and_records_history` → `crates/vw-agent/src/tools/cron_run/tests.rs::force_runs_job_and_records_history`（宿主边界保留）
- `crates/vw-agent/src/tools/cron_run/tests.rs::errors_for_missing_job` → `crates/vw-agent/src/tools/cron_run/tests.rs::errors_for_missing_job`（宿主边界保留）
- `crates/vw-agent/src/tools/cron_run/tests.rs::blocks_run_in_read_only_mode` → `crates/vw-agent/src/tools/cron_run/tests.rs::blocks_run_in_read_only_mode`（宿主边界保留）
- `crates/vw-agent/src/tools/cron_run/tests.rs::shell_run_requires_approval_for_medium_risk` → `crates/vw-agent/src/tools/cron_run/tests.rs::shell_run_requires_approval_for_medium_risk`（宿主边界保留）
- `crates/vw-agent/src/tools/cron_run/tests.rs::blocks_run_when_rate_limited` → `crates/vw-agent/src/tools/cron_run/tests.rs::blocks_run_when_rate_limited`（宿主边界保留）
- `crates/vw-agent/src/tools/cron_run/tests.rs::cron_history_manual_run_is_visible_until_completion` → `crates/vw-agent/src/tools/cron_run/tests.rs::cron_history_manual_run_is_visible_until_completion`（宿主边界保留）
- `crates/vw-agent/src/tools/cron_run/tests.rs::cron_history_cancelled_manual_run_leaves_no_history` → `crates/vw-agent/src/tools/cron_run/tests.rs::cron_history_cancelled_manual_run_leaves_no_history`（宿主边界保留）
- `crates/vw-agent/src/tools/decision/tests.rs::allow_and_ask_expose_updated_input_and_reason` → `crates/libs/tool-assembly/src/host_migration_33_tests/decision.rs::allow_and_ask_expose_updated_input_and_reason`
- `crates/vw-agent/src/tools/decision/tests.rs::finalize_allows_without_security_and_classifies_tool_permission_errors` → `crates/libs/tool-assembly/src/host_migration_33_tests/decision.rs::finalize_allows_without_security_and_classifies_tool_permission_errors`
- `crates/vw-agent/src/tools/decision/tests.rs::finalize_permission_decision_maps_allow_ask_and_denials` → `crates/libs/tool-assembly/src/host_migration_33_tests/decision.rs::finalize_permission_decision_maps_allow_ask_and_denials`
- `crates/vw-agent/src/tools/decision/tests.rs::deny_only_exposes_input_after_explicit_snapshot` → `crates/libs/tool-assembly/src/host_migration_33_tests/decision.rs::deny_only_exposes_input_after_explicit_snapshot`
- `crates/vw-agent/src/tools/decision/tests.rs::updated_input_snapshot_only_changes_denials` → `crates/libs/tool-assembly/src/host_migration_33_tests/decision.rs::updated_input_snapshot_only_changes_denials`
- `crates/vw-agent/src/tools/decision/tests.rs::security_decision_handles_readonly_rate_limit_and_blocked_paths` → `crates/libs/tool-assembly/src/host_migration_33_tests/decision.rs::security_decision_handles_readonly_rate_limit_and_blocked_paths`
- `crates/vw-agent/src/tools/decision/tests.rs::blocked_path_checks_string_and_array_fields` → `crates/libs/tool-assembly/src/host_migration_33_tests/decision.rs::blocked_path_checks_string_and_array_fields`
- `crates/vw-agent/src/tools/decision/tests.rs::shell_security_decision_marks_approved_inputs_and_preserves_denials` → `crates/vw-agent/src/tools/decision/tests.rs::shell_security_decision_marks_approved_inputs_and_preserves_denials`（宿主边界保留）
- `crates/vw-agent/src/tools/decision/tests.rs::request_interactive_approval_denies_when_manager_is_missing_or_user_declines` → `crates/vw-agent/src/tools/decision/tests.rs::request_interactive_approval_denies_when_manager_is_missing_or_user_declines`（宿主边界保留）
- `crates/vw-agent/src/tools/decision/tests.rs::non_cli_bypass_records_approval_and_allows_pending_decision` → `crates/vw-agent/src/tools/decision/tests.rs::non_cli_bypass_records_approval_and_allows_pending_decision`（宿主边界保留）
- `crates/vw-agent/src/tools/decision/tests.rs::non_cli_approval_context_sends_prompt_and_waits_for_resolution` → `crates/vw-agent/src/tools/decision/tests.rs::non_cli_approval_context_sends_prompt_and_waits_for_resolution`（宿主边界保留）
- `crates/vw-agent/src/tools/decision/tests.rs::command_input_only_accepts_shell_tools_and_string_commands` → `crates/libs/tool-assembly/src/host_migration_33_tests/decision.rs::command_input_only_accepts_shell_tools_and_string_commands`
- `crates/vw-agent/src/tools/decision/tests.rs::approval_marker_only_changes_shell_object_inputs` → `crates/libs/tool-assembly/src/host_migration_33_tests/decision.rs::approval_marker_only_changes_shell_object_inputs`
- `crates/vw-agent/src/tools/decision/tests.rs::permission_reason_helpers_preserve_structured_context` → `crates/libs/tool-assembly/src/host_migration_33_tests/decision.rs::permission_reason_helpers_preserve_structured_context`
- `crates/vw-agent/src/tools/decision/tests.rs::classify_message_separates_denials_from_execution_failures` → `crates/libs/tool-assembly/src/host_migration_33_tests/decision.rs::classify_message_separates_denials_from_execution_failures`
- `crates/vw-agent/src/tools/decision/tests.rs::codex_plugin_compat_t37_host_denial_precedes_hook_allow` → `crates/vw-agent/src/tools/decision/tests.rs::codex_plugin_compat_t37_host_denial_precedes_hook_allow`（宿主边界保留）
- `crates/vw-agent/src/tools/decision/tests.rs::agent_config_cleanup_full_access_retains_native_permission_denials` → `crates/vw-agent/src/tools/decision/tests.rs::agent_config_cleanup_full_access_retains_native_permission_denials`（宿主边界保留）
- `crates/vw-agent/src/tools/decision/tests.rs::agent_native_defaults_permission_honors_registered_tool_aliases` → `crates/vw-agent/src/tools/decision/tests.rs::agent_native_defaults_permission_honors_registered_tool_aliases`（宿主边界保留）
- `crates/vw-agent/src/tools/decision/tests.rs::agent_native_defaults_skill_permission_uses_registered_input` → `crates/vw-agent/src/tools/decision/tests.rs::agent_native_defaults_skill_permission_uses_registered_input`（宿主边界保留）
- `crates/vw-agent/src/tools/decision/tests.rs::dynamic_tools_contract_client_readonly_hint_cannot_grant_security_capability` → `crates/libs/tool-assembly/src/host_migration_33_tests/decision.rs::dynamic_tools_contract_client_readonly_hint_cannot_grant_security_capability`
- `crates/vw-agent/src/tools/delegated_tools_tests.rs::build_agentic_tools_blocks_batch_even_when_selected` → `crates/vw-agent/src/tools/delegated_tools_tests.rs::build_agentic_tools_blocks_batch_even_when_selected`（宿主边界保留）
- `crates/vw-agent/src/tools/delegated_tools_tests.rs::build_agentic_tools_empty_selection_keeps_builtin_and_blocks_recursive_tool` → `crates/vw-agent/src/tools/delegated_tools_tests.rs::build_agentic_tools_empty_selection_keeps_builtin_and_blocks_recursive_tool`（宿主边界保留）
- `crates/vw-agent/src/tools/delegated_tools_tests.rs::build_agentic_tools_matches_tool_aliases` → `crates/vw-agent/src/tools/delegated_tools_tests.rs::build_agentic_tools_matches_tool_aliases`（宿主边界保留）
- `crates/vw-agent/src/tools/delegated_tools_tests.rs::delegated_skill_tool_enforces_resolved_skill_capabilities` → `crates/vw-agent/src/tools/delegated_tools_tests.rs::delegated_skill_tool_enforces_resolved_skill_capabilities`（宿主边界保留）
- `crates/vw-agent/src/tools/delegated_tools_tests.rs::strict_skill_policy_without_defaults_does_not_expose_skill` → `crates/vw-agent/src/tools/delegated_tools_tests.rs::strict_skill_policy_without_defaults_does_not_expose_skill`（宿主边界保留）
- `crates/vw-agent/src/tools/delegated_tools_tests.rs::delegated_tool_ref_forwards_contract_and_execution` → `crates/vw-agent/src/tools/delegated_tools_tests.rs::delegated_tool_ref_forwards_contract_and_execution`（宿主边界保留）
- `crates/vw-agent/src/tools/delegated_tools_tests.rs::filtered_skill_tool_exposes_sorted_trimmed_skill_contract` → `crates/vw-agent/src/tools/delegated_tools_tests.rs::filtered_skill_tool_exposes_sorted_trimmed_skill_contract`（宿主边界保留）
- `crates/vw-agent/src/tools/delegated_tools_tests.rs::filtered_skill_tool_delegates_missing_and_blank_names` → `crates/vw-agent/src/tools/delegated_tools_tests.rs::filtered_skill_tool_delegates_missing_and_blank_names`（宿主边界保留）
- `crates/vw-agent/src/tools/delegated_tools_tests.rs::delegated_tools_keep_command_lifecycle_without_restoring_optional_capabilities` → `crates/vw-agent/src/tools/delegated_tools_tests.rs::delegated_tools_keep_command_lifecycle_without_restoring_optional_capabilities`（宿主边界保留）
- `crates/vw-agent/src/tools/excel_control_tests.rs::cc_control_14_no_session_discovers_empty_and_refuses_execution` → `crates/vw-agent/src/tools/excel_control_tests.rs::cc_control_14_no_session_discovers_empty_and_refuses_execution`（宿主边界保留）
- `crates/vw-agent/src/tools/excel_control_tests.rs::cc_control_14_duplicate_titles_require_exact_document_and_generation` → `crates/vw-agent/src/tools/excel_control_tests.rs::cc_control_14_duplicate_titles_require_exact_document_and_generation`（宿主边界保留）
- `crates/vw-agent/src/tools/excel_control_tests.rs::cc_control_14_switch_and_document_grant_both_gate_execution` → `crates/vw-agent/src/tools/excel_control_tests.rs::cc_control_14_switch_and_document_grant_both_gate_execution`（宿主边界保留）
- `crates/vw-agent/src/tools/excel_control_tests.rs::cc_control_14_missing_capability_and_unknown_fields_are_rejected` → `crates/vw-agent/src/tools/excel_control_tests.rs::cc_control_14_missing_capability_and_unknown_fields_are_rejected`（宿主边界保留）
- `crates/vw-agent/src/tools/excel_control_tests.rs::cc_control_14_write_returns_actual_readback_to_model` → `crates/vw-agent/src/tools/excel_control_tests.rs::cc_control_14_write_returns_actual_readback_to_model`（宿主边界保留）
- `crates/vw-agent/src/tools/excel_control_tests.rs::cc_control_14_disconnect_and_repair_invalidate_old_generation` → `crates/vw-agent/src/tools/excel_control_tests.rs::cc_control_14_disconnect_and_repair_invalidate_old_generation`（宿主边界保留）
- `crates/vw-agent/src/tools/excel_control_tests.rs::cc_control_14_foreign_executor_owner_cannot_discover_or_execute` → `crates/vw-agent/src/tools/excel_control_tests.rs::cc_control_14_foreign_executor_owner_cannot_discover_or_execute`（宿主边界保留）
- `crates/vw-agent/src/tools/excel_control_tests.rs::cc_control_14_revocation_during_command_discards_result` → `crates/vw-agent/src/tools/excel_control_tests.rs::cc_control_14_revocation_during_command_discards_result`（宿主边界保留）
- `crates/vw-agent/src/tools/excel_control_tests.rs::cc_control_14_invalid_rectangles_shapes_and_payloads_never_dispatch` → `crates/vw-agent/src/tools/excel_control_tests.rs::cc_control_14_invalid_rectangles_shapes_and_payloads_never_dispatch`（宿主边界保留）
- `crates/vw-agent/src/tools/excel_control_tests.rs::cc_control_14_abort_invalidates_connection_and_discards_late_result` → `crates/vw-agent/src/tools/excel_control_tests.rs::cc_control_14_abort_invalidates_connection_and_discards_late_result`（宿主边界保留）
- `crates/vw-agent/src/tools/excel_control_tests.rs::cc_control_14_factory_registers_excel_with_shared_gateway_authority` → `crates/vw-agent/src/tools/excel_control_tests.rs::cc_control_14_factory_registers_excel_with_shared_gateway_authority`（宿主边界保留）
- `crates/vw-agent/src/tools/excel_control_tests.rs::cc_control_14_malformed_readback_is_failure_and_closes_connection` → `crates/vw-agent/src/tools/excel_control_tests.rs::cc_control_14_malformed_readback_is_failure_and_closes_connection`（宿主边界保留）
- `crates/vw-agent/src/tools/excel_control_tests.rs::cc_control_14_document_approval_is_scoped_and_explicit_deny_is_preserved` → `crates/vw-agent/src/tools/excel_control_tests.rs::cc_control_14_document_approval_is_scoped_and_explicit_deny_is_preserved`（宿主边界保留）
- `crates/vw-agent/src/tools/external_directory/tests.rs::default_options_enforce_file_checks` → `crates/libs/tool-assembly/src/host_migration_33_tests/external_directory.rs::default_options_enforce_file_checks`
- `crates/vw-agent/src/tools/external_directory/tests.rs::relative_paths_are_trimmed_joined_and_normalized` → `crates/libs/tool-assembly/src/host_migration_33_tests/external_directory.rs::relative_paths_are_trimmed_joined_and_normalized`
- `crates/vw-agent/src/tools/external_directory/tests.rs::empty_target_is_ok` → `crates/libs/tool-assembly/src/host_migration_33_tests/external_directory.rs::empty_target_is_ok`
- `crates/vw-agent/src/tools/external_directory/tests.rs::bypass_skips_checks` → `crates/libs/tool-assembly/src/host_migration_33_tests/external_directory.rs::bypass_skips_checks`
- `crates/vw-agent/src/tools/external_directory/tests.rs::workspace_relative_path_is_allowed` → `crates/libs/tool-assembly/src/host_migration_33_tests/external_directory.rs::workspace_relative_path_is_allowed`
- `crates/vw-agent/src/tools/external_directory/tests.rs::outside_workspace_is_denied_without_allowlist` → `crates/libs/tool-assembly/src/host_migration_33_tests/external_directory.rs::outside_workspace_is_denied_without_allowlist`
- `crates/vw-agent/src/tools/external_directory/tests.rs::outside_file_denial_reports_parent_directory` → `crates/libs/tool-assembly/src/host_migration_33_tests/external_directory.rs::outside_file_denial_reports_parent_directory`
- `crates/vw-agent/src/tools/external_directory/tests.rs::outside_file_denial_reports_original_path_when_parent_is_missing` → `crates/libs/tool-assembly/src/host_migration_33_tests/external_directory.rs::outside_file_denial_reports_original_path_when_parent_is_missing`
- `crates/vw-agent/src/tools/external_directory/tests.rs::allowed_roots_path_is_allowed` → `crates/libs/tool-assembly/src/host_migration_33_tests/external_directory.rs::allowed_roots_path_is_allowed`
- `crates/vw-agent/src/tools/external_directory/tests.rs::tool_output_directory_is_allowed_outside_workspace` → `crates/libs/tool-assembly/src/host_migration_33_tests/external_directory.rs::tool_output_directory_is_allowed_outside_workspace`

## 跨目录挂载核对

- `crates/vw-agent/src/tools/apply_patch/mod.rs` → `crates/libs/tool-services/src/apply_patch/tests.rs`：保留现有真实宿主绑定；继续可迁：拆出领域断言，须重建真实安全端口而非复制宿主。
  - `schema_is_object` 保留原挂载。
  - `tool_metadata_and_failure_results_are_stable` 保留原挂载。
  - `apply_patch_rejects_invalid_empty_and_noop_inputs` 保留原挂载。
  - `apply_patch_move_updates_destination_and_removes_source` 保留原挂载。
  - `apply_patch_add_file_succeeds` 保留原挂载。
  - `apply_patch_output_contains_changes_metadata` 保留原挂载。
  - `apply_patch_reports_partial_read_state_when_context_present` 保留原挂载。
  - `apply_patch_invalidates_read_state_for_deleted_file` 保留原挂载。
- `crates/vw-agent/src/tools/batch/mod.rs` → `crates/libs/tool-services/src/batch/tests.rs`：保留现有真实宿主绑定；继续可迁：拆出领域断言，须重建真实安全端口而非复制宿主。
  - `batch_executes_calls` 保留原挂载。
  - `batch_rejects_recursive_call` 保留原挂载。
  - `schema_and_registry_helpers_are_stable` 保留原挂载。
  - `batch_accepts_aliases_and_default_empty_args` 保留原挂载。
  - `batch_reports_invalid_empty_unknown_and_failed_calls` 保留原挂载。
  - `batch_discards_calls_over_the_limit` 保留原挂载。
  - `batch_documented_example_executes_with_registered_tool_ids` 保留原挂载。
  - `batch_schema_declares_nested_args` 保留原挂载。
- `crates/vw-agent/src/tools/client_tool_tests.rs` → `crates/libs/tool-services/src/client_tool_tests.rs`：保留现有真实宿主绑定；继续可迁：拆出领域断言，须重建真实安全端口而非复制宿主。
  - `dynamic_tools_callback_response_owner_is_validated` 保留原挂载。
  - `dynamic_tools_callback_registered_tool_preserves_media_across_catalog_update` 保留原挂载。
  - `dynamic_tools_callback_registration_rejects_source_schema_and_direct_execution` 保留原挂载。
  - `dynamic_tools_callback_permission_denial_never_emits_request` 保留原挂载。
  - `dynamic_tools_callback_execution_rejects_wrong_tenant_workspace_and_missing_turn` 保留原挂载。

## 接线、依赖及静态审查

- 新增 37 个领域测试，lib.rs 自注册 host_migration_33_tests，原 catalog/external_directory 挂载删除，context/decision 逐整函数拆分；没有 include/path 回读宿主。
- 接收库已有 tool-context/tool-catalog/agent-security；新增 dev-only tool-files、agent-paths。真实调用仍由 AgentContextHost → tool_context/tool_files、ToolAgentHost → tool_agents；领域库不依赖 vw-agent。
- 领域夹具只适配 ContextHost，安全判断使用真实 SecurityPolicy、catalog 使用真实 ToolCatalog，目录仍检查真实临时路径；不提供技能授权，未涉及端口 panic；ensure_ready 无外部插件、needs_approval 无外部审批管理器。
- Scope 中生产符号均按上表保留真实端口或兼容别名；既有生产下沉未计收益。测试文件新增行数包括保持原断言所需的局部 MockTool。

## 未来验证

`python3 scripts/cargo_slot.py cargo test -p tool-assembly --lib host_migration_33_tests -- --test-threads=1`，预期非零 37 条；编译/行为尚未验证。
宿主保留用例按上表原完整模块名逐条过滤：`python3 scripts/cargo_slot.py cargo test -p vw-agent --lib tools::<模块>::<函数> -- --exact --test-threads=1`（按实际 lib list 确认前缀）。每条预期1；迁出用例未来用宿主 list 证明消失，不能以零执行测试当通过。

## 深度恢复补充（替代前文保留结论及未来数量）

全39路径重新核对：宿主 2474 → 721，净减 1753（70.9%）。已迁领域测试 67 条。未运行任何 Cargo、构建或测试；仅完成静态差异/名称/端口检查。

生产调用新增实质下沉：build_agentic_tools 的可信能力解析与工具组合整函数归 session-engine::delegated_tools，AgentHost 现有调用经薄导出进入领域，原包装与9条宿主测试挂载删除。session-engine 已有 tool-services 依赖，补 async-trait dev 依赖。

Excel 真实 TLS、WebSocket、transport、策略过期、取消和文档审批断言整体归 tool-office，保留原真实传输夹具，宿主仅留共享权限工厂注册一条。工具上下文的真实能力注入身份断言也已归 tool-assembly；真实 ApprovalManager、HookRunner、NoopObserver 并非模拟替代。原生 defaults/权限别名/技能输入及拒绝优先于 Hook 共4条断言使用真实 agent-defaults、permission-runtime、agent-hooks。Cron 缺失任务/只读/速率限制3条移至 tool-cron，注入会 panic 的执行端口证明拒绝不会下发；宿主仅留实际执行、审批后执行、运行中和取消历史整合。锁文件按实际依赖边补齐，未触发解析器。

更新后的逐测试映射：
- `crates/vw-agent/src/tools/catalog_tests.rs::dynamic_tools_catalog_replacement_pins_inflight_executor` → `crates/libs/tool-assembly/src/host_migration_33_tests/catalog.rs::dynamic_tools_catalog_replacement_pins_inflight_executor`
- `crates/vw-agent/src/tools/catalog_tests.rs::dynamic_tools_catalog_rejects_duplicates_without_publication` → `crates/libs/tool-assembly/src/host_migration_33_tests/catalog.rs::dynamic_tools_catalog_rejects_duplicates_without_publication`
- `crates/vw-agent/src/tools/catalog_tests.rs::dynamic_tools_catalog_revocation_does_not_restore_loaded_ids` → `crates/libs/tool-assembly/src/host_migration_33_tests/catalog.rs::dynamic_tools_catalog_revocation_does_not_restore_loaded_ids`
- `crates/vw-agent/src/tools/catalog_tests.rs::dynamic_tools_catalog_rejects_reserved_client_names` → `crates/libs/tool-assembly/src/host_migration_33_tests/catalog.rs::dynamic_tools_catalog_rejects_reserved_client_names`
- `crates/vw-agent/src/tools/catalog_tests.rs::dynamic_tools_catalog_same_number_from_another_catalog_is_rejected` → `crates/libs/tool-assembly/src/host_migration_33_tests/catalog.rs::dynamic_tools_catalog_same_number_from_another_catalog_is_rejected`
- `crates/vw-agent/src/tools/catalog_tests.rs::dynamic_tools_catalog_session_and_context_share_snapshot_without_granting_metadata` → `crates/libs/tool-assembly/src/host_migration_33_tests/catalog.rs::dynamic_tools_catalog_session_and_context_share_snapshot_without_granting_metadata`
- `crates/vw-agent/src/tools/context/tests.rs::default_workflow_state_is_unbound` → `crates/libs/tool-assembly/src/host_migration_33_tests/context.rs::default_workflow_state_is_unbound`
- `crates/vw-agent/src/tools/context/tests.rs::context_preserves_session_and_root` → `crates/libs/tool-assembly/src/host_migration_33_tests/context.rs::context_preserves_session_and_root`
- `crates/vw-agent/src/tools/context/tests.rs::workflow_transitions_reset_worktree` → `crates/libs/tool-assembly/src/host_migration_33_tests/context.rs::workflow_transitions_reset_worktree`
- `crates/vw-agent/src/tools/context/tests.rs::task_local_context_is_visible_only_inside_scope` → `crates/libs/tool-assembly/src/host_migration_33_tests/context.rs::task_local_context_is_visible_only_inside_scope`
- `crates/vw-agent/src/tools/context/tests.rs::normalize_path_uses_context_workspace_root` → `crates/libs/tool-assembly/src/host_migration_33_tests/context.rs::normalize_path_uses_context_workspace_root`
- `crates/vw-agent/src/tools/context/tests.rs::default_and_debug_report_runtime_capability_presence` → `crates/libs/tool-assembly/src/host_migration_33_tests/context.rs::default_and_debug_report_runtime_capability_presence`
- `crates/vw-agent/src/tools/context/tests.rs::builder_methods_expose_injected_runtime_capabilities` → `crates/libs/tool-assembly/src/host_migration_33_tests/context.rs::builder_methods_expose_injected_runtime_capabilities`
- `crates/vw-agent/src/tools/context/tests.rs::trusted_user_skill_selections_survive_tool_call_context_cloning` → `crates/libs/tool-assembly/src/host_migration_33_tests/context.rs::trusted_user_skill_selections_survive_tool_call_context_cloning`
- `crates/vw-agent/src/tools/context/tests.rs::shared_read_state_and_workflow_handles_are_reused` → `crates/libs/tool-assembly/src/host_migration_33_tests/context.rs::shared_read_state_and_workflow_handles_are_reused`
- `crates/vw-agent/src/tools/context/tests.rs::scoped_read_state_helpers_note_get_and_invalidate_paths` → `crates/libs/tool-assembly/src/host_migration_33_tests/context.rs::scoped_read_state_helpers_note_get_and_invalidate_paths`
- `crates/vw-agent/src/tools/cron_run/tests.rs::force_runs_job_and_records_history` → `crates/vw-agent/src/tools/cron_run/tests.rs::force_runs_job_and_records_history`
- `crates/vw-agent/src/tools/cron_run/tests.rs::errors_for_missing_job` → `crates/libs/tool-cron/src/host_migration_33_tests.rs::errors_for_missing_job`
- `crates/vw-agent/src/tools/cron_run/tests.rs::blocks_run_in_read_only_mode` → `crates/libs/tool-cron/src/host_migration_33_tests.rs::blocks_run_in_read_only_mode`
- `crates/vw-agent/src/tools/cron_run/tests.rs::shell_run_requires_approval_for_medium_risk` → `crates/vw-agent/src/tools/cron_run/tests.rs::shell_run_requires_approval_for_medium_risk`
- `crates/vw-agent/src/tools/cron_run/tests.rs::blocks_run_when_rate_limited` → `crates/libs/tool-cron/src/host_migration_33_tests.rs::blocks_run_when_rate_limited`
- `crates/vw-agent/src/tools/cron_run/tests.rs::cron_history_manual_run_is_visible_until_completion` → `crates/vw-agent/src/tools/cron_run/tests.rs::cron_history_manual_run_is_visible_until_completion`
- `crates/vw-agent/src/tools/cron_run/tests.rs::cron_history_cancelled_manual_run_leaves_no_history` → `crates/vw-agent/src/tools/cron_run/tests.rs::cron_history_cancelled_manual_run_leaves_no_history`
- `crates/vw-agent/src/tools/decision/tests.rs::allow_and_ask_expose_updated_input_and_reason` → `crates/libs/tool-assembly/src/host_migration_33_tests/decision.rs::allow_and_ask_expose_updated_input_and_reason`
- `crates/vw-agent/src/tools/decision/tests.rs::finalize_allows_without_security_and_classifies_tool_permission_errors` → `crates/libs/tool-assembly/src/host_migration_33_tests/decision.rs::finalize_allows_without_security_and_classifies_tool_permission_errors`
- `crates/vw-agent/src/tools/decision/tests.rs::finalize_permission_decision_maps_allow_ask_and_denials` → `crates/libs/tool-assembly/src/host_migration_33_tests/decision.rs::finalize_permission_decision_maps_allow_ask_and_denials`
- `crates/vw-agent/src/tools/decision/tests.rs::deny_only_exposes_input_after_explicit_snapshot` → `crates/libs/tool-assembly/src/host_migration_33_tests/decision.rs::deny_only_exposes_input_after_explicit_snapshot`
- `crates/vw-agent/src/tools/decision/tests.rs::updated_input_snapshot_only_changes_denials` → `crates/libs/tool-assembly/src/host_migration_33_tests/decision.rs::updated_input_snapshot_only_changes_denials`
- `crates/vw-agent/src/tools/decision/tests.rs::security_decision_handles_readonly_rate_limit_and_blocked_paths` → `crates/libs/tool-assembly/src/host_migration_33_tests/decision.rs::security_decision_handles_readonly_rate_limit_and_blocked_paths`
- `crates/vw-agent/src/tools/decision/tests.rs::blocked_path_checks_string_and_array_fields` → `crates/libs/tool-assembly/src/host_migration_33_tests/decision.rs::blocked_path_checks_string_and_array_fields`
- `crates/vw-agent/src/tools/decision/tests.rs::shell_security_decision_marks_approved_inputs_and_preserves_denials` → `crates/libs/tool-assembly/src/host_migration_33_tests/decision.rs::shell_security_decision_marks_approved_inputs_and_preserves_denials`
- `crates/vw-agent/src/tools/decision/tests.rs::request_interactive_approval_denies_when_manager_is_missing_or_user_declines` → `crates/vw-agent/src/tools/decision/tests.rs::request_interactive_approval_denies_when_manager_is_missing_or_user_declines`
- `crates/vw-agent/src/tools/decision/tests.rs::non_cli_bypass_records_approval_and_allows_pending_decision` → `crates/vw-agent/src/tools/decision/tests.rs::non_cli_bypass_records_approval_and_allows_pending_decision`
- `crates/vw-agent/src/tools/decision/tests.rs::non_cli_approval_context_sends_prompt_and_waits_for_resolution` → `crates/vw-agent/src/tools/decision/tests.rs::non_cli_approval_context_sends_prompt_and_waits_for_resolution`
- `crates/vw-agent/src/tools/decision/tests.rs::command_input_only_accepts_shell_tools_and_string_commands` → `crates/libs/tool-assembly/src/host_migration_33_tests/decision.rs::command_input_only_accepts_shell_tools_and_string_commands`
- `crates/vw-agent/src/tools/decision/tests.rs::approval_marker_only_changes_shell_object_inputs` → `crates/libs/tool-assembly/src/host_migration_33_tests/decision.rs::approval_marker_only_changes_shell_object_inputs`
- `crates/vw-agent/src/tools/decision/tests.rs::permission_reason_helpers_preserve_structured_context` → `crates/libs/tool-assembly/src/host_migration_33_tests/decision.rs::permission_reason_helpers_preserve_structured_context`
- `crates/vw-agent/src/tools/decision/tests.rs::classify_message_separates_denials_from_execution_failures` → `crates/libs/tool-assembly/src/host_migration_33_tests/decision.rs::classify_message_separates_denials_from_execution_failures`
- `crates/vw-agent/src/tools/decision/tests.rs::codex_plugin_compat_t37_host_denial_precedes_hook_allow` → `crates/libs/tool-assembly/src/host_migration_33_tests/decision.rs::codex_plugin_compat_t37_host_denial_precedes_hook_allow`
- `crates/vw-agent/src/tools/decision/tests.rs::agent_config_cleanup_full_access_retains_native_permission_denials` → `crates/libs/tool-assembly/src/host_migration_33_tests/decision.rs::agent_config_cleanup_full_access_retains_native_permission_denials`
- `crates/vw-agent/src/tools/decision/tests.rs::agent_native_defaults_permission_honors_registered_tool_aliases` → `crates/libs/tool-assembly/src/host_migration_33_tests/decision.rs::agent_native_defaults_permission_honors_registered_tool_aliases`
- `crates/vw-agent/src/tools/decision/tests.rs::agent_native_defaults_skill_permission_uses_registered_input` → `crates/libs/tool-assembly/src/host_migration_33_tests/decision.rs::agent_native_defaults_skill_permission_uses_registered_input`
- `crates/vw-agent/src/tools/decision/tests.rs::dynamic_tools_contract_client_readonly_hint_cannot_grant_security_capability` → `crates/libs/tool-assembly/src/host_migration_33_tests/decision.rs::dynamic_tools_contract_client_readonly_hint_cannot_grant_security_capability`
- `crates/vw-agent/src/tools/delegated_tools_tests.rs::build_agentic_tools_blocks_batch_even_when_selected` → `crates/libs/session-engine/src/host_migration_33_tests.rs::build_agentic_tools_blocks_batch_even_when_selected`
- `crates/vw-agent/src/tools/delegated_tools_tests.rs::build_agentic_tools_empty_selection_keeps_builtin_and_blocks_recursive_tool` → `crates/libs/session-engine/src/host_migration_33_tests.rs::build_agentic_tools_empty_selection_keeps_builtin_and_blocks_recursive_tool`
- `crates/vw-agent/src/tools/delegated_tools_tests.rs::build_agentic_tools_matches_tool_aliases` → `crates/libs/session-engine/src/host_migration_33_tests.rs::build_agentic_tools_matches_tool_aliases`
- `crates/vw-agent/src/tools/delegated_tools_tests.rs::delegated_skill_tool_enforces_resolved_skill_capabilities` → `crates/libs/session-engine/src/host_migration_33_tests.rs::delegated_skill_tool_enforces_resolved_skill_capabilities`
- `crates/vw-agent/src/tools/delegated_tools_tests.rs::strict_skill_policy_without_defaults_does_not_expose_skill` → `crates/libs/session-engine/src/host_migration_33_tests.rs::strict_skill_policy_without_defaults_does_not_expose_skill`
- `crates/vw-agent/src/tools/delegated_tools_tests.rs::delegated_tool_ref_forwards_contract_and_execution` → `crates/libs/session-engine/src/host_migration_33_tests.rs::delegated_tool_ref_forwards_contract_and_execution`
- `crates/vw-agent/src/tools/delegated_tools_tests.rs::filtered_skill_tool_exposes_sorted_trimmed_skill_contract` → `crates/libs/session-engine/src/host_migration_33_tests.rs::filtered_skill_tool_exposes_sorted_trimmed_skill_contract`
- `crates/vw-agent/src/tools/delegated_tools_tests.rs::filtered_skill_tool_delegates_missing_and_blank_names` → `crates/libs/session-engine/src/host_migration_33_tests.rs::filtered_skill_tool_delegates_missing_and_blank_names`
- `crates/vw-agent/src/tools/delegated_tools_tests.rs::delegated_tools_keep_command_lifecycle_without_restoring_optional_capabilities` → `crates/libs/session-engine/src/host_migration_33_tests.rs::delegated_tools_keep_command_lifecycle_without_restoring_optional_capabilities`
- `crates/vw-agent/src/tools/excel_control_tests.rs::cc_control_14_no_session_discovers_empty_and_refuses_execution` → `crates/libs/tool-office/src/host_migration_33_tests.rs::cc_control_14_no_session_discovers_empty_and_refuses_execution`
- `crates/vw-agent/src/tools/excel_control_tests.rs::cc_control_14_duplicate_titles_require_exact_document_and_generation` → `crates/libs/tool-office/src/host_migration_33_tests.rs::cc_control_14_duplicate_titles_require_exact_document_and_generation`
- `crates/vw-agent/src/tools/excel_control_tests.rs::cc_control_14_switch_and_document_grant_both_gate_execution` → `crates/libs/tool-office/src/host_migration_33_tests.rs::cc_control_14_switch_and_document_grant_both_gate_execution`
- `crates/vw-agent/src/tools/excel_control_tests.rs::cc_control_14_missing_capability_and_unknown_fields_are_rejected` → `crates/libs/tool-office/src/host_migration_33_tests.rs::cc_control_14_missing_capability_and_unknown_fields_are_rejected`
- `crates/vw-agent/src/tools/excel_control_tests.rs::cc_control_14_write_returns_actual_readback_to_model` → `crates/libs/tool-office/src/host_migration_33_tests.rs::cc_control_14_write_returns_actual_readback_to_model`
- `crates/vw-agent/src/tools/excel_control_tests.rs::cc_control_14_disconnect_and_repair_invalidate_old_generation` → `crates/libs/tool-office/src/host_migration_33_tests.rs::cc_control_14_disconnect_and_repair_invalidate_old_generation`
- `crates/vw-agent/src/tools/excel_control_tests.rs::cc_control_14_foreign_executor_owner_cannot_discover_or_execute` → `crates/libs/tool-office/src/host_migration_33_tests.rs::cc_control_14_foreign_executor_owner_cannot_discover_or_execute`
- `crates/vw-agent/src/tools/excel_control_tests.rs::cc_control_14_revocation_during_command_discards_result` → `crates/libs/tool-office/src/host_migration_33_tests.rs::cc_control_14_revocation_during_command_discards_result`
- `crates/vw-agent/src/tools/excel_control_tests.rs::cc_control_14_invalid_rectangles_shapes_and_payloads_never_dispatch` → `crates/libs/tool-office/src/host_migration_33_tests.rs::cc_control_14_invalid_rectangles_shapes_and_payloads_never_dispatch`
- `crates/vw-agent/src/tools/excel_control_tests.rs::cc_control_14_abort_invalidates_connection_and_discards_late_result` → `crates/libs/tool-office/src/host_migration_33_tests.rs::cc_control_14_abort_invalidates_connection_and_discards_late_result`
- `crates/vw-agent/src/tools/excel_control_tests.rs::cc_control_14_factory_registers_excel_with_shared_gateway_authority` → `crates/vw-agent/src/tools/excel_control_tests.rs::cc_control_14_factory_registers_excel_with_shared_gateway_authority`
- `crates/vw-agent/src/tools/excel_control_tests.rs::cc_control_14_malformed_readback_is_failure_and_closes_connection` → `crates/libs/tool-office/src/host_migration_33_tests.rs::cc_control_14_malformed_readback_is_failure_and_closes_connection`
- `crates/vw-agent/src/tools/excel_control_tests.rs::cc_control_14_document_approval_is_scoped_and_explicit_deny_is_preserved` → `crates/libs/tool-office/src/host_migration_33_tests.rs::cc_control_14_document_approval_is_scoped_and_explicit_deny_is_preserved`
- `crates/vw-agent/src/tools/external_directory/tests.rs::default_options_enforce_file_checks` → `crates/libs/tool-assembly/src/host_migration_33_tests/external_directory.rs::default_options_enforce_file_checks`
- `crates/vw-agent/src/tools/external_directory/tests.rs::relative_paths_are_trimmed_joined_and_normalized` → `crates/libs/tool-assembly/src/host_migration_33_tests/external_directory.rs::relative_paths_are_trimmed_joined_and_normalized`
- `crates/vw-agent/src/tools/external_directory/tests.rs::empty_target_is_ok` → `crates/libs/tool-assembly/src/host_migration_33_tests/external_directory.rs::empty_target_is_ok`
- `crates/vw-agent/src/tools/external_directory/tests.rs::bypass_skips_checks` → `crates/libs/tool-assembly/src/host_migration_33_tests/external_directory.rs::bypass_skips_checks`
- `crates/vw-agent/src/tools/external_directory/tests.rs::workspace_relative_path_is_allowed` → `crates/libs/tool-assembly/src/host_migration_33_tests/external_directory.rs::workspace_relative_path_is_allowed`
- `crates/vw-agent/src/tools/external_directory/tests.rs::outside_workspace_is_denied_without_allowlist` → `crates/libs/tool-assembly/src/host_migration_33_tests/external_directory.rs::outside_workspace_is_denied_without_allowlist`
- `crates/vw-agent/src/tools/external_directory/tests.rs::outside_file_denial_reports_parent_directory` → `crates/libs/tool-assembly/src/host_migration_33_tests/external_directory.rs::outside_file_denial_reports_parent_directory`
- `crates/vw-agent/src/tools/external_directory/tests.rs::outside_file_denial_reports_original_path_when_parent_is_missing` → `crates/libs/tool-assembly/src/host_migration_33_tests/external_directory.rs::outside_file_denial_reports_original_path_when_parent_is_missing`
- `crates/vw-agent/src/tools/external_directory/tests.rs::allowed_roots_path_is_allowed` → `crates/libs/tool-assembly/src/host_migration_33_tests/external_directory.rs::allowed_roots_path_is_allowed`
- `crates/vw-agent/src/tools/external_directory/tests.rs::tool_output_directory_is_allowed_outside_workspace` → `crates/libs/tool-assembly/src/host_migration_33_tests/external_directory.rs::tool_output_directory_is_allowed_outside_workspace`

更新后的全部保留文件及行数：
- `crates/vw-agent/src/tools/agent_hosts.rs`：242 行；真实 ChildCoordinator/TaskCoordinator/Agent/Session/Thread 端口，协调器和外部生命周期状态不复制。
- `crates/vw-agent/src/tools/agent_tool/mod.rs`：32 行；保留具体宿主端口/公开兼容别名；测试仅保留真实审批等待、shell策略、Cron执行与共享Office工厂边界（旧保留理由以上方补充为准）。
- `crates/vw-agent/src/tools/apply_patch/mod.rs`：6 行；保留具体宿主端口/公开兼容别名；测试仅保留真实审批等待、shell策略、Cron执行与共享Office工厂边界（旧保留理由以上方补充为准）。
- `crates/vw-agent/src/tools/batch/mod.rs`：8 行；保留具体宿主端口/公开兼容别名；测试仅保留真实审批等待、shell策略、Cron执行与共享Office工厂边界（旧保留理由以上方补充为准）。
- `crates/vw-agent/src/tools/browser/mod.rs`：3 行；保留具体宿主端口/公开兼容别名；测试仅保留真实审批等待、shell策略、Cron执行与共享Office工厂边界（旧保留理由以上方补充为准）。
- `crates/vw-agent/src/tools/browser_open/mod.rs`：3 行；保留具体宿主端口/公开兼容别名；测试仅保留真实审批等待、shell策略、Cron执行与共享Office工厂边界（旧保留理由以上方补充为准）。
- `crates/vw-agent/src/tools/catalog.rs`：2 行；保留具体宿主端口/公开兼容别名；测试仅保留真实审批等待、shell策略、Cron执行与共享Office工厂边界（旧保留理由以上方补充为准）。
- `crates/vw-agent/src/tools/client_tool.rs`：2 行；保留具体宿主端口/公开兼容别名；测试仅保留真实审批等待、shell策略、Cron执行与共享Office工厂边界（旧保留理由以上方补充为准）。
- `crates/vw-agent/src/tools/client_tool_tests.rs`：1 行；保留具体宿主端口/公开兼容别名；测试仅保留真实审批等待、shell策略、Cron执行与共享Office工厂边界（旧保留理由以上方补充为准）。
- `crates/vw-agent/src/tools/codesearch/mod.rs`：3 行；保留具体宿主端口/公开兼容别名；测试仅保留真实审批等待、shell策略、Cron执行与共享Office工厂边界（旧保留理由以上方补充为准）。
- `crates/vw-agent/src/tools/computer_control.rs`：7 行；保留具体宿主端口/公开兼容别名；测试仅保留真实审批等待、shell策略、Cron执行与共享Office工厂边界（旧保留理由以上方补充为准）。
- `crates/vw-agent/src/tools/config/mod.rs`：2 行；保留具体宿主端口/公开兼容别名；测试仅保留真实审批等待、shell策略、Cron执行与共享Office工厂边界（旧保留理由以上方补充为准）。
- `crates/vw-agent/src/tools/content_search/mod.rs`：2 行；保留具体宿主端口/公开兼容别名；测试仅保留真实审批等待、shell策略、Cron执行与共享Office工厂边界（旧保留理由以上方补充为准）。
- `crates/vw-agent/src/tools/context/mod.rs`：78 行；保留具体宿主端口/公开兼容别名；测试仅保留真实审批等待、shell策略、Cron执行与共享Office工厂边界（旧保留理由以上方补充为准）。
- `crates/vw-agent/src/tools/context/run_worktree.rs`：3 行；保留具体宿主端口/公开兼容别名；测试仅保留真实审批等待、shell策略、Cron执行与共享Office工厂边界（旧保留理由以上方补充为准）。
- `crates/vw-agent/src/tools/context/shell_policy.rs`：2 行；保留具体宿主端口/公开兼容别名；测试仅保留真实审批等待、shell策略、Cron执行与共享Office工厂边界（旧保留理由以上方补充为准）。
- `crates/vw-agent/src/tools/cron_add/mod.rs`：1 行；保留具体宿主端口/公开兼容别名；测试仅保留真实审批等待、shell策略、Cron执行与共享Office工厂边界（旧保留理由以上方补充为准）。
- `crates/vw-agent/src/tools/cron_list/mod.rs`：1 行；保留具体宿主端口/公开兼容别名；测试仅保留真实审批等待、shell策略、Cron执行与共享Office工厂边界（旧保留理由以上方补充为准）。
- `crates/vw-agent/src/tools/cron_remove/mod.rs`：1 行；保留具体宿主端口/公开兼容别名；测试仅保留真实审批等待、shell策略、Cron执行与共享Office工厂边界（旧保留理由以上方补充为准）。
- `crates/vw-agent/src/tools/cron_run/mod.rs`：12 行；保留具体宿主端口/公开兼容别名；测试仅保留真实审批等待、shell策略、Cron执行与共享Office工厂边界（旧保留理由以上方补充为准）。
- `crates/vw-agent/src/tools/cron_run/tests.rs`：129 行；保留具体宿主端口/公开兼容别名；测试仅保留真实审批等待、shell策略、Cron执行与共享Office工厂边界（旧保留理由以上方补充为准）。
- `crates/vw-agent/src/tools/cron_runs/mod.rs`：1 行；保留具体宿主端口/公开兼容别名；测试仅保留真实审批等待、shell策略、Cron执行与共享Office工厂边界（旧保留理由以上方补充为准）。
- `crates/vw-agent/src/tools/cron_update/mod.rs`：1 行；保留具体宿主端口/公开兼容别名；测试仅保留真实审批等待、shell策略、Cron执行与共享Office工厂边界（旧保留理由以上方补充为准）。
- `crates/vw-agent/src/tools/decision/mod.rs`：14 行；保留具体宿主端口/公开兼容别名；测试仅保留真实审批等待、shell策略、Cron执行与共享Office工厂边界（旧保留理由以上方补充为准）。
- `crates/vw-agent/src/tools/decision/tests.rs`：81 行；保留具体宿主端口/公开兼容别名；测试仅保留真实审批等待、shell策略、Cron执行与共享Office工厂边界（旧保留理由以上方补充为准）。
- `crates/vw-agent/src/tools/delegate/mod.rs`：19 行；保留具体宿主端口/公开兼容别名；测试仅保留真实审批等待、shell策略、Cron执行与共享Office工厂边界（旧保留理由以上方补充为准）。
- `crates/vw-agent/src/tools/delegated_tools.rs`：2 行；保留具体宿主端口/公开兼容别名；测试仅保留真实审批等待、shell策略、Cron执行与共享Office工厂边界（旧保留理由以上方补充为准）。
- `crates/vw-agent/src/tools/discovery.rs`：1 行；保留具体宿主端口/公开兼容别名；测试仅保留真实审批等待、shell策略、Cron执行与共享Office工厂边界（旧保留理由以上方补充为准）。
- `crates/vw-agent/src/tools/enter_worktree/mod.rs`：6 行；保留具体宿主端口/公开兼容别名；测试仅保留真实审批等待、shell策略、Cron执行与共享Office工厂边界（旧保留理由以上方补充为准）。
- `crates/vw-agent/src/tools/excel_control.rs`：4 行；保留具体宿主端口/公开兼容别名；测试仅保留真实审批等待、shell策略、Cron执行与共享Office工厂边界（旧保留理由以上方补充为准）。
- `crates/vw-agent/src/tools/excel_control_tests.rs`：35 行；保留具体宿主端口/公开兼容别名；测试仅保留真实审批等待、shell策略、Cron执行与共享Office工厂边界（旧保留理由以上方补充为准）。
- `crates/vw-agent/src/tools/executor/mod.rs`：2 行；保留具体宿主端口/公开兼容别名；测试仅保留真实审批等待、shell策略、Cron执行与共享Office工厂边界（旧保留理由以上方补充为准）。
- `crates/vw-agent/src/tools/exit_worktree/mod.rs`：6 行；保留具体宿主端口/公开兼容别名；测试仅保留真实审批等待、shell策略、Cron执行与共享Office工厂边界（旧保留理由以上方补充为准）。
- `crates/vw-agent/src/tools/external_directory/mod.rs`：6 行；保留具体宿主端口/公开兼容别名；测试仅保留真实审批等待、shell策略、Cron执行与共享Office工厂边界（旧保留理由以上方补充为准）。
- `crates/vw-agent/src/tools/file_edit/mod.rs`：3 行；保留具体宿主端口/公开兼容别名；测试仅保留真实审批等待、shell策略、Cron执行与共享Office工厂边界（旧保留理由以上方补充为准）。

未来 `python3 scripts/cargo_slot.py cargo test -p tool-assembly --lib host_migration_33_tests -- --test-threads=1`：预期 43 条非零用例；均未执行。

未来 `python3 scripts/cargo_slot.py cargo test -p tool-office --lib host_migration_33_tests -- --test-threads=1`：预期 12 条非零用例；均未执行。

未来 `python3 scripts/cargo_slot.py cargo test -p session-engine --lib host_migration_33_tests -- --test-threads=1`：预期 9 条非零用例；均未执行。

未来 `python3 scripts/cargo_slot.py cargo test -p tool-cron --lib host_migration_33_tests -- --test-threads=1`：预期 3 条非零用例；均未执行。

真实宿主 list 验证迁出函数消失亦未执行；本任务不宣称整轮35000行门槛达标。

Shell 生产适配亦移至 tool-assembly::shell_permission，真实调用由 AgentContextHost 经薄导出接回；原 shell 决策断言同组迁出。最终净减70.9%，宿主721行（含具体协调器242行及兼容入口）。
